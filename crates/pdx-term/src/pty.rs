//! Pseudo-terminal support.
//!
//! On Unix (Linux / Android) this is a real PTY built with `libc` only:
//! `openpty` + `fork` + `setsid` + `TIOCSCTTY` + `execve`. On non-Unix hosts
//! every operation returns a typed `CapabilityFailure` so the crate still
//! compiles and its host tests still run.

#[cfg(unix)]
mod unix {
    use pdx_core::{PdxError, Result};
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int, c_void};
    use std::os::unix::io::RawFd;

    /// A child process attached to the slave side of a PTY.
    pub struct Pty {
        master: RawFd,
        child: libc::pid_t,
        reaped: bool,
        exit_status: Option<i32>,
    }

    fn last_error(context: &str) -> PdxError {
        let error = std::io::Error::last_os_error();
        let errno = error.raw_os_error().unwrap_or(-1);
        PdxError::io(format!("{context} failed: {error} (errno {errno})"))
    }

    fn cstring(value: &str, what: &str) -> Result<CString> {
        CString::new(value).map_err(|_| PdxError::bad_input(format!("{what} contains a NUL byte")))
    }

    // Bionic (Android) exports `environ` but the `libc` crate does not expose
    // it on that target, so declare the symbol directly there.
    #[cfg(target_os = "android")]
    extern "C" {
        static environ: *const *const c_char;
    }

    #[cfg(target_os = "android")]
    fn inherited_environ() -> *const *const c_char {
        unsafe { environ }
    }

    #[cfg(not(target_os = "android"))]
    fn inherited_environ() -> *const *const c_char {
        unsafe { libc::environ as *const *const c_char }
    }

    fn decode_status(status: c_int) -> i32 {
        if libc::WIFEXITED(status) {
            libc::WEXITSTATUS(status)
        } else if libc::WIFSIGNALED(status) {
            128 + libc::WTERMSIG(status)
        } else {
            -1
        }
    }

    impl Pty {
        /// Spawn `shell` with `args` and `env` on a fresh PTY.
        ///
        /// `rows`/`cols` are the initial window size. If `env` is empty the
        /// child inherits the parent's environment.
        pub fn spawn(
            shell: &str,
            args: &[&str],
            env: &[(&str, &str)],
            rows: u16,
            cols: u16,
        ) -> Result<Pty> {
            let shell_c = cstring(shell, "shell path")?;
            let argv0 = cstring(shell.rsplit('/').next().unwrap_or(shell), "argv[0]")?;
            let arg_c: Result<Vec<CString>> =
                args.iter().map(|arg| cstring(arg, "argument")).collect();
            let arg_c = arg_c?;

            let env_c: Result<Vec<CString>> = env
                .iter()
                .map(|(key, value)| {
                    if key.contains('=') {
                        return Err(PdxError::bad_input("environment key contains '='"));
                    }
                    cstring(&format!("{key}={value}"), "environment entry")
                })
                .collect();
            let env_c = env_c?;

            // Keep every CString alive until after fork/exec.
            let mut argv: Vec<*const c_char> = Vec::with_capacity(arg_c.len() + 2);
            argv.push(argv0.as_ptr());
            for arg in &arg_c {
                argv.push(arg.as_ptr());
            }
            argv.push(std::ptr::null());

            let mut envp_owned: Vec<*const c_char> = Vec::with_capacity(env_c.len() + 1);
            for entry in &env_c {
                envp_owned.push(entry.as_ptr());
            }
            envp_owned.push(std::ptr::null());
            let envp: *const *const c_char = if env_c.is_empty() {
                inherited_environ()
            } else {
                envp_owned.as_ptr()
            };

            let winsize = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };

            let mut master: RawFd = -1;
            let mut slave: RawFd = -1;
            let opened = unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    &winsize,
                )
            };
            if opened < 0 {
                return Err(last_error("openpty"));
            }

            let child = unsafe { libc::fork() };
            if child < 0 {
                let error = last_error("fork");
                unsafe {
                    libc::close(master);
                    libc::close(slave);
                }
                return Err(error);
            }

            if child == 0 {
                unsafe {
                    libc::setsid();
                    libc::ioctl(slave, libc::TIOCSCTTY as _, 0);
                    libc::dup2(slave, 0);
                    libc::dup2(slave, 1);
                    libc::dup2(slave, 2);
                    if slave > 2 {
                        libc::close(slave);
                    }
                    libc::close(master);
                    libc::execve(shell_c.as_ptr(), argv.as_ptr(), envp);
                    libc::_exit(127);
                }
            }

            unsafe { libc::close(slave) };
            Ok(Pty {
                master,
                child,
                reaped: false,
                exit_status: None,
            })
        }

        /// Read output produced by the child. Blocks until data is available.
        pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            loop {
                let count = unsafe {
                    libc::read(self.master, buf.as_mut_ptr().cast::<c_void>(), buf.len())
                };
                if count >= 0 {
                    return Ok(count as usize);
                }
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                return Err(last_error("read from PTY master"));
            }
        }

        /// Write input to the child's terminal.
        pub fn write(&mut self, bytes: &[u8]) -> Result<usize> {
            loop {
                let count = unsafe {
                    libc::write(self.master, bytes.as_ptr().cast::<c_void>(), bytes.len())
                };
                if count >= 0 {
                    return Ok(count as usize);
                }
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                return Err(last_error("write to PTY master"));
            }
        }

        /// Update the child's terminal window size via `TIOCSWINSZ`.
        pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
            let winsize = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let result = unsafe {
                libc::ioctl(
                    self.master,
                    libc::TIOCSWINSZ as _,
                    &winsize as *const libc::winsize,
                )
            };
            if result < 0 {
                Err(last_error("ioctl(TIOCSWINSZ)"))
            } else {
                Ok(())
            }
        }

        /// Non-blocking child status check. Returns `Ok(None)` while running.
        pub fn try_wait(&mut self) -> Result<Option<i32>> {
            if self.reaped {
                return Ok(self.exit_status);
            }
            let mut status: c_int = 0;
            loop {
                let result = unsafe { libc::waitpid(self.child, &mut status, libc::WNOHANG) };
                if result == 0 {
                    return Ok(None);
                }
                if result == self.child {
                    let code = decode_status(status);
                    self.reaped = true;
                    self.exit_status = Some(code);
                    return Ok(Some(code));
                }
                let error = std::io::Error::last_os_error();
                match error.raw_os_error() {
                    Some(libc::EINTR) => continue,
                    Some(libc::ECHILD) => {
                        self.reaped = true;
                        self.exit_status = Some(-1);
                        return Ok(Some(-1));
                    }
                    _ => return Err(last_error("waitpid")),
                }
            }
        }

        /// Send a signal to the child process.
        pub fn kill(&self, signal: c_int) -> Result<()> {
            let result = unsafe { libc::kill(self.child, signal) };
            if result < 0 {
                Err(last_error("kill"))
            } else {
                Ok(())
            }
        }

        pub fn pid(&self) -> libc::pid_t {
            self.child
        }
    }

    impl Drop for Pty {
        fn drop(&mut self) {
            if self.master >= 0 {
                unsafe { libc::close(self.master) };
                self.master = -1;
            }
            if self.reaped {
                return;
            }
            unsafe {
                libc::kill(self.child, libc::SIGTERM);
                let mut status: c_int = 0;
                let sleep = libc::timespec {
                    tv_sec: 0,
                    tv_nsec: 10_000_000,
                };
                for _ in 0..50 {
                    let result = libc::waitpid(self.child, &mut status, libc::WNOHANG);
                    if result == self.child || result < 0 {
                        self.reaped = true;
                        break;
                    }
                    libc::nanosleep(&sleep, std::ptr::null_mut());
                }
                if !self.reaped {
                    libc::kill(self.child, libc::SIGKILL);
                    loop {
                        let result = libc::waitpid(self.child, &mut status, 0);
                        if result == self.child || result < 0 {
                            break;
                        }
                    }
                    self.reaped = true;
                }
            }
        }
    }
}

