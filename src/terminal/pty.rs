use anyhow::{Context, Result};
use log::info;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

/// Manages a PTY session running Git Bash → Claude
pub struct PtySession {
    /// Channel to send input bytes to the PTY
    pub input_tx: mpsc::Sender<Vec<u8>>,
    /// Channel to receive output bytes from the PTY
    pub output_rx: mpsc::Receiver<Vec<u8>>,
}

impl PtySession {
    /// Spawn a new PTY session running Claude inside Git Bash
    pub fn spawn(git_bash: PathBuf, claude_exe: PathBuf) -> Result<Self> {
        let (input_tx, input_rx) = mpsc::channel::<Vec<u8>>();
        let (output_tx, output_rx) = mpsc::channel::<Vec<u8>>();

        info!("Spawning Git Bash at: {}", git_bash.display());
        info!("Claude CLI at: {}", claude_exe.display());

        // Spawn Git Bash with Claude as the initial command
        let mut child = Command::new(&git_bash)
            .args(["-c", &format!("\"{}\"", claude_exe.display())])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("CLAUDE_CODE_GIT_BASH_PATH", &git_bash)
            .env("TERM", "xterm-256color")
            .spawn()
            .context("Failed to spawn Git Bash process")?;

        let mut stdin = child.stdin.take().context("No stdin")?;
        let mut stdout = child.stdout.take().context("No stdout")?;

        // Thread: pipe input_rx → child stdin
        thread::spawn(move || {
            while let Ok(data) = input_rx.recv() {
                if stdin.write_all(&data).is_err() {
                    break;
                }
                let _ = stdin.flush();
            }
        });

        // Thread: pipe child stdout → output_tx
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match stdout.read(&mut buf) {
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

        Ok(Self {
            input_tx,
            output_rx,
        })
    }

    /// Send raw bytes to the PTY (keyboard input)
    pub fn write(&self, data: &[u8]) -> Result<()> {
        self.input_tx
            .send(data.to_vec())
            .context("PTY input channel closed")?;
        Ok(())
    }

    /// Try to read available output (non-blocking)
    pub fn try_read(&self) -> Option<Vec<u8>> {
        self.output_rx.try_recv().ok()
    }
}
