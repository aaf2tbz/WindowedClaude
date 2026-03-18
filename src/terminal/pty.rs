use anyhow::{Context, Result};
use log::info;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

/// Manages a PTY session running Claude
pub struct PtySession {
    pub input_tx: mpsc::Sender<Vec<u8>>,
    pub output_rx: mpsc::Receiver<Vec<u8>>,
}

impl PtySession {
    /// Spawn Claude in a subprocess.
    /// On Windows: runs via cmd.exe with CLAUDE_CODE_GIT_BASH_PATH set.
    /// On macOS/Linux: runs via bash.
    pub fn spawn(git_bash: PathBuf, claude_exe: PathBuf, auto_accept: bool) -> Result<Self> {
        let (input_tx, input_rx) = mpsc::channel::<Vec<u8>>();
        let (output_tx, output_rx) = mpsc::channel::<Vec<u8>>();

        info!("Claude CLI at: {}", claude_exe.display());
        info!("Git Bash at: {}", git_bash.display());

        let mut cmd = if cfg!(windows) {
            // On Windows: run Claude directly via cmd.exe
            // Claude Code needs CLAUDE_CODE_GIT_BASH_PATH to find Git Bash
            let mut args = vec![claude_exe.to_string_lossy().to_string()];
            if auto_accept {
                args.push("--dangerously-skip-permissions".to_string());
                info!("Auto-accept mode enabled");
            }

            let mut c = Command::new("cmd.exe");
            c.arg("/C").args(&args);
            c.env("CLAUDE_CODE_GIT_BASH_PATH", &git_bash);
            c.env("TERM", "xterm-256color");
            c
        } else {
            // On macOS/Linux: run through bash
            let claude_cmd = if auto_accept {
                info!("Auto-accept mode enabled");
                format!("\"{}\" --dangerously-skip-permissions", claude_exe.display())
            } else {
                format!("\"{}\"", claude_exe.display())
            };

            let mut c = Command::new(&git_bash);
            c.args(["-c", &claude_cmd]);
            c.env("TERM", "xterm-256color");
            c
        };

        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn Claude process")?;

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