#[cfg(unix)]
pub use unix::Pty;

#[cfg(not(unix))]
mod unsupported {
    use pdx_core::{PdxError, Result};
    use std::os::raw::c_int;

    /// Placeholder type that reports a typed capability failure on hosts
    /// without Unix PTY support (notably Windows).
    #[derive(Debug)]
    pub struct Pty;

    impl Pty {
        pub fn spawn(
            _shell: &str,
            _args: &[&str],
            _env: &[(&str, &str)],
            _rows: u16,
            _cols: u16,
        ) -> Result<Pty> {
            Err(PdxError::capability(
                "PTY requires a Unix host (Android/Linux)",
            ))
        }

        pub fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
            Err(PdxError::capability(
                "PTY requires a Unix host (Android/Linux)",
            ))
        }

        pub fn write(&mut self, _bytes: &[u8]) -> Result<usize> {
            Err(PdxError::capability(
                "PTY requires a Unix host (Android/Linux)",
            ))
        }

        pub fn resize(&mut self, _rows: u16, _cols: u16) -> Result<()> {
            Err(PdxError::capability(
                "PTY requires a Unix host (Android/Linux)",
            ))
        }

        pub fn try_wait(&mut self) -> Result<Option<i32>> {
            Err(PdxError::capability(
                "PTY requires a Unix host (Android/Linux)",
            ))
        }

        pub fn kill(&self, _signal: c_int) -> Result<()> {
            Err(PdxError::capability(
                "PTY requires a Unix host (Android/Linux)",
            ))
        }
    }
}

#[cfg(not(unix))]
pub use unsupported::Pty;
