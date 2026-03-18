use anyhow::{Context, Result};
use log::info;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

/// Reader buffer size — large enough for ConPTY to avoid backpressure.
/// When Claude runs agents/bash, output bursts can be 100KB+. A small buffer
/// causes ConPTY's internal pipe to fill, blocking the child process on write
/// until the reader drains it. 64KB matches our per-frame render budget and
/// keeps the ConPTY pipe flowing smoothly.
const PTY_READ_BUF_SIZE: usize = 64 * 1024;

/// Manages a real PTY session running Claude.
/// Uses ConPTY on Windows, Unix PTY on macOS/Linux.
/// This gives Claude a real terminal so its TUI renders properly.
pub struct PtySession {
    pub input_tx: mpsc::Sender<Vec<u8>>,
    pub output_rx: mpsc::Receiver<Vec<u8>>,
    // Keep the master PTY alive — dropping it kills the child
    _master: Box<dyn portable_pty::MasterPty + Send>,
    // Keep the child handle alive so it isn't orphaned (critical on Windows ConPTY)
    _child: Arc<Mutex<Box<dyn portable_pty::Child + Send + Sync>>>,
    // Separate kill handle — can be called while child is locked elsewhere
    child_killer: Box<dyn portable_pty::ChildKiller + Send + Sync>,
    // Signals when the PTY reader thread has exited (pipe broken / child dead)
    reader_alive: Arc<AtomicBool>,
}

impl PtySession {
    /// Spawn Claude in a real PTY.
    pub fn spawn(
        git_bash: PathBuf,
        claude_exe: PathBuf,
        auto_accept: bool,
        continue_session: bool,
        cols: u16,
        rows: u16,
    ) -> Result<Self> {
        info!("Claude CLI at: {}", claude_exe.display());
        info!("Git Bash at: {}", git_bash.display());
        info!("PTY size: {}x{}", cols, rows);

        // Create a real PTY (ConPTY on Windows)
        let pty_system = NativePtySystem::default();
        let pty_pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to open PTY")?;

        // Build the command
        let cmd = if cfg!(windows) {
            // On Windows: run Claude directly, set Git Bash path
            let mut c = CommandBuilder::new(&claude_exe);
            if auto_accept {
                c.arg("--dangerously-skip-permissions");
                info!("Auto-accept mode enabled");
            }
            if continue_session {
                c.arg("--continue");
                info!("Continuing previous conversation");
            }
            c.env("CLAUDE_CODE_GIT_BASH_PATH", git_bash.to_string_lossy().as_ref());
            c.env("TERM", "xterm-256color");
            c
        } else {
            // On macOS/Linux: run Claude directly (no bash wrapper needed)
            let mut c = CommandBuilder::new(&claude_exe);
            if auto_accept {
                c.arg("--dangerously-skip-permissions");
                info!("Auto-accept mode enabled");
            }
            if continue_session {
                c.arg("--continue");
                info!("Continuing previous conversation");
            }
            c.env("TERM", "xterm-256color");
            c.env("COLORTERM", "truecolor");
            c.env("LANG", "en_US.UTF-8");
            c.env("LC_ALL", "en_US.UTF-8");
            // Ensure Claude knows it's in a real terminal
            c.env("FORCE_COLOR", "1");
            // Load the user's full login shell PATH so hooks (Signet, etc.) find all tools
            // This covers ~/.local/bin, ~/.bun/bin, homebrew, nvm, etc.
            let home = dirs::home_dir().unwrap_or_default();
            let shell_path = std::process::Command::new("/bin/bash")
                .args(["-lc", "echo $PATH"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| std::env::var("PATH").unwrap_or_default());
            c.env("PATH", &shell_path);
            c.env("HOME", home.to_string_lossy().as_ref());
            c
        };

        // Spawn the child process in the PTY
        let child = pty_pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn Claude in PTY")?;

        // Extract a kill handle before wrapping child in Arc<Mutex>
        let child_killer = child.clone_killer();
        let child = Arc::new(Mutex::new(child));

        // Drop the slave — the child owns it now
        drop(pty_pair.slave);

        // Get read/write handles to the master side of the PTY
        let mut reader = pty_pair
            .master
            .try_clone_reader()
            .context("Failed to clone PTY reader")?;
        let mut writer = pty_pair
            .master
            .take_writer()
            .context("Failed to take PTY writer")?;

        let (input_tx, input_rx) = mpsc::channel::<Vec<u8>>();
        let (output_tx, output_rx) = mpsc::channel::<Vec<u8>>();

        // Thread: pipe input_rx → PTY master (keyboard input → Claude)
        thread::spawn(move || {
            while let Ok(data) = input_rx.recv() {
                if writer.write_all(&data).is_err() {
                    break;
                }
                let _ = writer.flush();
            }
        });

        // Thread: pipe PTY master → output_tx (Claude output → renderer)
        // Uses a large buffer (64KB) to prevent ConPTY backpressure on Windows.
        // When Claude runs agents/bash producing heavy output, a small buffer
        // causes the ConPTY internal pipe to fill, blocking the child on write
        // and eventually killing it when the pipe stalls.
        let reader_alive = Arc::new(AtomicBool::new(true));
        let reader_alive_clone = reader_alive.clone();
        thread::spawn(move || {
            let mut buf = vec![0u8; PTY_READ_BUF_SIZE];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if output_tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        // On Windows, ConPTY can return transient errors during
                        // heavy output. Only break on fatal pipe errors.
                        if e.kind() == std::io::ErrorKind::BrokenPipe
                            || e.kind() == std::io::ErrorKind::UnexpectedEof
                        {
                            break;
                        }
                        // For other errors (WouldBlock, Interrupted), retry
                        if e.kind() == std::io::ErrorKind::Interrupted {
                            continue;
                        }
                        // Unknown error — log and bail
                        log::warn!("PTY read error: {}", e);
                        break;
                    }
                }
            }
            reader_alive_clone.store(false, Ordering::Relaxed);
        });

        info!("PTY session spawned successfully");

        Ok(Self {
            input_tx,
            output_rx,
            _master: pty_pair.master,
            _child: child,
            child_killer,
            reader_alive,
        })
    }

    /// Resize the PTY (called when the window resizes)
    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self._master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }

    pub fn write(&self, data: &[u8]) -> Result<()> {
        self.input_tx
            .send(data.to_vec())
            .context("PTY input channel closed")?;
        Ok(())
    }

    pub fn try_read(&self) -> Option<Vec<u8>> {
        self.output_rx.try_recv().ok()
    }

    /// Check if the child process is still running.
    /// Returns Ok(None) if alive, Ok(Some(status)) if exited.
    pub fn try_wait(&self) -> std::io::Result<Option<portable_pty::ExitStatus>> {
        self._child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .try_wait()
    }

    /// Forcefully kill the child process.
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.child_killer.kill()
    }

    /// Whether the PTY reader thread is still alive (pipe not broken).
    pub fn is_reader_alive(&self) -> bool {
        self.reader_alive.load(Ordering::Relaxed)
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        // Kill the child first to prevent ConPTY hang on Windows.
        // ClosePseudoConsole() blocks if the child is still alive with buffered output.
        let _ = self.child_killer.kill();
    }
}
