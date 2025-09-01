/* src/shell.rs */

use anyhow::{Result, bail};
use portable_pty::{Child, CommandBuilder, PtySize, native_pty_system};
use std::io::{self, Read, Write};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub struct ShellProcess {
    child: Box<dyn Child + Send>,
    writer: Box<dyn Write + Send>,
    reader_thread: Option<JoinHandle<()>>,
    pub output_buffer: Arc<Mutex<Vec<u8>>>,
}

impl ShellProcess {
    pub fn new(rows: u16, cols: u16) -> Result<Self> {
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

    pub fn read_output_bytes(&self) -> Option<Vec<u8>> {
        let mut buffer_lock = self.output_buffer.lock().unwrap();
        if buffer_lock.is_empty() {
            None
        } else {
            let output = buffer_lock.clone();
            buffer_lock.clear();
            Some(output)
        }
    }
}

impl Drop for ShellProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        if let Some(handle) = self.reader_thread.take() {
            let _ = handle.join();
        }
    }
}
