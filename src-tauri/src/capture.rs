use crate::ir;
use crate::ir::Capture;
use portable_pty::native_pty_system;
use portable_pty::{CommandBuilder, PtySize};
use std::io::Read;
use std::time::{Duration, Instant};

/// Capture limits.
pub const MAX_BUFFER: usize = 5 * 1024 * 1024; // 5 MB
pub const TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub struct CaptureError(pub String);

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CaptureError {}

/// Desktop launchers (installed .deb) start the app with a minimal PATH that
/// lacks user-local bin dirs, and `$SHELL -c` is non-login so it never
/// sources `~/.profile` to repair it. Append the usual user-local dirs that
/// actually exist so commands like `opencode` or `omp` resolve.
/// ponytail: append (system dirs keep priority); prepend instead if local
/// binaries must shadow system ones like a real terminal does.
fn augmented_path() -> Option<std::ffi::OsString> {
    let current = std::env::var_os("PATH")?;
    let home = std::env::var_os("HOME")?;
    let mut parts: Vec<_> = std::env::split_paths(&current).collect();
    for dir in ["bin", ".local/bin", ".cargo/bin", ".bun/bin", "go/bin"] {
        let candidate = std::path::Path::new(&home).join(dir);
        if candidate.is_dir() && !parts.contains(&candidate) {
            parts.push(candidate);
        }
    }
    std::env::join_paths(parts).ok()
}

/// Result of running a command in a PTY.
#[derive(Debug)]
pub struct RunOutcome {
    pub capture: Capture,
    pub exit_code: Option<u32>,
    pub truncated: bool,
    pub timed_out: bool,
}

/// Runs `command` with `$SHELL -c`, captures the raw ANSI output and
/// converts it to the IR. Blocks until the process exits, the timeout is
/// reached or the stop flag is set.
pub fn run_command(
    command: &str,
    cwd: Option<&str>,
    cols: u16,
    rows: u16,
    stop: Option<&std::sync::atomic::AtomicBool>,
) -> Result<RunOutcome, CaptureError> {
    run_command_inner(command, cwd, cols, rows, stop, TIMEOUT)
}

