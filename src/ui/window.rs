use crate::config::Config;
use crate::installer;
use crate::terminal::{PtySession, Terminal};
use crate::ui::renderer::Renderer;
use crate::ui::theme;
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use anyhow::Result;
use arboard::Clipboard;
use log::info;
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{CursorIcon, ResizeDirection, Window, WindowAttributes, WindowId};

const INITIAL_WIDTH: u32 = 1000;
const INITIAL_HEIGHT: u32 = 650;
const TITLE: &str = "WindowedClaude";
const TITLE_BAR_HEIGHT: f64 = 36.0;

/// Hit zones for title bar buttons (relative to right edge)
const BTN_SIZE: f64 = 12.0;
const BTN_SPACING: f64 = 28.0;
const BTN_RIGHT_PAD: f64 = 20.0;

/// App state machine
#[derive(Debug, Clone, Copy, PartialEq)]
enum AppPhase {
    /// Normal terminal operation
    Terminal,
    /// Showing the welcome/shortcut prompt (first run only)
    WelcomeScreen,
}

struct App {
    config: Config,
    phase: AppPhase,
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    renderer: Option<Renderer>,
    terminal: Option<Terminal>,
    pty: Option<PtySession>,
    modifiers: ModifiersState,
    width: u32,
    height: u32,
    // Mouse state
    cursor_x: f64,
    cursor_y: f64,
    mouse_pressed: bool,
    selecting: bool,
    maximized: bool,
}

impl App {
    fn new(config: Config, show_welcome: bool) -> Self {
        let phase = if show_welcome {
            AppPhase::WelcomeScreen
        } else {
            AppPhase::Terminal
        };
        Self {
            config,
            phase,
            window: None,
            surface: None,
            renderer: None,
            terminal: None,
            pty: None,
            modifiers: ModifiersState::empty(),
            width: INITIAL_WIDTH,
            height: INITIAL_HEIGHT,
            cursor_x: 0.0,
            cursor_y: 0.0,
            mouse_pressed: false,
            selecting: false,
            maximized: false,
        }
    }

    fn spawn_pty(&mut self) {
        let git_bash = self.config.git_bash_path.clone()
            .unwrap_or_else(installer::git_bash_path);
        let claude_cli = installer::claude_cli_path();

        match PtySession::spawn(git_bash, claude_cli) {
            Ok(session) => {
                info!("PTY session spawned");
                self.pty = Some(session);
            }
            Err(e) => {
                log::error!("Failed to spawn PTY: {}", e);
            }
        }
    }

    fn update_title(&self) {
        if let Some(window) = &self.window {
            let t = self.current_theme();
            let opacity_str = if self.config.transparent {
                format!(" | {:.0}%", self.config.opacity * 100.0)
            } else {
                String::new()
            };
            window.set_title(&format!("{} — {}{}", TITLE, t.name, opacity_str));
        }
    }

