use crate::config::Config;
use crate::installer;
use crate::terminal::{PtySession, Terminal};
use crate::ui::renderer::Renderer;
use crate::ui::theme;
use crate::ui::theme::Color;
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
#[derive(Debug, Clone, PartialEq)]
enum AppPhase {
    /// Downloading Git + installing Claude (first run)
    Installing { status: String },
    /// Showing the welcome/shortcut prompt (first run only)
    WelcomeScreen,
    /// Normal terminal operation
    Terminal,
}

/// A single terminal tab with its own PTY + terminal emulator
struct TabState {
    id: usize,
    title: String,
    terminal: Terminal,
    pty: Option<PtySession>,
}

struct App {
    config: Config,
    phase: AppPhase,
    needs_install: bool,
    needs_welcome: bool,
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    renderer: Option<Renderer>,
    // Multi-tab terminal support
    tabs: Vec<TabState>,
    active_tab: usize,
    next_tab_id: usize,
    modifiers: ModifiersState,
    width: u32,
    height: u32,
    // Mouse state
    cursor_x: f64,
    cursor_y: f64,
    mouse_pressed: bool,
    selecting: bool,
    maximized: bool,
    settings_open: bool,
    /// Which settings row the mouse is hovering over (for visual feedback)
    settings_hover_row: i32,
    /// Flash timer for settings click feedback (counts down frames)
    settings_click_flash: u8,
    // Keybinds overlay state
    keybinds_open: bool,
    keybinds_hover_row: i32,
    keybinds_editing_index: i32,
    keybinds_waiting_for_key: bool,
    /// Working copy of keybinds being edited (not saved until user clicks Save)
    keybinds_draft: crate::config::KeyBinds,
    // Installer channel — receives status updates from background thread
    install_rx: Option<std::sync::mpsc::Receiver<installer::InstallMsg>>,
}

impl App {
    fn new(config: Config, needs_install: bool, needs_welcome: bool) -> Self {
        let phase = if needs_install {
            AppPhase::Installing { status: "Preparing...".to_string() }
        } else if needs_welcome {
            AppPhase::WelcomeScreen
        } else {
            AppPhase::Terminal
        };
        let keybinds_draft = config.keybinds.clone();
        Self {
            config,
            phase,
            needs_install,
            needs_welcome,
            window: None,
            surface: None,
            renderer: None,
            tabs: Vec::new(),
            active_tab: 0,
            next_tab_id: 0,
            modifiers: ModifiersState::empty(),
            width: INITIAL_WIDTH,
            height: INITIAL_HEIGHT,
            cursor_x: 0.0,
            cursor_y: 0.0,
            mouse_pressed: false,
            selecting: false,
            maximized: false,
            settings_open: false,
            settings_hover_row: -1,
            settings_click_flash: 0,
            keybinds_open: false,
            keybinds_hover_row: -1,
            keybinds_editing_index: -1,
            keybinds_waiting_for_key: false,
            keybinds_draft,
            install_rx: None,
        }
    }

    /// Start the installer on a background thread
    fn start_installer(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel::<installer::InstallMsg>();
        self.install_rx = Some(rx);

        std::thread::spawn(move || {
            match installer::run_first_time_setup_with_progress(&tx) {
                Ok(()) => {
                    let _ = tx.send(installer::InstallMsg::Done);
                }
                Err(e) => {
                    let _ = tx.send(installer::InstallMsg::Error(format!("{}", e)));
                }
            }
        });
    }

    fn spawn_pty(&mut self) {
        self.spawn_new_tab();
    }

    /// Spawn a new tab with its own terminal + PTY
    fn spawn_new_tab(&mut self) {
        let git_bash = self.config.git_bash_path.clone()
            .unwrap_or_else(installer::git_bash_path);
        let claude_cli = installer::claude_cli_path();

        let (cols, rows) = self.renderer.as_ref()
            .map(|r| (r.cols as u16, r.rows as u16))
            .unwrap_or((120, 35));

        let terminal = Terminal::new(cols as usize, rows as usize);

        let pty = match PtySession::spawn(git_bash, claude_cli, self.config.auto_accept, cols, rows) {
            Ok(session) => {
                info!("PTY session spawned for tab ({}x{})", cols, rows);
                Some(session)
            }
            Err(e) => {
                log::error!("Failed to spawn PTY: {}", e);
                None
            }
        };

        let id = self.next_tab_id;
        self.next_tab_id += 1;

        self.tabs.push(TabState {
            id,
            title: format!("Claude {}", self.tabs.len() + 1),
            terminal,
            pty,
        });

        self.active_tab = self.tabs.len() - 1;
    }

