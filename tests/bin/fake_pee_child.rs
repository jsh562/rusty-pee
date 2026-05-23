//! Deterministic child-process stand-in for integration tests (AD-013, FR-026).
//!
//! Gated behind the `dev-helpers` Cargo feature — NOT installed by
//! `cargo install rusty-pee`. Tests find this binary via the
//! `CARGO_BIN_EXE_fake-pee-child` env var (set by Cargo at test build time).
//!
//! Invocation contract: `fake-pee-child --transform=<name>` (transform value
//! is the test's directive). The child reads stdin per the transform spec and
//! optionally writes a side-channel report file (path comes from the
//! `RUSTY_PEE_FAKE_CHILD_REPORT` env var).
//!
//! ## Supported transforms (per FR-026)
//!
//! | Transform | Behavior |
//! |---|---|
//! | `count` | Write line count of stdin to report file; exit 0 |
//! | `echo` | Echo stdin to stdout verbatim; exit 0 |
//! | `exit:<N>` | Consume stdin; exit with code N |
//! | `sleep-per-byte:<ms>` | Read stdin one byte at a time, sleeping <ms> per byte; exit 0 |
//! | `emit:<text>` | Write the given literal to stdout, ignore stdin; exit 0 |
//! | `report-stdin` | Write stdin verbatim to report file; exit 0 |
//! | `noop` | Consume stdin, exit 0 (silent, no report) |

use std::env;
use std::fs;
use std::io::{Read, Write};
use std::process::ExitCode;
use std::time::Duration;

const REPORT_ENV: &str = "RUSTY_PEE_FAKE_CHILD_REPORT";

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().collect();

    // Parse --transform=<name>; everything else is ignored.
    let mut transform: Option<String> = None;
    for arg in argv.iter().skip(1) {
        if let Some(value) = arg.strip_prefix("--transform=") {
            transform = Some(value.to_string());
        }
    }
    let Some(transform) = transform else {
        eprintln!("fake-pee-child: missing --transform=<name>");
        return ExitCode::from(2);
    };

    match transform.as_str() {
        "noop" => {
            // Consume stdin to avoid SIGPIPE on the parent.
            let _ = drain_stdin();
            ExitCode::SUCCESS
        }
        "echo" => match echo_stdin() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("fake-pee-child: echo failed: {e}");
                ExitCode::from(1)
            }
        },
        "count" => match count_lines() {
            Ok(n) => {
                write_report(&format!("{n}"));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("fake-pee-child: count failed: {e}");
                ExitCode::from(1)
            }
        },
        "report-stdin" => match drain_stdin() {
            Ok(bytes) => {
                write_report_bytes(&bytes);
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("fake-pee-child: report-stdin failed: {e}");
                ExitCode::from(1)
            }
        },
        t if t.starts_with("exit:") => {
            // Consume stdin to avoid SIGPIPE on the parent.
            let _ = drain_stdin();
            let code: i32 = match t["exit:".len()..].parse() {
                Ok(c) => c,
                Err(_) => {
                    eprintln!("fake-pee-child: invalid exit code");
                    return ExitCode::from(2);
                }
            };
            if code == 0 {
                return ExitCode::SUCCESS;
            }
            let byte = if (1..=255).contains(&code) {
                code as u8
            } else {
                1u8
            };
            ExitCode::from(byte)
        }
        t if t.starts_with("emit:") => {
            // Ignore stdin entirely (don't drain).
            let text = &t["emit:".len()..];
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            if let Err(e) = out.write_all(text.as_bytes()) {
                eprintln!("fake-pee-child: emit write failed: {e}");
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        t if t.starts_with("sleep-per-byte:") => {
            let ms: u64 = match t["sleep-per-byte:".len()..].parse() {
                Ok(m) => m,
                Err(_) => {
                    eprintln!("fake-pee-child: invalid sleep-per-byte ms");
                    return ExitCode::from(2);
                }
            };
            let stdin = std::io::stdin();
            let mut handle = stdin.lock();
            let mut byte = [0u8; 1];
            loop {
                match handle.read(&mut byte) {
                    Ok(0) => break,
                    Ok(_) => {
                        std::thread::sleep(Duration::from_millis(ms));
                    }
                    Err(e) => {
                        eprintln!("fake-pee-child: sleep-per-byte read failed: {e}");
                        return ExitCode::from(1);
                    }
                }
            }
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("fake-pee-child: unknown transform: {other}");
            ExitCode::from(2)
        }
    }
}

fn drain_stdin() -> std::io::Result<Vec<u8>> {
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut buf = Vec::new();
    handle.read_to_end(&mut buf)?;
    Ok(buf)
}

fn echo_stdin() -> std::io::Result<()> {
    let bytes = drain_stdin()?;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    out.write_all(&bytes)?;
    Ok(())
}

fn count_lines() -> std::io::Result<usize> {
    let bytes = drain_stdin()?;
    Ok(bytes.iter().filter(|&&b| b == b'\n').count())
}

fn write_report(s: &str) {
    if let Some(path) = env::var_os(REPORT_ENV) {
        let _ = fs::write(path, s);
    }
}

fn write_report_bytes(b: &[u8]) {
    if let Some(path) = env::var_os(REPORT_ENV) {
        let _ = fs::write(path, b);
    }
}
