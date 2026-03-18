mod pty;

pub use pty::PtySession;

use alacritty_terminal::event::Event as TermEvent;
use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
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

    /// Extract all text rows from the terminal grid (scrollback history + visible screen).
    /// Each row is trimmed of trailing whitespace. Trailing empty lines are removed.
    pub fn extract_lines(&self) -> Vec<String> {
        let term = match self.term.lock() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };
        let grid = term.grid();
        let total = grid.total_lines();
        let screen = grid.screen_lines();
        let cols = grid.columns();
        let history = total - screen;

        let mut lines = Vec::with_capacity(total);

        // History lines (negative indices, oldest first)
        for i in (0..history).rev() {
            let line_idx = Line(-(i as i32) - 1);
            let mut row = String::with_capacity(cols);
            for col in 0..cols {
                row.push(grid[line_idx][Column(col)].c);
            }
            lines.push(row.trim_end().to_string());
        }

        // Visible screen lines
        for i in 0..screen {
            let line_idx = Line(i as i32);
            let mut row = String::with_capacity(cols);
            for col in 0..cols {
                row.push(grid[line_idx][Column(col)].c);
            }
            lines.push(row.trim_end().to_string());
        }

        // Trim trailing empty lines
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }

        lines
    }

    /// Replay saved lines back through the VTE parser to populate scrollback.
    pub fn replay_lines(&mut self, lines: &[String]) {
        for line in lines {
            let mut bytes = line.as_bytes().to_vec();
            bytes.extend_from_slice(b"\r\n");
            self.process(&bytes);
        }
        // Visual separator in dim gray
        self.process(b"\x1b[90m\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80 session restored \xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\x1b[0m\r\n");
    }
}
