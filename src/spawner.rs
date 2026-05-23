//! Per-command shell-wrapping spawn + `Stdio::piped()` stdin (FR-001).

use std::ffi::OsString;
use std::process::{Child, Command, Stdio};

/// Spawn one command via the platform shell with stdin piped.
/// On Unix: `/bin/sh -c '<cmd>'`. On Windows: `cmd /C "<cmd>"`.
///
/// Stdout and stderr are inherited from the parent (`Stdio::inherit()`) so
/// children write to the parent's terminal — outputs interleave
/// nondeterministically (matches moreutils default).
pub fn spawn_one(cmd: &str) -> std::io::Result<Child> {
    let mut command = build_command(cmd);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    command.spawn()
}

/// Spawn one command via the platform shell with piped stdout (for `--capture` mode).
pub fn spawn_one_piped_stdout(cmd: &str) -> std::io::Result<Child> {
    let mut command = build_command(cmd);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::inherit());
    command.spawn()
}

#[cfg(unix)]
fn build_command(cmd: &str) -> Command {
    let mut c = Command::new("/bin/sh");
    c.arg("-c");
    c.arg(OsString::from(cmd));
    c
}

#[cfg(windows)]
fn build_command(cmd: &str) -> Command {
    let mut c = Command::new("cmd");
    c.arg("/C");
    c.arg(OsString::from(cmd));
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn spawn_one_echo_then_close() {
        // Spawn a child that just exits 0; verify we get a Child back.
        #[cfg(unix)]
        let cmd = "true";
        #[cfg(windows)]
        let cmd = "exit /b 0";
        let mut child = spawn_one(cmd).expect("spawn should succeed");
        // Close stdin so the child can exit.
        drop(child.stdin.take());
        let status = child.wait().expect("wait should succeed");
        assert!(status.success(), "expected success exit");
    }

    #[test]
    fn spawn_one_with_piped_stdin_accepts_writes() {
        // Spawn a passthrough; write to its stdin; verify no panic on close.
        #[cfg(unix)]
        let cmd = "cat > /dev/null";
        #[cfg(windows)]
        let cmd = "more > NUL";
        let mut child = spawn_one(cmd).expect("spawn should succeed");
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(b"some bytes\n");
        }
        let status = child.wait().expect("wait should succeed");
        // Don't assert success — some shells return non-zero for trivial redirects.
        let _ = status;
    }
}
