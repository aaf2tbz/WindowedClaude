use crate::config::Config;
use crate::installer;
use crate::terminal::{PtySession, Terminal};
use crate::ui::renderer::Renderer;
use crate::ui::theme;
use anyhow::Result;
use log::info;
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

const INITIAL_WIDTH: u32 = 1000;
const INITIAL_HEIGHT: u32 = 650;
const TITLE: &str = "ClaudeTerm";

struct App {
    config: Config,
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    renderer: Option<Renderer>,
    terminal: Option<Terminal>,
    pty: Option<PtySession>,
    modifiers: ModifiersState,
    width: u32,
    height: u32,
}

impl App {
    fn new(config: Config) -> Self {
        Self {
            config,
            window: None,
            surface: None,
            renderer: None,
            terminal: None,
            pty: None,
            modifiers: ModifiersState::empty(),
            width: INITIAL_WIDTH,
            height: INITIAL_HEIGHT,
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

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
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

        // Create renderer
        let renderer = Renderer::new(t.clone(), self.config.font_size);

        // Create terminal emulator sized to match renderer grid
        let terminal = Terminal::new(renderer.cols, renderer.rows);

        self.surface = Some(surface);
        self.renderer = Some(renderer);
        self.terminal = Some(terminal);
        self.window = Some(window);

        // Spawn the PTY process
        self.spawn_pty();
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
                    // Resize the terminal grid to match
                    if let Some(terminal) = &mut self.terminal {
                        terminal.resize(renderer.cols, renderer.rows);
                    }
                }
                self.request_redraw();
            }

            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }

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
                let ctrl = self.modifiers.control_key();
                let shift = self.modifiers.shift_key();

                // --- App hotkeys (Ctrl+Shift combos) ---
                if ctrl && shift {
                    match &logical_key {
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
                        Key::Character(c) if c.as_str().eq_ignore_ascii_case("o") => {
                            self.config.toggle_transparency();
                            self.update_title();
                            self.request_redraw();
                            return;
                        }
                        Key::Character(c) if c.as_str() == "+" || c.as_str() == "=" => {
                            self.config.adjust_opacity(0.05);
                            self.update_title();
                            self.request_redraw();
                            return;
                        }
                        Key::Character(c) if c.as_str() == "_" || c.as_str() == "-" => {
                            self.config.adjust_opacity(-0.05);
                            self.update_title();
                            self.request_redraw();
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
                // Drain PTY output into the terminal emulator
                if let (Some(pty), Some(terminal)) = (&self.pty, &mut self.terminal) {
                    while let Some(data) = pty.try_read() {
                        terminal.process(&data);
                    }
                }

                // Render frame
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

                self.request_redraw();
            }

            _ => {}
        }
    }
}

pub fn run(config: Config) -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new(config);
    event_loop.run_app(&mut app)?;
    Ok(())
}