    /// Close the tab at the given index. Returns true if we should close the window.
    fn close_tab(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() {
            return false;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            return true; // Close window
        }
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        false
    }

    fn update_title(&self) {
        if let Some(window) = &self.window {
            let t = self.current_theme();
            let mut extras = String::new();
            if self.config.auto_accept {
                extras.push_str(" | AUTO");
            }
            if self.config.transparent {
                extras.push_str(&format!(" | {:.0}%", self.config.opacity * 100.0));
            }
            window.set_title(&format!("{} — {}{}", TITLE, t.name, extras));
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
        // Account for tab bar height when tabs > 1
        let tab_bar_offset = if self.tabs.len() > 1 { 28u32 } else { 0 };
        renderer.resize(self.width, self.height.saturating_sub(tab_bar_offset));
        let cols = renderer.cols;
        let rows = renderer.rows;
        // Resize ALL tabs
        for tab in &mut self.tabs {
            tab.terminal.resize(cols, rows);
            if let Some(pty) = &tab.pty {
                pty.resize(cols as u16, rows as u16);
            }
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
        let tab_bar_offset = if self.tabs.len() > 1 { 28.0 } else { 0.0 };
        let grid_x = renderer.grid_x() as f64;
        let grid_y = renderer.grid_y() as f64 + tab_bar_offset;

        // Must be within the terminal grid area (inside padding)
        let term_x = x - grid_x;
        let term_y = y - grid_y;
        if term_x < 0.0 || term_y < 0.0 {
            return None;
        }

        let col = (term_x / renderer.cell_width as f64) as usize;
        let row = (term_y / renderer.cell_height as f64) as usize;
        let col = col.min(renderer.cols.saturating_sub(1));
        let row = row.min(renderer.rows.saturating_sub(1));

        let cell_mid = grid_x + (col as f64 + 0.5) * renderer.cell_width as f64;
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
        // Theme pill
        if let Some(renderer) = &self.renderer {
            if renderer.theme_pill.contains(x, y) {
                return Some(TitleBarButton::ThemePill);
            }
            if renderer.settings_pill.contains(x, y) {
                return Some(TitleBarButton::SettingsPill);
            }
            if renderer.keybinds_pill.contains(x, y) {
                return Some(TitleBarButton::KeybindsPill);
            }
        }
        None
    }
}

#[derive(Debug)]
enum TitleBarButton {
    Close,
    Maximize,
    Minimize,
    ThemePill,
    SettingsPill,
    KeybindsPill,
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
            .with_transparent(true);

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

        self.surface = Some(surface);
        self.renderer = Some(renderer);
        self.window = Some(window);

        // Start the appropriate phase
        match &self.phase {
            AppPhase::Installing { .. } => {
                self.start_installer();
            }
            AppPhase::Terminal => {
                self.spawn_pty();
            }
            AppPhase::WelcomeScreen => {
                // Wait for user input
            }
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
                    let tab_bar_offset = if self.tabs.len() > 1 { 28u32 } else { 0 };
                    renderer.resize(self.width, self.height.saturating_sub(tab_bar_offset));
                    let cols = renderer.cols;
                    let rows = renderer.rows;
                    // Resize ALL tabs
                    for tab in &mut self.tabs {
                        tab.terminal.resize(cols, rows);
                        if let Some(pty) = &tab.pty {
                            pty.resize(cols as u16, rows as u16);
                        }
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
                        if let Some(tab) = self.tabs.get(self.active_tab) {
                            if let Ok(mut term) = tab.terminal.term.lock() {
                                if let Some(ref mut sel) = term.selection {
                                    sel.update(point, side);
                                }
                            }
                        }
                        self.request_redraw();
                    }
                }

                // Track keybinds hover row
                if self.keybinds_open {
                    if let Some(renderer) = &self.renderer {
                        let (px, py, pw, ph) = renderer.keybinds_panel_bounds(
                            self.width as usize, self.height as usize,
                        );
                        let mx = position.x as usize;
                        let my = position.y as usize;
                        if mx >= px && mx <= px + pw && my >= py && my <= py + ph {
                            let header_y = py + 16;
                            let row_h = renderer.cell_height + 12;
                            let sep_y = header_y + renderer.cell_height + 12 + renderer.cell_height + 4;
                            let rows_start_y = sep_y + 6;
                            if my >= rows_start_y {
                                self.keybinds_hover_row = ((my - rows_start_y) / row_h) as i32;
                            } else {
                                self.keybinds_hover_row = -1;
                            }
                        } else {
                            self.keybinds_hover_row = -1;
                        }
                    }
                    self.request_redraw();
                }

                // Track settings hover row
                if self.settings_open {
                    if let Some(renderer) = &self.renderer {
                        let (px, py, pw, ph) = renderer.settings_panel_bounds(
                            self.width as usize, self.height as usize,
                        );
                        let mx = position.x as usize;
                        let my = position.y as usize;
                        if mx >= px && mx <= px + pw && my >= py && my <= py + ph {
                            let header_y = py + 16;
                            let row_spacing = renderer.cell_height + 16;
                            let first_row_y = header_y + renderer.cell_height + 20;
                            if my >= first_row_y {
                                self.settings_hover_row = ((my - first_row_y) / row_spacing) as i32;
                            } else {
                                self.settings_hover_row = -1;
                            }
                        } else {
                            self.settings_hover_row = -1;
                        }
                    }
                    self.request_redraw();
                }

                // Update cursor icon based on edge proximity
                if let Some(window) = &self.window {
                    if let Some(dir) = self.resize_direction(position.x, position.y) {
                        window.set_cursor(CursorIcon::from(dir));
                    } else if self.in_title_bar(position.y) {
                        window.set_cursor(CursorIcon::Default);
                    } else if self.settings_open {
                        window.set_cursor(CursorIcon::Pointer);
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
                                Some(TitleBarButton::ThemePill) => {
                                    self.config.cycle_theme();
                                    let new_theme = self.current_theme().clone();
                                    if let Some(renderer) = &mut self.renderer {
                                        renderer.theme = new_theme;
                                    }
                                    self.update_title();
                                    self.request_redraw();
                                    info!("Theme (pill): {}", self.config.theme_id);
                                    return;
                                }
                                Some(TitleBarButton::SettingsPill) => {
                                    self.settings_open = !self.settings_open;
                                    self.keybinds_open = false;
                                    self.request_redraw();
                                    info!("Settings: {}", if self.settings_open { "opened" } else { "closed" });
                                    return;
                                }
                                Some(TitleBarButton::KeybindsPill) => {
                                    self.keybinds_open = !self.keybinds_open;
                                    self.settings_open = false;
                                    if self.keybinds_open {
                                        // Load fresh draft from config
                                        self.keybinds_draft = self.config.keybinds.clone();
                                        self.keybinds_editing_index = -1;
                                        self.keybinds_waiting_for_key = false;
                                    }
                                    self.request_redraw();
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
                        } else if self.keybinds_open {
                            // Keybinds overlay click handling
                            if let Some(renderer) = &self.renderer {
                                let (px, py, pw, ph) = renderer.keybinds_panel_bounds(
                                    self.width as usize, self.height as usize,
                                );
                                let mx = self.cursor_x as usize;
                                let my = self.cursor_y as usize;

                                // Close X
                                if mx >= px + pw - 30 && mx <= px + pw - 10
                                    && my >= py + 8 && my <= py + 30 {
                                    self.keybinds_open = false;
                                    self.request_redraw();
                                    return;
                                }

                                if mx >= px && mx <= px + pw && my >= py && my <= py + ph {
                                    let header_y = py + 16;
                                    let row_h = renderer.cell_height + 12;
                                    let sep_y = header_y + renderer.cell_height + 12 + renderer.cell_height + 4;
                                    let rows_start_y = sep_y + 6;
                                    let num_actions = crate::config::KEYBIND_ACTIONS.len();

                                    // Check if clicking a row
                                    if my >= rows_start_y && my < rows_start_y + num_actions * row_h {
                                        let row_idx = (my - rows_start_y) / row_h;
                                        if row_idx < num_actions {
                                            self.keybinds_editing_index = row_idx as i32;
                                            self.keybinds_waiting_for_key = true;
                                            self.request_redraw();
                                            return;
                                        }
                                    }

                                    // Check bottom buttons (after separator)
                                    let btn_y = rows_start_y + num_actions * row_h + 4 + 12;
                                    let btn_h = renderer.cell_height + 10;

                                    let btn_pad = 14;
                                    let btn_gap = 20;
                                    if my >= btn_y && my <= btn_y + btn_h {
                                        // Save button
                                        let save_w = "Save".len() * renderer.cell_width + btn_pad * 2;
                                        let save_x = px + 24;
                                        if mx >= save_x && mx <= save_x + save_w {
                                            self.config.keybinds = self.keybinds_draft.clone();
                                            self.config.save();
                                            self.keybinds_open = false;
                                            info!("Keybinds saved");
                                            self.request_redraw();
                                            return;
                                        }

                                        // Discard button
                                        let discard_w = "Discard".len() * renderer.cell_width + btn_pad * 2;
                                        let discard_x = save_x + save_w + btn_gap;
                                        if mx >= discard_x && mx <= discard_x + discard_w {
                                            self.keybinds_draft = self.config.keybinds.clone();
                                            self.keybinds_open = false;
                                            info!("Keybinds discarded");
                                            self.request_redraw();
                                            return;
                                        }

                                        // Reset button
                                        let reset_w = "Reset Defaults".len() * renderer.cell_width + btn_pad * 2;
                                        let reset_x = discard_x + discard_w + btn_gap;
                                        if mx >= reset_x && mx <= reset_x + reset_w {
                                            self.keybinds_draft.reset_all();
                                            info!("Keybinds reset to defaults");
                                            self.request_redraw();
                                            return;
                                        }
                                    }
                                } else {
                                    // Click outside panel
                                    self.keybinds_open = false;
                                    self.request_redraw();
                                }
                            }
                        } else if self.settings_open {
                            // Settings overlay is open — handle clicks within it
                            if let Some(renderer) = &self.renderer {
                                let (px, py, pw, ph) = renderer.settings_panel_bounds(
                                    self.width as usize, self.height as usize,
                                );
                                let mx = self.cursor_x as usize;
                                let my = self.cursor_y as usize;

                                // Close X button (top right of panel)
                                if mx >= px + pw - 30 && mx <= px + pw - 10
                                    && my >= py + 8 && my <= py + 30 {
                                    self.settings_open = false;
                                    self.request_redraw();
                                    return;
                                }

                                // Check if click is inside panel
                                if mx >= px && mx <= px + pw && my >= py && my <= py + ph {
                                    let row_x = px + 20;
                                    let value_x = px + 220;
                                    let header_y = py + 16;
                                    let row_spacing = renderer.cell_height + 16;
                                    let mut row_y = header_y + renderer.cell_height + 20;

                                    // Theme row
                                    if my >= row_y.saturating_sub(4) && my <= row_y + renderer.cell_height + 8 && mx >= value_x {
                                        self.config.cycle_theme();
                                        let new_theme = self.current_theme().clone();
                                        if let Some(r) = &mut self.renderer {
                                            r.theme = new_theme;
                                        }
                                        self.update_title();
                                        self.settings_click_flash = 6;
                                        self.request_redraw();
                                        return;
                                    }
                                    row_y += row_spacing;

                                    // Font size row
                                    if my >= row_y.saturating_sub(4) && my <= row_y + renderer.cell_height + 8 {
                                        // - button
                                        if mx >= value_x && mx <= value_x + 22 {
                                            self.config.font_size = (self.config.font_size - 1.0).max(8.0);
                                            self.config.save();
                                            self.settings_click_flash = 6;
                                            self.rebuild_renderer();
                                            return;
                                        }
                                        // + button (approximate position)
                                        let size_str_len = format!("{:.0}pt", self.config.font_size).len();
                                        let plus_x = value_x + 30 + size_str_len * renderer.cell_width + 8;
                                        if mx >= plus_x && mx <= plus_x + 22 {
                                            self.config.font_size = (self.config.font_size + 1.0).min(48.0);
                                            self.config.save();
                                            self.settings_click_flash = 6;
                                            self.rebuild_renderer();
                                            return;
                                        }
                                    }
                                    row_y += row_spacing;

                                    // Transparency row
                                    if my >= row_y.saturating_sub(4) && my <= row_y + renderer.cell_height + 8 && mx >= value_x {
                                        self.config.toggle_transparency();
                                        self.update_title();
                                        self.settings_click_flash = 6;
                                        self.request_redraw();
                                        return;
                                    }
                                    row_y += row_spacing;

                                    // Opacity row (if transparency on)
                                    if self.config.transparent {
                                        if my >= row_y.saturating_sub(4) && my <= row_y + renderer.cell_height + 8 {
                                            if mx >= value_x && mx <= value_x + 22 {
                                                self.config.adjust_opacity(-0.05);
                                                self.update_title();
                                                self.settings_click_flash = 6;
                                                self.request_redraw();
                                                return;
                                            }
                                            let opacity_str_len = format!("{:.0}%", self.config.opacity * 100.0).len();
                                            let plus_x = value_x + 30 + opacity_str_len * renderer.cell_width + 8;
                                            if mx >= plus_x && mx <= plus_x + 22 {
                                                self.config.adjust_opacity(0.05);
                                                self.update_title();
                                                self.settings_click_flash = 6;
                                                self.request_redraw();
                                                return;
                                            }
                                        }
                                        row_y += row_spacing;
                                    }

                                    // Reinstall shortcuts button
                                    let btn_text = "Reinstall Shortcuts";
                                    let btn_w = btn_text.len() * renderer.cell_width + 24;
                                    let btn_h = renderer.cell_height + 12;
                                    if mx >= row_x && mx <= row_x + btn_w
                                        && my >= row_y && my <= row_y + btn_h {
                                        info!("Reinstalling shortcuts...");
                                        if let Err(e) = installer::shortcuts::create_desktop_shortcut() {
                                            log::warn!("Desktop shortcut failed: {}", e);
                                        }
                                        if let Err(e) = installer::shortcuts::create_start_menu_shortcut() {
                                            log::warn!("Start menu shortcut failed: {}", e);
                                        }
                                        self.settings_click_flash = 6;
                                        self.request_redraw();
                                        return;
                                    }
                                } else {
                                    // Click outside panel = close settings
                                    self.settings_open = false;
                                    self.request_redraw();
                                }
                            }
                        } else {
                            // Check if click is on tab bar
                            if self.tabs.len() > 1 {
                                let tab_bar_y = TITLE_BAR_HEIGHT as usize;
                                let tab_bar_h = 28usize;
                                if (self.cursor_y as usize) >= tab_bar_y
                                    && (self.cursor_y as usize) < tab_bar_y + tab_bar_h
                                {
                                    // Determine which tab was clicked
                                    let tab_w = 150usize;
                                    let mx = self.cursor_x as usize;
                                    let tab_idx = mx / tab_w;
                                    if tab_idx < self.tabs.len() {
                                        // Check for close button (last 20px of tab)
                                        let tab_end = (tab_idx + 1) * tab_w;
                                        if mx >= tab_end.saturating_sub(24) && mx < tab_end {
                                            if self.close_tab(tab_idx) {
                                                event_loop.exit();
                                            }
                                        } else {
                                            self.active_tab = tab_idx;
                                        }
                                        self.request_redraw();
                                        return;
                                    }
                                }
                            }

                            // Terminal area — start text selection
                            if let Some((point, side)) = self.pixel_to_point(self.cursor_x, self.cursor_y) {
                                self.selecting = true;
                                if let Some(tab) = self.tabs.get(self.active_tab) {
                                    if let Ok(mut term) = tab.terminal.term.lock() {
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

                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if let Ok(mut term) = tab.terminal.term.lock() {
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
                // Block keyboard during install
                if matches!(self.phase, AppPhase::Installing { .. }) {
                    return;
                }

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

                // Close settings on Escape
                if self.settings_open {
                    if matches!(&logical_key, Key::Named(NamedKey::Escape)) {
                        self.settings_open = false;
                        self.request_redraw();
                        return;
                    }
                    return;
                }

                // Keybinds overlay keyboard handling
                if self.keybinds_open {
                    if self.keybinds_waiting_for_key {
                        // Escape cancels editing
                        if matches!(&logical_key, Key::Named(NamedKey::Escape)) {
                            self.keybinds_waiting_for_key = false;
                            self.keybinds_editing_index = -1;
                            self.request_redraw();
                            return;
                        }

                        // Capture the key combo
                        let ctrl = self.modifiers.control_key();
                        let shift_mod = self.modifiers.shift_key();

                        let key_name = match &logical_key {
                            Key::Character(c) => {
                                let s = c.as_str().to_uppercase();
                                Some(s)
                            }
                            Key::Named(NamedKey::Tab) => Some("Tab".to_string()),
                            Key::Named(NamedKey::Enter) => Some("Enter".to_string()),
                            Key::Named(NamedKey::Backspace) => Some("Backspace".to_string()),
                            Key::Named(NamedKey::Delete) => Some("Delete".to_string()),
                            Key::Named(NamedKey::ArrowUp) => Some("Up".to_string()),
                            Key::Named(NamedKey::ArrowDown) => Some("Down".to_string()),
                            Key::Named(NamedKey::ArrowLeft) => Some("Left".to_string()),
                            Key::Named(NamedKey::ArrowRight) => Some("Right".to_string()),
                            Key::Named(NamedKey::Home) => Some("Home".to_string()),
                            Key::Named(NamedKey::End) => Some("End".to_string()),
                            Key::Named(NamedKey::PageUp) => Some("PageUp".to_string()),
                            Key::Named(NamedKey::PageDown) => Some("PageDown".to_string()),
                            Key::Named(NamedKey::F1) => Some("F1".to_string()),
                            Key::Named(NamedKey::F2) => Some("F2".to_string()),
                            Key::Named(NamedKey::F3) => Some("F3".to_string()),
                            Key::Named(NamedKey::F4) => Some("F4".to_string()),
                            Key::Named(NamedKey::F5) => Some("F5".to_string()),
                            Key::Named(NamedKey::F6) => Some("F6".to_string()),
                            Key::Named(NamedKey::F7) => Some("F7".to_string()),
                            Key::Named(NamedKey::F8) => Some("F8".to_string()),
                            Key::Named(NamedKey::F9) => Some("F9".to_string()),
                            Key::Named(NamedKey::F10) => Some("F10".to_string()),
                            Key::Named(NamedKey::F11) => Some("F11".to_string()),
                            Key::Named(NamedKey::F12) => Some("F12".to_string()),
                            _ => None,
                        };

                        if let Some(key) = key_name {
                            // Build combo string
                            let mut combo = String::new();
                            if ctrl { combo.push_str("Ctrl+"); }
                            if shift_mod { combo.push_str("Shift+"); }
                            combo.push_str(&key);

                            // Apply to draft
                            let idx = self.keybinds_editing_index as usize;
                            if idx < crate::config::KEYBIND_ACTIONS.len() {
                                let action_id = crate::config::KEYBIND_ACTIONS[idx].0;
                                self.keybinds_draft.set(action_id, &combo);
                                info!("Keybind set: {} = {}", action_id, combo);
                            }
                            self.keybinds_waiting_for_key = false;
                            self.keybinds_editing_index = -1;
                            self.request_redraw();
                        }
                        return;
                    }

                    // Not waiting for key — Escape closes overlay
                    if matches!(&logical_key, Key::Named(NamedKey::Escape)) {
                        self.keybinds_open = false;
                        self.request_redraw();
                        return;
                    }
                    return;
                }

                let ctrl = self.modifiers.control_key();
                let shift = self.modifiers.shift_key();

                // --- Config-driven keybind matching ---
                // Build the key name from the pressed key
                let key_name: Option<String> = match &logical_key {
                    Key::Character(c) => {
                        let s = c.as_str();
                        // Normalize: "+" and "=" are the same physical key
                        if s == "+" { Some("=".to_string()) }
                        else if s == "_" { Some("-".to_string()) }
                        else { Some(s.to_uppercase()) }
                    }
                    Key::Named(NamedKey::Tab) => Some("Tab".to_string()),
                    _ => None,
                };

                if let Some(ref key) = key_name {
                    use crate::config::KeyBinds;
                    let kb = &self.config.keybinds;

                    // New Tab
                    if KeyBinds::combo_matches(kb.get("new_tab"), ctrl, shift, key) {
                        if self.phase == AppPhase::Terminal {
                            self.spawn_new_tab();
                            self.rebuild_renderer();
                            info!("New tab (total: {})", self.tabs.len());
                        }
                        return;
                    }

                    // Close Tab
                    if KeyBinds::combo_matches(kb.get("close_tab"), ctrl, shift, key) {
                        if self.phase == AppPhase::Terminal && !self.tabs.is_empty() {
                            let idx = self.active_tab;
                            if self.close_tab(idx) {
                                event_loop.exit();
                            } else {
                                self.rebuild_renderer();
                            }
                            self.request_redraw();
                            info!("Closed tab (remaining: {})", self.tabs.len());
                        }
                        return;
                    }

                    // Next Tab
                    if KeyBinds::combo_matches(kb.get("next_tab"), ctrl, shift, key) {
                        if !self.tabs.is_empty() {
                            self.active_tab = (self.active_tab + 1) % self.tabs.len();
                            self.request_redraw();
                        }
                        return;
                    }

                    // Prev Tab
                    if KeyBinds::combo_matches(kb.get("prev_tab"), ctrl, shift, key) {
                        if !self.tabs.is_empty() {
                            self.active_tab = if self.active_tab == 0 {
                                self.tabs.len() - 1
                            } else {
                                self.active_tab - 1
                            };
                            self.request_redraw();
                        }
                        return;
                    }

                    // Toggle Transparency
                    if KeyBinds::combo_matches(kb.get("toggle_transparency"), ctrl, shift, key) {
                        self.config.toggle_transparency();
                        self.update_title();
                        self.request_redraw();
                        return;
                    }

                    // Copy
                    if KeyBinds::combo_matches(kb.get("copy"), ctrl, shift, key) {
                        if let Some(tab) = self.tabs.get(self.active_tab) {
                            if let Ok(term) = tab.terminal.term.lock() {
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

                    // Paste
                    if KeyBinds::combo_matches(kb.get("paste"), ctrl, shift, key) {
                        if let Ok(mut clip) = Clipboard::new() {
                            if let Ok(text) = clip.get_text() {
                                if let Some(tab) = self.tabs.get(self.active_tab) {
                                    if let Some(pty) = &tab.pty {
                                        let _ = pty.write(b"\x1b[200~");
                                        let _ = pty.write(text.as_bytes());
                                        let _ = pty.write(b"\x1b[201~");
                                        info!("Pasted {} chars", text.len());
                                    }
                                }
                            }
                        }
                        return;
                    }

                    // Increase Opacity
                    if KeyBinds::combo_matches(kb.get("increase_opacity"), ctrl, shift, key) {
                        self.config.adjust_opacity(0.05);
                        self.update_title();
                        self.request_redraw();
                        return;
                    }

                    // Decrease Opacity
                    if KeyBinds::combo_matches(kb.get("decrease_opacity"), ctrl, shift, key) {
                        self.config.adjust_opacity(-0.05);
                        self.update_title();
                        self.request_redraw();
                        return;
                    }

                    // Font Size +
                    if KeyBinds::combo_matches(kb.get("increase_font"), ctrl, shift, key) {
                        self.config.font_size = (self.config.font_size + 1.0).min(48.0);
                        self.config.save();
                        self.rebuild_renderer();
                        return;
                    }

                    // Font Size -
                    if KeyBinds::combo_matches(kb.get("decrease_font"), ctrl, shift, key) {
                        self.config.font_size = (self.config.font_size - 1.0).max(8.0);
                        self.config.save();
                        self.rebuild_renderer();
                        return;
                    }

                    // Font Reset
                    if KeyBinds::combo_matches(kb.get("reset_font"), ctrl, shift, key) {
                        self.config.font_size = 14.0;
                        self.config.save();
                        self.rebuild_renderer();
                        return;
                    }
                }

                // Ctrl+1-9 — jump to tab by index (always hardcoded, not rebindable)
                if ctrl && !shift {
                    if let Key::Character(c) = &logical_key {
                        let ch = c.as_str().bytes().next().unwrap_or(0);
                        if ch >= b'1' && ch <= b'9' {
                            let idx = (ch - b'1') as usize;
                            if idx < self.tabs.len() {
                                self.active_tab = idx;
                                self.request_redraw();
                            }
                            return;
                        }
                    }
                }

                // --- Terminal input passthrough ---
                let active_pty = self.tabs.get(self.active_tab).and_then(|t| t.pty.as_ref());
                if let Some(pty) = active_pty {
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
                // Drain install progress messages (collect first to avoid borrow conflict)
                let mut install_msgs = Vec::new();
                if let Some(rx) = &self.install_rx {
                    while let Ok(msg) = rx.try_recv() {
                        install_msgs.push(msg);
                    }
                }
                for msg in install_msgs {
                    match msg {
                        installer::InstallMsg::Progress(s) => {
                            self.phase = AppPhase::Installing { status: s };
                        }
                        installer::InstallMsg::Done => {
                            info!("Installation complete");
                            self.install_rx = None;
                            if self.needs_welcome {
                                self.phase = AppPhase::WelcomeScreen;
                            } else {
                                self.phase = AppPhase::Terminal;
                                self.spawn_pty();
                            }
                        }
                        installer::InstallMsg::Error(e) => {
                            log::error!("Install failed: {}", e);
                            self.phase = AppPhase::Installing {
                                status: format!("ERROR: {}", e),
                            };
                            self.install_rx = None;
                        }
                    }
                }

                // Clone phase to avoid borrow issues with self
                let phase = self.phase.clone();

                match &phase {
                    AppPhase::Installing { status } => {
                        // Render install progress screen
                        if let (Some(surface), Some(renderer)) =
                            (&mut self.surface, &mut self.renderer)
                        {
                            let w = self.width as usize;
                            let h = self.height as usize;

                            if let Ok(mut buffer) = surface.buffer_mut() {
                                let opacity = self.config.effective_opacity();
                                renderer.render_install_progress(&mut buffer, w, h, opacity, status);
                                let _ = buffer.present();
                            }
                        }
                    }
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
                        // Drain PTY output for ALL tabs (prevents memory buildup)
                        for tab in &mut self.tabs {
                            if let Some(pty) = &tab.pty {
                                while let Some(data) = pty.try_read() {
                                    tab.terminal.process(&data);
                                }
                            }
                        }

                        // Render active tab's terminal frame
                        if let Some(surface) = &mut self.surface {
                            if let Some(renderer) = &mut self.renderer {
                                let w = self.width as usize;
                                let h = self.height as usize;
                                let tab_count = self.tabs.len();
                                let active = self.active_tab;

                                if let Ok(mut buffer) = surface.buffer_mut() {
                                    let opacity = self.config.effective_opacity();

                                    // Render active tab's terminal
                                    if let Some(tab) = self.tabs.get(active) {
                                        if tab_count > 1 {
                                            // With tab bar: render frame in offset area
                                            let tab_bar_h = 28usize;
                                            renderer.render_frame(&mut buffer, w, h, opacity, &tab.terminal);

                                            // Render tab bar (between title bar and terminal grid)
                                            let tab_bar_y = renderer.title_bar_height;
                                            let tab_bar_bg = Color::rgb(
                                                renderer.theme.title_bar_bg.r,
                                                renderer.theme.title_bar_bg.g,
                                                renderer.theme.title_bar_bg.b,
                                            );
                                            Renderer::fill_rect(&mut buffer, w, 0, tab_bar_y, w, tab_bar_h, tab_bar_bg);

                                            // Draw tabs
                                            let tab_w = 150usize;
                                            for (i, t) in self.tabs.iter().enumerate() {
                                                let tx = i * tab_w;
                                                let is_active = i == active;

                                                // Active tab bg
                                                if is_active {
                                                    let active_bg = Color::rgb(
                                                        renderer.theme.title_bar_bg.r.saturating_add(renderer.theme.cursor.r / 6),
                                                        renderer.theme.title_bar_bg.g.saturating_add(renderer.theme.cursor.g / 6),
                                                        renderer.theme.title_bar_bg.b.saturating_add(renderer.theme.cursor.b / 6),
                                                    );
                                                    Renderer::fill_rect(&mut buffer, w, tx, tab_bar_y, tab_w, tab_bar_h, active_bg);
                                                    // Bottom accent line
                                                    Renderer::fill_rect(&mut buffer, w, tx, tab_bar_y + tab_bar_h - 2, tab_w, 2, renderer.theme.cursor);
                                                }

                                                // Tab title (truncated)
                                                let title: String = t.title.chars().take(14).collect();
                                                let text_color = if is_active {
                                                    renderer.theme.fg
                                                } else {
                                                    Color::rgb(renderer.theme.fg.r / 2 + 30, renderer.theme.fg.g / 2 + 30, renderer.theme.fg.b / 2 + 30)
                                                };
                                                renderer.render_string(&mut buffer, w, tx + 8, tab_bar_y + 6, &title, text_color);

                                                // Close X button
                                                let close_x = tx + tab_w - 20;
                                                let close_color = Color::rgb(renderer.theme.fg.r / 3 + 20, renderer.theme.fg.g / 3 + 20, renderer.theme.fg.b / 3 + 20);
                                                renderer.render_string(&mut buffer, w, close_x, tab_bar_y + 6, "x", close_color);

                                                // Right separator
                                                if i < tab_count - 1 {
                                                    Renderer::fill_rect(&mut buffer, w, tx + tab_w - 1, tab_bar_y + 4, 1, tab_bar_h - 8, renderer.theme.window_border);
                                                }
                                            }
                                        } else {
                                            // Single tab: no tab bar, render normally
                                            renderer.render_frame(&mut buffer, w, h, opacity, &tab.terminal);
                                        }
                                    }

                                    // Settings overlay (on top of terminal)
                                    if self.settings_open {
                                        renderer.render_settings_overlay(
                                            &mut buffer, w, h, &self.config,
                                            self.settings_hover_row, self.settings_click_flash,
                                        );
                                        if self.settings_click_flash > 0 {
                                            self.settings_click_flash -= 1;
                                        }
                                    }

                                    // Keybinds overlay (on top of terminal)
                                    if self.keybinds_open {
                                        renderer.render_keybinds_overlay(
                                            &mut buffer, w, h,
                                            &self.config.keybinds,
                                            &self.keybinds_draft,
                                            self.keybinds_editing_index,
                                            self.keybinds_hover_row,
                                            self.keybinds_waiting_for_key,
                                        );
                                    }

                                    let _ = buffer.present();
                                }
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

pub fn run(config: Config, needs_install: bool, needs_welcome: bool) -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new(config, needs_install, needs_welcome);
    event_loop.run_app(&mut app)?;
    Ok(())
}
