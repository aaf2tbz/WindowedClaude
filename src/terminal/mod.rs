mod pty;

pub use pty::PtySession;

use alacritty_terminal::event::Event as TermEvent;
use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::Config as TermConfig;
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi;
use std::sync::{Arc, Mutex};

/// Event listener — no-op for now (we poll instead of event-driven)
#[derive(Clone)]
pub struct EventProxy;

impl EventListener for EventProxy {
    fn send_event(&self, _event: TermEvent) {}
}

/// Simple dimensions for creating a Term
struct TermSize {
    columns: usize,
    screen_lines: usize,
}

impl Dimensions for TermSize {
    fn total_lines(&self) -> usize {
        self.screen_lines
    }

    fn screen_lines(&self) -> usize {
        self.screen_lines
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

/// Terminal state: wraps alacritty_terminal::Term + VTE parser
pub struct Terminal {
    pub term: Arc<Mutex<Term<EventProxy>>>,
    parser: ansi::Processor,
}

impl Terminal {
    pub fn new(cols: usize, rows: usize) -> Self {
        let size = TermSize {
            columns: cols,
            screen_lines: rows,
        };
        let term = Term::new(TermConfig::default(), &size, EventProxy);

        Self {
            term: Arc::new(Mutex::new(term)),
            parser: ansi::Processor::new(),
        }
    }

    /// Feed raw bytes from the PTY into the VTE parser → Term
    pub fn process(&mut self, bytes: &[u8]) {
        if let Ok(mut term) = self.term.lock() {
            self.parser.advance(&mut *term, bytes);
        }
    }

    /// Resize the terminal grid
    pub fn resize(&mut self, cols: usize, rows: usize) {
        if let Ok(mut term) = self.term.lock() {
            let size = TermSize {
                columns: cols,
                screen_lines: rows,
            };
            term.resize(size);
        }
    }
}