    fn current_theme(&self) -> &'static theme::Theme {
        theme::theme_by_id(&self.config.theme_id)
    }

    /// Transition from welcome screen to terminal mode
    fn finish_welcome(&mut self, create_shortcut: bool) {
        if create_shortcut {
            info!("User opted to create desktop shortcut");
            if let Err(e) = installer::shortcuts::create_desktop_shortcut() {
                log::warn!("Desktop shortcut failed: {}", e);
            }
        } else {
            info!("User declined desktop shortcut");
        }
        installer::mark_shortcut_prompted();
        self.phase = AppPhase::Terminal;
        self.spawn_pty();
        self.request_redraw();
    }

    /// Rebuild the renderer (e.g., after font size change) and resize terminal to match
    fn rebuild_renderer(&mut self) {
        let theme = self.current_theme().clone();
        let mut renderer = Renderer::new(theme, self.config.font_size);
        renderer.resize(self.width, self.height);
        if let Some(terminal) = &mut self.terminal {
            terminal.resize(renderer.cols, renderer.rows);
        }
        self.renderer = Some(renderer);
        self.request_redraw();
        info!("Font size: {}", self.config.font_size);
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Convert pixel coordinates to terminal grid point
    fn pixel_to_point(&self, x: f64, y: f64) -> Option<(Point, Side)> {
        let renderer = self.renderer.as_ref()?;
        let term_y = y - renderer.title_bar_height as f64;
        if term_y < 0.0 {
            return None;
        }
        let col = (x / renderer.cell_width as f64) as usize;
        let row = (term_y / renderer.cell_height as f64) as usize;
        let col = col.min(renderer.cols.saturating_sub(1));
        let row = row.min(renderer.rows.saturating_sub(1));

        // Determine which side of the cell the click is on
        let cell_mid = (col as f64 + 0.5) * renderer.cell_width as f64;
        let side = if x < cell_mid { Side::Left } else { Side::Right };

        Some((Point::new(Line(row as i32), Column(col)), side))
    }

    /// Check if a click position is within the title bar area
    fn in_title_bar(&self, y: f64) -> bool {
        y < TITLE_BAR_HEIGHT
    }

    /// Check if cursor is on a resize edge (5px border)
    fn resize_direction(&self, x: f64, y: f64) -> Option<ResizeDirection> {
        const EDGE: f64 = 5.0;
        let w = self.width as f64;
        let h = self.height as f64;

        let left = x < EDGE;
        let right = x > w - EDGE;
        let top = y < EDGE;
        let bottom = y > h - EDGE;

        match (left, right, top, bottom) {
            (true, _, true, _) => Some(ResizeDirection::NorthWest),
            (_, true, true, _) => Some(ResizeDirection::NorthEast),
            (true, _, _, true) => Some(ResizeDirection::SouthWest),
            (_, true, _, true) => Some(ResizeDirection::SouthEast),
            (true, _, _, _) => Some(ResizeDirection::West),
            (_, true, _, _) => Some(ResizeDirection::East),
            (_, _, true, _) => Some(ResizeDirection::North),
            (_, _, _, true) => Some(ResizeDirection::South),
            _ => None,
        }
    }

    /// Check which title bar button was clicked (if any)
    fn title_bar_button(&self, x: f64, y: f64) -> Option<TitleBarButton> {
        if !self.in_title_bar(y) {
            return None;
        }
        let w = self.width as f64;
        let right = w - BTN_RIGHT_PAD;

        // Close button (rightmost)
        if x >= right - BTN_SIZE && x <= right + BTN_SIZE {
            return Some(TitleBarButton::Close);
        }
        // Maximize button
        let max_x = right - BTN_SPACING;
        if x >= max_x - BTN_SIZE && x <= max_x + BTN_SIZE {
            return Some(TitleBarButton::Maximize);
        }
        // Minimize button
        let min_x = right - BTN_SPACING * 2.0;
        if x >= min_x - BTN_SIZE && x <= min_x + BTN_SIZE {
            return Some(TitleBarButton::Minimize);
        }
        None
    }
}

