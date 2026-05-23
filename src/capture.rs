//! `--capture` mode: replace each child's stdout with a piped handle, buffer
//! to completion, emit captured chunks in argv order after all children exit
//! (FR-017 + AD-007).
//!
//! Without `--capture` (the default), children inherit the parent's stdout
//! and their outputs interleave nondeterministically. With `--capture`, the
//! parent guarantees argv-ordered emission — useful for tests and scripts
//! that need reproducible output (US5).
//!
//! Clarification Q2 + Q6 from spec.md:
//! - Q2: at spawn time, each child's stdout is replaced with `Stdio::piped()`.
//! - Q6: empty-stdout children emit nothing (no separator inserted).

use std::io::Read;
use std::process::{Child, ExitStatus};

use crate::fanout;
use crate::spawner;

/// Spawn all commands with piped stdout (for capture mode). Returns the
/// spawned children in argv order. On any spawn failure, kills already-spawned
/// children and returns the io error.
pub fn spawn_all_piped(cmds: &[String]) -> std::io::Result<Vec<Child>> {
    let mut children = Vec::with_capacity(cmds.len());
    for cmd in cmds {
        match spawner::spawn_one_piped_stdout(cmd) {
            Ok(c) => children.push(c),
            Err(e) => {
                for mut prior in children.into_iter() {
                    let _ = prior.kill();
                    let _ = prior.wait();
                }
                return Err(e);
            }
        }
    }
    Ok(children)
}

/// Drive the fan-out + capture: feed reader to every child's stdin, then read
/// every child's stdout into a per-argv-position buffer, then wait + collect
/// statuses, then emit captured bytes in argv order to `out`.
///
/// Returns the per-child exit statuses for downstream aggregation.
pub fn run_with_capture<R: Read, W: std::io::Write>(
    reader: R,
    children: Vec<Child>,
    out: &mut W,
) -> std::io::Result<Vec<ExitStatus>> {
    // Take ownership of every child's stdout BEFORE the fan-out loop closes
    // their stdin handles (otherwise the children would block on writes if
    // they emit before consuming all input).
    let mut children = children;
    let stdouts: Vec<Option<std::process::ChildStdout>> =
        children.iter_mut().map(|c| c.stdout.take()).collect();

    // Spawn a thread per child to drain its stdout in parallel with the
    // fan-out write loop. This prevents a child that emits a lot of stdout
    // from blocking on its own pipe buffer while we're still feeding it
    // stdin.
    let drainer_handles: Vec<std::thread::JoinHandle<std::io::Result<Vec<u8>>>> = stdouts
        .into_iter()
        .map(|maybe_stdout| {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                if let Some(mut h) = maybe_stdout {
                    h.read_to_end(&mut buf)?;
                }
                Ok(buf)
            })
        })
        .collect();

    // Run the fan-out write loop (closes stdin + waits children).
    let statuses = fanout::run(reader, children)?;

    // Join all drainer threads to collect each child's full stdout.
    let mut captured: Vec<Vec<u8>> = Vec::with_capacity(drainer_handles.len());
    for handle in drainer_handles {
        let bytes = handle
            .join()
            .map_err(|_| std::io::Error::other("capture drainer thread panicked"))??;
        captured.push(bytes);
    }

    // FR-017 + Clarification Q6: emit in argv order. Each non-empty captured
    // chunk is separated from the next by a single LF if it doesn't already
    // end with one. Empty children emit nothing (no separator inserted).
    for bytes in &captured {
        if bytes.is_empty() {
            continue;
        }
        out.write_all(bytes)?;
        if !bytes.ends_with(b"\n") {
            out.write_all(b"\n")?;
        }
    }
    out.flush()?;

    Ok(statuses)
}
