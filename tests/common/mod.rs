//! Shared test harness helpers for integration tests.

#![allow(dead_code)]

use std::path::PathBuf;

/// Locate the `fake-pee-child` helper binary built by Cargo with the
/// `dev-helpers` feature. `CARGO_BIN_EXE_<name>` is set by Cargo at test
/// build time per https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates.
pub fn fake_pee_child_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake-pee-child"))
}

/// Allocate a fresh tempdir scoped to the test.
pub fn with_tempdir() -> tempfile::TempDir {
    tempfile::tempdir().expect("could not create test tempdir")
}

/// Build a shell command string that invokes the fake-pee-child with the given
/// transform, redirecting its stdout to a side-channel file. The path is
/// normalized to forward slashes so `/bin/sh -c` (Unix) and `cmd /C` (Windows)
/// both handle it correctly. Quoting is intentionally omitted — both rusty-pee
/// build paths and the project-temp tempdirs are space-free, and Windows
/// `cmd /C` mangles embedded double quotes.
pub fn fake_child_emit_cmd(transform: &str, sink_path: &std::path::Path) -> String {
    let bin = fake_pee_child_path().to_string_lossy().replace('\\', "/");
    let sink = sink_path.to_string_lossy().replace('\\', "/");
    format!("{bin} --transform={transform} > {sink}")
}

/// Build a shell command string that just runs fake-pee-child with no
/// redirection (for exit-code tests where the child's stdout isn't needed).
pub fn fake_child_cmd(transform: &str) -> String {
    let bin = fake_pee_child_path().to_string_lossy().replace('\\', "/");
    format!("{bin} --transform={transform}")
}
