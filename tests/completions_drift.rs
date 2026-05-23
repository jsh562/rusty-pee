//! US7 (Shell Completions, P3) drift tests.
//!
//! Regenerates each shell's completion script from the current `Cli` schema
//! and asserts byte equality with the committed file in `completions/`.

use clap::CommandFactory;
use clap_complete::Shell;
use rusty_pee::cli::Cli;
use std::fs;
use std::path::PathBuf;

fn generate(shell: Shell) -> Vec<u8> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    let mut out: Vec<u8> = Vec::new();
    clap_complete::generate(shell, &mut cmd, name, &mut out);
    normalize_line_endings(&out)
}

fn normalize_line_endings(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().copied().filter(|b| *b != b'\r').collect()
}

fn committed(filename: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("completions")
        .join(filename);
    let bytes =
        fs::read(&path).unwrap_or_else(|e| panic!("committed completion missing at {path:?}: {e}"));
    normalize_line_endings(&bytes)
}

#[test]
fn drift_bash() {
    assert_eq!(
        committed("rusty-pee.bash"),
        generate(Shell::Bash),
        "bash completion drift — regenerate with `cargo run -- completions bash > completions/rusty-pee.bash`"
    );
}

#[test]
fn drift_zsh() {
    assert_eq!(
        committed("_rusty-pee"),
        generate(Shell::Zsh),
        "zsh completion drift — regenerate with `cargo run -- completions zsh > completions/_rusty-pee`"
    );
}

#[test]
fn drift_fish() {
    assert_eq!(
        committed("rusty-pee.fish"),
        generate(Shell::Fish),
        "fish completion drift — regenerate"
    );
}

#[test]
fn drift_powershell() {
    assert_eq!(
        committed("rusty-pee.ps1"),
        generate(Shell::PowerShell),
        "powershell completion drift — regenerate"
    );
}

#[test]
fn strict_mode_rejects_completions_subcommand() {
    // T098 / US7 AS2 / FR-013: Strict mode rejects `completions` subcommand.
    let mut cmd = assert_cmd::Command::cargo_bin("rusty-pee").expect("binary built");
    cmd.env_remove("RUSTY_PEE_STRICT");
    cmd.arg("--strict").arg("completions").arg("bash");
    let output = cmd.write_stdin("").assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown option -- 'completions'"),
        "Strict mode must reject completions; got: {stderr:?}"
    );
}