/// Same as `run_command` with an explicit timeout, so tests can exercise the
/// timeout path without waiting `TIMEOUT` seconds.
fn run_command_inner(
    command: &str,
    cwd: Option<&str>,
    cols: u16,
    rows: u16,
    stop: Option<&std::sync::atomic::AtomicBool>,
    timeout: Duration,
) -> Result<RunOutcome, CaptureError> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| CaptureError(e.to_string()))?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.arg("-c");
    cmd.arg(command);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    if let Some(path) = augmented_path() {
        cmd.env("PATH", path);
    }
    if let Some(dir) = cwd {
        cmd.cwd(dir);
    }

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| CaptureError(e.to_string()))?;
    drop(pair.slave); // The parent doesn't need the slave after the spawn.

    // A blocking `reader.read` can hang forever on a silent process, which
    // would defeat the deadline: a dedicated thread pushes read chunks over a
    // channel and the main loop polls it with `recv_timeout`.
    let (tx, rx) = std::sync::mpsc::channel::<std::io::Result<Vec<u8>>>();
    let reader_handle = {
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| CaptureError(e.to_string()))?;
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF: the process closed the PTY.
                    Ok(n) => {
                        if tx.send(Ok(buf[..n].to_vec())).is_err() {
                            break; // Receiver dropped: stop reading.
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        break;
                    }
                }
            }
        })
    };

    let deadline = Instant::now() + timeout;
    // Upper bound per poll so the stop flag is checked even while blocked.
    const STOP_POLL: Duration = Duration::from_millis(100);
    let mut raw: Vec<u8> = Vec::with_capacity(64 * 1024);
    let mut truncated = false;
    let mut timed_out = false;
    let mut read_err: Option<CaptureError> = None;

    loop {
        if let Some(flag) = stop {
            if flag.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            timed_out = true;
            break;
        }
        match rx.recv_timeout(remaining.min(STOP_POLL)) {
            Ok(Ok(chunk)) => {
                if raw.len() + chunk.len() <= MAX_BUFFER {
                    raw.extend_from_slice(&chunk);
                } else {
                    let room = MAX_BUFFER - raw.len();
                    raw.extend_from_slice(&chunk[..room]);
                    truncated = true;
                    break;
                }
            }
            Ok(Err(e)) => {
                read_err = Some(CaptureError(e.to_string()));
                break;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break, // EOF
        }
    }

    // Kill first: closing the PTY unblocks the reader thread's pending read.
    let _ = child.kill();
    let _ = reader_handle.join();
    if let Some(e) = read_err {
        return Err(e);
    }

    let exit_code = child.wait().ok().map(|s| s.exit_code());

    let mut parser = vt100::Parser::new(rows, cols, 0);
    parser.process(&raw);
    let capture = ir::from_vt100(&parser, command.to_string()).trimmed();

    Ok(RunOutcome {
        capture,
        exit_code,
        truncated,
        timed_out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn captures_plain_echo() {
        let out = run_command("echo hello", None, 80, 24, None).unwrap();
        assert_eq!(out.exit_code, Some(0));
        assert_eq!(out.capture.lines[0].runs[0].text, "hello");
        assert_eq!(out.capture.command_line, "echo hello");
        assert!(!out.truncated && !out.timed_out);
    }

    #[test]
    fn captures_colored_ls() {
        let out = run_command("ls --color=always /", None, 80, 24, None).unwrap();
        assert_eq!(out.exit_code, Some(0));
        assert!(out
            .capture
            .lines
            .iter()
            .any(|l| l.runs.iter().any(|r| r.fg.is_some())));
    }

    #[test]
    fn respects_cwd() {
        let out = run_command("pwd", Some("/tmp"), 80, 24, None).unwrap();
        let text: String = out.capture.lines[0]
            .runs
            .iter()
            .map(|r| r.text.as_str())
            .collect();
        assert!(text.contains("tmp"), "pwd returned: {text}");
    }

    #[test]
    fn nonzero_exit_code_propagates() {
        let out = run_command("exit 3", None, 80, 24, None).unwrap();
        assert_eq!(out.exit_code, Some(3));
    }

    #[test]
    fn narrow_pty_wraps_long_output() {
        // 15 chars at 10 columns wrap into two physical lines: the line
        // break comes from the terminal, not the exporter.
        let out = run_command("printf abcdefghijklmno", None, 10, 24, None).unwrap();
        let line = |i: usize| -> String {
            out.capture.lines[i]
                .runs
                .iter()
                .map(|r| r.text.as_str())
                .collect()
        };
        assert_eq!(line(0), "abcdefghij");
        assert_eq!(line(1), "klmno");
    }

    #[test]
    fn stop_flag_kills_hanging_command() {
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let setter = std::thread::spawn(move || {
            // Set the flag AFTER run_command has started, like the Stop button.
            std::thread::sleep(Duration::from_millis(300));
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        });
        let start = Instant::now();
        run_command("sleep 30", None, 80, 24, Some(&stop)).unwrap();
        setter.join().unwrap();
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "stop flag did not kill the command: {:?}",
            start.elapsed()
        );
    }

    #[test]
    fn augmented_path_keeps_original_dirs_and_adds_existing_user_dirs() {
        let original: Vec<_> =
            std::env::split_paths(&std::env::var_os("PATH").expect("PATH set")).collect();
        let augmented: Vec<_> =
            std::env::split_paths(&augmented_path().expect("some PATH")).collect();
        // Original dirs stay at the front, in order.
        assert_eq!(&augmented[..original.len()], &original[..]);
        // Appended dirs must exist and be user-local; none repeats another.
        for p in &augmented[original.len()..] {
            assert!(p.is_dir(), "appended dir does not exist: {p:?}");
            assert!(p.starts_with(std::env::var_os("HOME").unwrap()));
        }
        let appended: std::collections::HashSet<_> = augmented[original.len()..].iter().collect();
        assert_eq!(
            appended.len(),
            augmented.len() - original.len(),
            "duplicate appended dirs in PATH"
        );
    }

    #[test]
    fn timeout_kills_sleep() {
        // Exercise the real timeout path with a short deadline instead of
        // waiting `TIMEOUT` seconds for `sleep 5`.
        let start = Instant::now();
        let out =
            run_command_inner("sleep 5", None, 80, 24, None, Duration::from_millis(300)).unwrap();
        let elapsed = start.elapsed();
        assert!(out.timed_out, "expected timed_out = true");
        assert!(
            elapsed < Duration::from_secs(5),
            "timeout did not kill the command: {elapsed:?}"
        );
    }
}
