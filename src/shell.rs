/* src/shell.rs */

use anyhow::{bail, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize, Child};
use std::io::{self, Read, Write};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

// A wrapper to handle the shell subprocess
pub struct ShellProcess {
    // pty_system is no longer needed here as it's only used during creation.
    child: Box<dyn Child + Send>,
    writer: Box<dyn Write + Send>,
    pub reader_thread: Option<JoinHandle<()>>,
    pub output_buffer: Arc<Mutex<Vec<u8>>>,
}

impl ShellProcess {
    pub fn new(rows: u16, cols: u16) -> Result<Self> {
        // Use pty_system as a local variable.
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            ..Default::default()
        })?;

        let shell_program = Self::find_shell()?;
        let mut cmd = CommandBuilder::new(shell_program);
        cmd.cwd(std::env::current_dir()?);

        let child = pair.slave.spawn_command(cmd)?;
        let writer = pair.master.take_writer()?;
        let mut reader = pair.master.try_clone_reader()?;
        let output_buffer = Arc::new(Mutex::new(Vec::new()));
        let thread_buffer = output_buffer.clone();

        let reader_thread = thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let mut buffer_lock = thread_buffer.lock().unwrap();
                        buffer_lock.extend_from_slice(&buf[..n]);
                    }
                }
            }
        });

        Ok(Self {
            child,
            writer,
            reader_thread: Some(reader_thread),
            output_buffer,
        })
    }

    #[cfg(not(windows))]
    fn find_shell() -> Result<String> {
        for shell in ["fish", "zsh", "bash", "sh"] {
            if Command::new(shell).arg("-c").arg("echo").output().is_ok() {
                return Ok(shell.to_string());
            }
        }
        bail!("No compatible shell found. Please install fish, zsh, bash, or sh.");
    }
    
    #[cfg(windows)]
    fn find_shell() -> Result<String> {
        Ok("powershell.exe".to_string())
    }

    pub fn write_to_shell(&mut self, data: &[u8]) -> io::Result<()> {
        self.writer.write_all(data)
    }

    pub fn read_output(&self) -> Option<String> {
        let mut buffer_lock = self.output_buffer.lock().unwrap();
        if buffer_lock.is_empty() {
            None
        } else {
            let output = String::from_utf8_lossy(buffer_lock.as_slice()).to_string();
            buffer_lock.clear();
            Some(output)
        }
    }
}

impl Drop for ShellProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}