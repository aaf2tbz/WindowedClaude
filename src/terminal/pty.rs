use anyhow::{Context, Result};
use log::info;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

/// Manages a real PTY session running Claude.
/// Uses ConPTY on Windows, Unix PTY on macOS/Linux.
/// This gives Claude a real terminal so its TUI renders properly.
pub struct PtySession {
    pub input_tx: mpsc::Sender<Vec<u8>>,
    pub output_rx: mpsc::Receiver<Vec<u8>>,
    // Keep the master PTY alive — dropping it kills the child
    _master: Box<dyn portable_pty::MasterPty + Send>,
}

impl PtySession {
    /// Spawn Claude in a real PTY.
    pub fn spawn(
        git_bash: PathBuf,
        claude_exe: PathBuf,
        auto_accept: bool,
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
        let mut cmd = if cfg!(windows) {
            // On Windows: run Claude directly, set Git Bash path
            let mut c = CommandBuilder::new(&claude_exe);
            if auto_accept {
                c.arg("--dangerously-skip-permissions");
                info!("Auto-accept mode enabled");
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
            c.env("TERM", "xterm-256color");
            c.env("COLORTERM", "truecolor");
            c.env("LANG", "en_US.UTF-8");
            c.env("LC_ALL", "en_US.UTF-8");
            // Ensure Claude knows it's in a real terminal
            c.env("FORCE_COLOR", "1");
            // Ensure ~/.local/bin is in PATH so Claude's hooks (Signet, etc.) can find tools
            let home = dirs::home_dir().unwrap_or_default();
            let local_bin = home.join(".local").join("bin");
            let current_path = std::env::var("PATH").unwrap_or_default();
            if !current_path.contains(&local_bin.to_string_lossy().to_string()) {
                c.env("PATH", format!("{}:{}", local_bin.display(), current_path));
            } else {
                c.env("PATH", &current_path);
            }
            // Pass HOME so hooks can resolve ~ paths
            c.env("HOME", home.to_string_lossy().as_ref());
            c
        };

        // Spawn the child process in the PTY
        let _child = pty_pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn Claude in PTY")?;

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
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if output_tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        info!("PTY session spawned successfully");

        Ok(Self {
            input_tx,
            output_rx,
            _master: pty_pair.master,
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
}
