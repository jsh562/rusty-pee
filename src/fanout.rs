//! Core fan-out write loop (FR-002, FR-003, FR-004, FR-006).
//!
//! Reads stdin in BUFSIZ chunks and writes each chunk to every live child's
//! stdin in argv order. On `BrokenPipe` from any child, that child is dropped
//! from the live-set and the loop continues feeding the remaining children
//! (FR-003). When the parent's stdin reaches EOF, all children's stdin handles
//! are closed and the parent waits on each in argv order, collecting exit
//! codes (FR-004).

use std::io::{Read, Write};
use std::process::{Child, ChildStdin, ExitStatus};

use crate::BUFSIZ;

/// Run the fan-out loop over `children` against the bytes from `reader`.
/// Returns the collected `ExitStatus` for each child in argv order.
///
/// **Mid-chunk BrokenPipe semantics (AD-004)**: when a child returns
/// `BrokenPipe` mid-chunk, the remaining live children still receive the
/// COMPLETE current chunk in argv order before the failing child is dropped
/// from the live-set.
pub fn run<R: Read>(mut reader: R, mut children: Vec<Child>) -> std::io::Result<Vec<ExitStatus>> {
    // Pre-extract every child's stdin handle. We hold these directly (not
    // via &mut Child) so we can drop individual handles to close stdin on
    // dead children mid-loop while still being able to wait() the children
    // at the end.
    let mut stdins: Vec<Option<ChildStdin>> = children.iter_mut().map(|c| c.stdin.take()).collect();

    // Live-set: argv-order indices of children that still accept writes.
    let mut live: Vec<usize> = (0..stdins.len()).collect();

    if !stdins.is_empty() {
        let mut buf = vec![0u8; BUFSIZ];
        'outer: loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break 'outer;
            }
            let chunk = &buf[..n];

            let mut to_drop: Vec<usize> = Vec::new();
            for &idx in &live {
                if let Some(handle) = stdins[idx].as_mut() {
                    match handle.write_all(chunk) {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                            // FR-003: drop this child, continue with remaining
                            // children for THIS chunk and all subsequent chunks.
                            to_drop.push(idx);
                        }
                        Err(_) => {
                            // Other write errors (FR-003 default-on
                            // ignore_write_errors): treat the same as BrokenPipe.
                            to_drop.push(idx);
                        }
                    }
                }
            }
            for idx in to_drop {
                // Drop the stdin handle, removing the writable end of that pipe.
                stdins[idx] = None;
                live.retain(|&i| i != idx);
            }
            if live.is_empty() {
                // All children have closed; drain the rest of the input.
                loop {
                    let m = reader.read(&mut buf)?;
                    if m == 0 {
                        break;
                    }
                }
                break 'outer;
            }
        }
    } else {
        // Zero children — still drain stdin to nothing (FR-006).
        let mut buf = vec![0u8; BUFSIZ];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
        }
    }

    // FR-004: close all children's stdin handles + wait in argv order.
    drop(stdins);
    let mut statuses: Vec<ExitStatus> = Vec::with_capacity(children.len());
    for mut c in children.into_iter() {
        let status = c.wait()?;
        statuses.push(status);
    }
    Ok(statuses)
}