#[derive(Debug)]
enum TitleBarButton {
    Close,
    Maximize,
    Minimize,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let size = winit::dpi::LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT);
        let t = self.current_theme();

        let attrs = WindowAttributes::default()
            .with_title(format!("{} — {}", TITLE, t.name))
            .with_inner_size(size)
            .with_min_inner_size(winit::dpi::LogicalSize::new(400u32, 300u32))
            .with_decorations(false)
            .with_transparent(self.config.transparent);

        let window = Rc::new(
            event_loop.create_window(attrs).expect("Failed to create window"),
        );

        let context = softbuffer::Context::new(window.clone())
            .expect("Failed to create softbuffer context");
        let mut surface = Surface::new(&context, window.clone())
            .expect("Failed to create softbuffer surface");

        let phys = window.inner_size();
        self.width = phys.width.max(1);
        self.height = phys.height.max(1);
        surface
            .resize(
                NonZeroU32::new(self.width).unwrap(),
                NonZeroU32::new(self.height).unwrap(),
            )
            .expect("Failed to resize surface");

        let renderer = Renderer::new(t.clone(), self.config.font_size);
        let terminal = Terminal::new(renderer.cols, renderer.rows);

        self.surface = Some(surface);
        self.renderer = Some(renderer);
        self.terminal = Some(terminal);
        self.window = Some(window);

        // Only spawn PTY immediately if skipping welcome screen
        if self.phase == AppPhase::Terminal {
            self.spawn_pty();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Window close requested");
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                self.width = size.width.max(1);
                self.height = size.height.max(1);

                if let Some(surface) = &mut self.surface {
                    let _ = surface.resize(
                        NonZeroU32::new(self.width).unwrap(),
                        NonZeroU32::new(self.height).unwrap(),
                    );
                }
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(self.width, self.height);
                    if let Some(terminal) = &mut self.terminal {
                        terminal.resize(renderer.cols, renderer.rows);
                    }
                }
                self.request_redraw();
            }

            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }

            // --- Mouse: track cursor position + resize cursor icon ---
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_x = position.x;
                self.cursor_y = position.y;

                // Update selection if dragging
                if self.selecting {
                    if let Some((point, side)) = self.pixel_to_point(position.x, position.y) {
                        if let Some(terminal) = &self.terminal {
                            if let Ok(mut term) = terminal.term.lock() {
                                if let Some(ref mut sel) = term.selection {
                                    sel.update(point, side);
                                }
                            }
                        }
                        self.request_redraw();
                    }
                }

                // Update cursor icon based on edge proximity
                if let Some(window) = &self.window {
                    if let Some(dir) = self.resize_direction(position.x, position.y) {
                        window.set_cursor(CursorIcon::from(dir));
                    } else if self.in_title_bar(position.y) {
                        window.set_cursor(CursorIcon::Default);
                    } else {
                        window.set_cursor(CursorIcon::Text);
                    }
                }
            }

            // --- Mouse: clicks for title bar buttons + drag ---
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                match state {
                    ElementState::Pressed => {
                        self.mouse_pressed = true;

                        // Check resize edges first (takes priority)
                        if let Some(dir) = self.resize_direction(self.cursor_x, self.cursor_y) {
                            if let Some(window) = &self.window {
                                let _ = window.drag_resize_window(dir);
                            }
                            return;
                        }

                        if self.in_title_bar(self.cursor_y) {
                            // Check title bar buttons first
                            match self.title_bar_button(self.cursor_x, self.cursor_y) {
                                Some(TitleBarButton::Close) => {
                                    event_loop.exit();
                                    return;
                                }
                                Some(TitleBarButton::Maximize) => {
                                    if let Some(window) = &self.window {
                                        self.maximized = !self.maximized;
                                        window.set_maximized(self.maximized);
                                    }
                                    return;
                                }
                                Some(TitleBarButton::Minimize) => {
                                    if let Some(window) = &self.window {
                                        window.set_minimized(true);
                                    }
                                    return;
                                }
                                None => {
                                    // Title bar drag — initiate window move
                                    if let Some(window) = &self.window {
                                        let _ = window.drag_window();
                                    }
                                    return;
                                }
                            }
                        } else {
                            // Terminal area — start text selection
                            if let Some((point, side)) = self.pixel_to_point(self.cursor_x, self.cursor_y) {
                            self.selecting = true;
                            if let Some(terminal) = &self.terminal {
                                if let Ok(mut term) = terminal.term.lock() {
                                    let sel = Selection::new(SelectionType::Simple, point, side);
                                    term.selection = Some(sel);
                                }
                            }
                            self.request_redraw();
                            }
                        }
                    }
                    ElementState::Released => {
                        self.mouse_pressed = false;
                        self.selecting = false;
                    }
                }
            }

            // --- Mouse: double-click title bar to maximize ---
            WindowEvent::DoubleTapGesture { .. } => {
                if self.in_title_bar(self.cursor_y) {
                    if let Some(window) = &self.window {
                        self.maximized = !self.maximized;
                        window.set_maximized(self.maximized);
                    }
                }
            }

            // --- Mouse: scroll wheel for terminal scrollback ---
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as i32,
                    MouseScrollDelta::PixelDelta(pos) => (pos.y / 20.0) as i32,
                };

                if let Some(terminal) = &self.terminal {
                    if let Ok(mut term) = terminal.term.lock() {
                        let scroll = alacritty_terminal::grid::Scroll::Delta(lines);
                        term.scroll_display(scroll);
                    }
                }
                self.request_redraw();
            }

            // --- Keyboard ---
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: ElementState::Pressed,
                        text,
                        ..
                    },
                ..
            } => {
                // --- Welcome screen input ---
                if self.phase == AppPhase::WelcomeScreen {
                    match &logical_key {
                        Key::Character(c) if c.as_str().eq_ignore_ascii_case("y") => {
                            self.finish_welcome(true);
                            return;
                        }
                        Key::Character(c) if c.as_str().eq_ignore_ascii_case("n") => {
                            self.finish_welcome(false);
                            return;
                        }
                        Key::Named(NamedKey::Enter) => {
                            // Enter defaults to Yes
                            self.finish_welcome(true);
                            return;
                        }
                        Key::Named(NamedKey::Escape) => {
                            // Escape defaults to No
                            self.finish_welcome(false);
                            return;
                        }
                        _ => return, // Ignore other keys during welcome
                    }
                }

                let ctrl = self.modifiers.control_key();
                let shift = self.modifiers.shift_key();

                // --- App hotkeys (Ctrl+Shift combos) ---
                if ctrl && shift {
                    match &logical_key {
                        // Ctrl+Shift+T — cycle theme
                        Key::Character(c) if c.as_str().eq_ignore_ascii_case("t") => {
                            self.config.cycle_theme();
                            let new_theme = self.current_theme().clone();
                            if let Some(renderer) = &mut self.renderer {
                                renderer.theme = new_theme;
                            }
                            self.update_title();
                            self.request_redraw();
                            info!("Theme: {}", self.config.theme_id);
                            return;
                        }
                        // Ctrl+Shift+O — toggle transparency
                        Key::Character(c) if c.as_str().eq_ignore_ascii_case("o") => {
                            self.config.toggle_transparency();
                            self.update_title();
                            self.request_redraw();
                            return;
                        }
                        // Ctrl+Shift+C — copy selection to clipboard
                        Key::Character(c) if c.as_str().eq_ignore_ascii_case("c") => {
                            if let Some(terminal) = &self.terminal {
                                if let Ok(term) = terminal.term.lock() {
                                    // Get selected text from terminal
                                    let text = term.selection_to_string();
                                    if let Some(text) = text {
                                        if let Ok(mut clip) = Clipboard::new() {
                                            let _ = clip.set_text(&text);
                                            info!("Copied {} chars", text.len());
                                        }
                                    }
                                }
                            }
                            return;
                        }
                        // Ctrl+Shift+V — paste from clipboard into terminal
                        Key::Character(c) if c.as_str().eq_ignore_ascii_case("v") => {
                            if let Ok(mut clip) = Clipboard::new() {
                                if let Ok(text) = clip.get_text() {
                                    if let Some(pty) = &self.pty {
                                        // Bracket paste mode: wrap in escape sequences
                                        let _ = pty.write(b"\x1b[200~");
                                        let _ = pty.write(text.as_bytes());
                                        let _ = pty.write(b"\x1b[201~");
                                        info!("Pasted {} chars", text.len());
                                    }
                                }
                            }
                            return;
                        }
                        // Ctrl+Shift+= — increase opacity
                        Key::Character(c) if c.as_str() == "+" || c.as_str() == "=" => {
                            self.config.adjust_opacity(0.05);
                            self.update_title();
                            self.request_redraw();
                            return;
                        }
                        // Ctrl+Shift+- — decrease opacity
                        Key::Character(c) if c.as_str() == "_" || c.as_str() == "-" => {
                            self.config.adjust_opacity(-0.05);
                            self.update_title();
                            self.request_redraw();
                            return;
                        }
                        _ => {}
                    }
                }

                // --- Ctrl-only hotkeys (font size) ---
                if ctrl && !shift {
                    match &logical_key {
                        // Ctrl+= — increase font size
                        Key::Character(c) if c.as_str() == "=" || c.as_str() == "+" => {
                            self.config.font_size = (self.config.font_size + 1.0).min(48.0);
                            self.config.save();
                            self.rebuild_renderer();
                            return;
                        }
                        // Ctrl+- — decrease font size
                        Key::Character(c) if c.as_str() == "-" => {
                            self.config.font_size = (self.config.font_size - 1.0).max(8.0);
                            self.config.save();
                            self.rebuild_renderer();
                            return;
                        }
                        // Ctrl+0 — reset font size to default
                        Key::Character(c) if c.as_str() == "0" => {
                            self.config.font_size = 14.0;
                            self.config.save();
                            self.rebuild_renderer();
                            return;
                        }
                        _ => {}
                    }
                }

                // --- Terminal input passthrough ---
                if let Some(pty) = &self.pty {
                    let bytes: Option<Vec<u8>> = if ctrl {
                        match &logical_key {
                            Key::Character(c) => {
                                let ch = c.as_str().bytes().next().unwrap_or(0);
                                if ch.is_ascii_alphabetic() {
                                    Some(vec![(ch.to_ascii_lowercase() - b'a') + 1])
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    } else {
                        match &logical_key {
                            Key::Named(NamedKey::Enter) => Some(b"\r".to_vec()),
                            Key::Named(NamedKey::Backspace) => Some(b"\x7f".to_vec()),
                            Key::Named(NamedKey::Tab) => Some(b"\t".to_vec()),
                            Key::Named(NamedKey::Escape) => Some(b"\x1b".to_vec()),
                            Key::Named(NamedKey::ArrowUp) => Some(b"\x1b[A".to_vec()),
                            Key::Named(NamedKey::ArrowDown) => Some(b"\x1b[B".to_vec()),
                            Key::Named(NamedKey::ArrowRight) => Some(b"\x1b[C".to_vec()),
                            Key::Named(NamedKey::ArrowLeft) => Some(b"\x1b[D".to_vec()),
                            Key::Named(NamedKey::Home) => Some(b"\x1b[H".to_vec()),
                            Key::Named(NamedKey::End) => Some(b"\x1b[F".to_vec()),
                            Key::Named(NamedKey::PageUp) => Some(b"\x1b[5~".to_vec()),
                            Key::Named(NamedKey::PageDown) => Some(b"\x1b[6~".to_vec()),
                            Key::Named(NamedKey::Delete) => Some(b"\x1b[3~".to_vec()),
                            Key::Named(NamedKey::Insert) => Some(b"\x1b[2~".to_vec()),
                            Key::Named(NamedKey::F1) => Some(b"\x1bOP".to_vec()),
                            Key::Named(NamedKey::F2) => Some(b"\x1bOQ".to_vec()),
                            Key::Named(NamedKey::F3) => Some(b"\x1bOR".to_vec()),
                            Key::Named(NamedKey::F4) => Some(b"\x1bOS".to_vec()),
                            Key::Named(NamedKey::F5) => Some(b"\x1b[15~".to_vec()),
                            Key::Named(NamedKey::F6) => Some(b"\x1b[17~".to_vec()),
                            Key::Named(NamedKey::F7) => Some(b"\x1b[18~".to_vec()),
                            Key::Named(NamedKey::F8) => Some(b"\x1b[19~".to_vec()),
                            Key::Named(NamedKey::F9) => Some(b"\x1b[20~".to_vec()),
                            Key::Named(NamedKey::F10) => Some(b"\x1b[21~".to_vec()),
                            Key::Named(NamedKey::F11) => Some(b"\x1b[23~".to_vec()),
                            Key::Named(NamedKey::F12) => Some(b"\x1b[24~".to_vec()),
                            _ => text.as_ref().map(|t| t.as_str().as_bytes().to_vec()),
                        }
                    };

                    if let Some(data) = bytes {
                        let _ = pty.write(&data);
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                match self.phase {
                    AppPhase::WelcomeScreen => {
                        // Render the welcome/shortcut prompt screen
                        if let (Some(surface), Some(renderer)) =
                            (&mut self.surface, &mut self.renderer)
                        {
                            let w = self.width as usize;
                            let h = self.height as usize;

                            if let Ok(mut buffer) = surface.buffer_mut() {
                                let opacity = self.config.effective_opacity();
                                renderer.render_welcome(&mut buffer, w, h, opacity);
                                let _ = buffer.present();
                            }
                        }
                    }
                    AppPhase::Terminal => {
                        // Drain PTY output into the terminal emulator
                        if let (Some(pty), Some(terminal)) = (&self.pty, &mut self.terminal) {
                            while let Some(data) = pty.try_read() {
                                terminal.process(&data);
                            }
                        }

                        // Render terminal frame
                        if let (Some(surface), Some(renderer), Some(terminal)) =
                            (&mut self.surface, &mut self.renderer, &self.terminal)
                        {
                            let w = self.width as usize;
                            let h = self.height as usize;

                            if let Ok(mut buffer) = surface.buffer_mut() {
                                let opacity = self.config.effective_opacity();
                                renderer.render_frame(&mut buffer, w, h, opacity, terminal);
                                let _ = buffer.present();
                            }
                        }
                    }
                }

                self.request_redraw();
            }

            _ => {}
        }
    }
}

pub fn run(config: Config, show_welcome: bool) -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new(config, show_welcome);
    event_loop.run_app(&mut app)?;
    Ok(())
}
