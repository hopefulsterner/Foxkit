//! PTY (pseudo-terminal) management

use std::path::Path;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use anyhow::Result;

use crate::{Screen, TerminalSize};

/// PTY handle
pub struct Pty {
    /// Child process ID
    #[cfg(unix)]
    pid: i32,
    /// Master FD
    #[cfg(unix)]
    master_fd: i32,
}

impl Pty {
    /// Spawn a new PTY with the given shell
    pub async fn spawn(
        shell: &str,
        cwd: &Path,
        env: &HashMap<String, String>,
        size: TerminalSize,
        screen: Arc<RwLock<Screen>>,
    ) -> Result<(Self, mpsc::UnboundedSender<Vec<u8>>)> {
        #[cfg(unix)]
        {
            Self::spawn_unix(shell, cwd, env, size, screen).await
        }
        
        #[cfg(not(unix))]
        {
            // Windows implementation would use ConPTY
            anyhow::bail!("PTY not implemented for this platform")
        }
    }

    #[cfg(unix)]
    async fn spawn_unix(
        shell: &str,
        cwd: &Path,
        env: &HashMap<String, String>,
        size: TerminalSize,
        screen: Arc<RwLock<Screen>>,
    ) -> Result<(Self, mpsc::UnboundedSender<Vec<u8>>)> {
        use std::os::unix::io::{AsRawFd, FromRawFd};
        use std::process::Stdio;
        
        // Create PTY
        let pty_pair = nix::pty::openpty(None, None)?;
        let master_fd = pty_pair.master.as_raw_fd();
        let slave_fd = pty_pair.slave.as_raw_fd();

        // Set terminal size
        let winsize = nix::pty::Winsize {
            ws_row: size.rows,
            ws_col: size.cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        
        unsafe {
            nix::libc::ioctl(master_fd, nix::libc::TIOCSWINSZ, &winsize);
        }

        // Fork and exec shell
        match unsafe { nix::unistd::fork() }? {
            nix::unistd::ForkResult::Child => {
                // Child process
                drop(pty_pair.master);
                
                // Create new session
                nix::unistd::setsid()?;
                
                // Set controlling terminal
                unsafe {
                    nix::libc::ioctl(slave_fd, nix::libc::TIOCSCTTY, 0);
                }
                
                // Redirect stdio
                nix::unistd::dup2(slave_fd, 0)?;
                nix::unistd::dup2(slave_fd, 1)?;
                nix::unistd::dup2(slave_fd, 2)?;
                
                if slave_fd > 2 {
                    nix::unistd::close(slave_fd)?;
                }
                
                // Change directory
                std::env::set_current_dir(cwd)?;
                
                // Set environment
                for (key, value) in env {
                    // SAFETY: We're in a forked child process before exec,
                    // so no other threads exist that could race
                    unsafe { std::env::set_var(key, value); }
                }
                
                // Exec shell
                let shell_cstr = std::ffi::CString::new(shell)?;
                nix::unistd::execvp(&shell_cstr, &[&shell_cstr])?;
                
                unreachable!()
            }
            nix::unistd::ForkResult::Parent { child } => {
                // Parent process
                drop(pty_pair.slave);
                
                let pid = child.as_raw();
                
                // Create input channel
                let (input_tx, mut input_rx) = mpsc::unbounded_channel::<Vec<u8>>();
                
                // Spawn read task
                let screen_clone = Arc::clone(&screen);
                let master_fd_clone = master_fd;
                tokio::spawn(async move {
                    let mut buffer = [0u8; 4096];
                    let master_file = unsafe { std::fs::File::from_raw_fd(master_fd_clone) };
                    let mut parser = vt100::Parser::new(size.rows, size.cols, 1000);
                    
                    loop {
                        use std::io::Read;
                        let mut master_ref = &master_file;
                        match master_ref.read(&mut buffer) {
                            Ok(0) => break, // EOF
                            Ok(n) => {
                                // Parse VT100 sequences
                                parser.process(&buffer[..n]);
                                
                                // Update screen
                                let vt_screen = parser.screen();
                                let mut screen = screen_clone.write();
                                screen.update_from_vt100(vt_screen);
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                            Err(_) => break,
                        }
                    }
                    
                    // Don't close the fd, we'll handle it in Pty::kill
                    std::mem::forget(master_file);
                });
                
                // Spawn write task
                let master_fd_write = master_fd;
                tokio::spawn(async move {
                    while let Some(data) = input_rx.recv().await {
                        use std::io::Write;
                        let mut master_file = unsafe { std::fs::File::from_raw_fd(master_fd_write) };
                        let _ = master_file.write_all(&data);
                        let _ = master_file.flush();
                        std::mem::forget(master_file);
                    }
                });
                
                Ok((Self { pid, master_fd }, input_tx))
            }
        }
    }

    /// Resize the PTY
    pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        #[cfg(unix)]
        {
            let winsize = nix::pty::Winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            
            unsafe {
                nix::libc::ioctl(self.master_fd, nix::libc::TIOCSWINSZ, &winsize);
            }
        }
        
        Ok(())
    }

    /// Kill the PTY process
    pub fn kill(&self) -> Result<()> {
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            
            let _ = kill(Pid::from_raw(self.pid), Signal::SIGTERM);
            
            // Close master fd
            let _ = nix::unistd::close(self.master_fd);
        }
        
        Ok(())
    }
}
