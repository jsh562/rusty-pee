//! Strict moreutils-compat mode entry point.
//!
//! Mirrors the rusty-sponge/rusty-vipe `strict.rs` pattern: bypasses clap
//! entirely (clap can't produce byte-equal moreutils errors), runs a
//! hand-rolled argv scan, emits moreutils-style stderr per FR-013 + FR-018.
//!
//! ## STF-003 option A
//!
//! For any unknown flag, we emit ONLY the first unknown-flag error and exit
//! non-zero. moreutils' Perl `Getopt::Long` iterates per-character; we
//! accept that as documented divergence (Strict-mode "moreutils-style"
//! rather than "moreutils-byte-equal" — see FR-013 note).
//!
//! ## Recognized inputs (Strict mode)
//!
//! | input                       | behavior                                        |
//! |-----------------------------|-------------------------------------------------|
//! | `--`                        | end-of-options; rest are positional commands    |
//! | `--strict` / `--no-strict`  | consumed by mode resolution upstream; ignored   |
//! | `--capture`                 | rejected per FR-013/FR-018 (Rusty extension)    |
//! | `--help` / `--version`      | rejected per FR-013                             |
//! | `completions`               | rejected per FR-013                             |
//! | other `-x` / `--foo`        | first-error formatter (STF-003 option A)        |
//! | positionals                 | command strings, spawned via platform shell     |

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use crate::{aggregate, fanout, spawner};

/// The first unknown flag encountered by the Strict-mode parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownFlag {
    /// Single-character short flag, e.g. `-x` → `Short('x')`.
    Short(char),
    /// Long flag with `--` prefix stripped, e.g. `--foo` → `Long("foo")`.
    Long(String),
}

/// First-error-only formatter for unknown flags per FR-013.
pub fn format_unknown_flag(flag: &UnknownFlag) -> String {
    match flag {
        UnknownFlag::Short(c) => format!("rusty-pee: invalid option -- '{c}'"),
        UnknownFlag::Long(name) => format!("rusty-pee: unknown option -- '{name}'"),
    }
}

/// Moreutils byte-equal spawn-failure formatter per FR-009 + HINT-004.
///
/// Note the "Can not" two-word spelling — that's the literal moreutils byte
/// sequence; do NOT change to "Cannot".
pub fn format_spawn_failure(cmd: &str) -> String {
    format!("pee: Can not open pipe to '{cmd}'")
}

/// Strict-mode entry point. Bypasses clap entirely.
pub fn run(argv: &[OsString]) -> ExitCode {
    let parsed = match parse_argv(argv) {
        Ok(p) => p,
        Err(unk) => {
            let msg = format_unknown_flag(&unk);
            let _ = writeln!(std::io::stderr().lock(), "{msg}");
            return ExitCode::from(2);
        }
    };

    // FR-006: zero commands → drain stdin + exit 0 (matches Default mode).
    if parsed.commands.is_empty() {
        use std::io::Read;
        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        let mut buf = vec![0u8; crate::BUFSIZ];
        loop {
            match handle.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => {}
                Err(_) => return ExitCode::from(1),
            }
        }
        return ExitCode::SUCCESS;
    }

    // Spawn every command via the platform shell. On failure, emit the
    // moreutils byte-equal error and exit 1 (FR-009 Strict branch).
    let mut children = Vec::with_capacity(parsed.commands.len());
    for cmd in &parsed.commands {
        match spawner::spawn_one(cmd) {
            Ok(c) => children.push(c),
            Err(_) => {
                let _ = writeln!(std::io::stderr().lock(), "{}", format_spawn_failure(cmd));
                // Best-effort: kill any already-spawned children.
                for mut c in children.into_iter() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                return ExitCode::from(1);
            }
        }
    }

    // Fan-out + wait.
    let stdin = std::io::stdin();
    let statuses = match fanout::run(stdin.lock(), children) {
        Ok(s) => s,
        Err(_) => return ExitCode::from(1),
    };

    // FR-008: bitwise OR aggregation.
    let codes: Vec<i32> = statuses.iter().map(|s| s.code().unwrap_or(1)).collect();
    let aggregated = aggregate::strict_or(&codes);
    let byte = if (0..=255).contains(&aggregated) {
        aggregated as u8
    } else {
        1u8
    };
    ExitCode::from(byte)
}

/// Result of scanning the Strict-mode argv.
#[derive(Debug, Default)]
struct StrictArgs {
    commands: Vec<String>,
}

/// Parse argv per Strict-mode rules. On the FIRST unknown flag, return
/// `Err(UnknownFlag)` — the caller emits one error line and exits non-zero.
///
/// Per FR-013, the following are rejected as unknown flags/subcommands:
/// - `--help`, `--version`, `--capture`
/// - `completions` (subcommand-style positional in first position)
fn parse_argv(argv: &[OsString]) -> Result<StrictArgs, UnknownFlag> {
    let mut out = StrictArgs::default();
    let mut iter = argv.iter().skip(1);
    let mut first_positional = true;

    while let Some(arg) = iter.next() {
        let s = arg.to_string_lossy();

        // Upstream mode-resolution flags — already consumed.
        if s == "--strict" || s == "--no-strict" {
            continue;
        }

        // End-of-options sentinel: rest are commands.
        if s == "--" {
            for rest in iter.by_ref() {
                out.commands.push(rest.to_string_lossy().into_owned());
            }
            break;
        }

        // `completions` as first positional → rejected per FR-013.
        if first_positional && s == "completions" {
            return Err(UnknownFlag::Long(String::from("completions")));
        }

        // Long flags.
        if let Some(rest) = s.strip_prefix("--") {
            // `--capture`, `--help`, `--version` — all rejected.
            let flag_name = rest.split('=').next().unwrap_or(rest).to_string();
            return Err(UnknownFlag::Long(flag_name));
        }

        // Short flags (`-x`, `-xyz`).
        if let Some(rest) = s.strip_prefix('-') {
            if !rest.is_empty() {
                let first = rest.chars().next().expect("non-empty after strip_prefix");
                return Err(UnknownFlag::Short(first));
            }
        }

        // Positional: command string.
        first_positional = false;
        out.commands.push(s.into_owned());
    }

    Ok(out)
}

/// Pre-clap scan for `--strict` / `--no-strict` flags. Last occurrence wins.
pub fn pre_scan_strict_flag(argv: &[OsString]) -> Option<bool> {
    let mut chosen: Option<bool> = None;
    for arg in argv.iter().skip(1) {
        let s = arg.to_string_lossy();
        if s == "--strict" {
            chosen = Some(true);
        } else if s == "--no-strict" {
            chosen = Some(false);
        } else if s == "--" {
            break;
        }
    }
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(|s| OsString::from(*s)).collect()
    }

    #[test]
    fn parse_no_flags_yields_no_commands() {
        let r = parse_argv(&argv(&["pee"])).unwrap();
        assert!(r.commands.is_empty());
    }

    #[test]
    fn parse_two_commands() {
        let r = parse_argv(&argv(&["pee", "wc -l", "grep foo"])).unwrap();
        assert_eq!(r.commands, vec!["wc -l", "grep foo"]);
    }

    #[test]
    fn parse_rejects_help() {
        let err = parse_argv(&argv(&["pee", "--help"])).unwrap_err();
        assert_eq!(err, UnknownFlag::Long(String::from("help")));
    }

    #[test]
    fn parse_rejects_version() {
        let err = parse_argv(&argv(&["pee", "--version"])).unwrap_err();
        assert_eq!(err, UnknownFlag::Long(String::from("version")));
    }

    #[test]
    fn parse_rejects_capture() {
        let err = parse_argv(&argv(&["pee", "--capture"])).unwrap_err();
        assert_eq!(err, UnknownFlag::Long(String::from("capture")));
    }

    #[test]
    fn parse_rejects_completions_subcommand() {
        let err = parse_argv(&argv(&["pee", "completions", "bash"])).unwrap_err();
        assert_eq!(err, UnknownFlag::Long(String::from("completions")));
    }

    #[test]
    fn parse_completions_after_other_positional_treated_as_command() {
        // Once a non-completions positional has been seen, "completions"
        // becomes just another command string.
        let r = parse_argv(&argv(&["pee", "wc -l", "completions"])).unwrap();
        assert_eq!(r.commands, vec!["wc -l", "completions"]);
    }

    #[test]
    fn parse_rejects_unknown_long_flag() {
        let err = parse_argv(&argv(&["pee", "--foo"])).unwrap_err();
        assert_eq!(err, UnknownFlag::Long(String::from("foo")));
    }

    #[test]
    fn parse_rejects_unknown_short_flag() {
        let err = parse_argv(&argv(&["pee", "-x"])).unwrap_err();
        assert_eq!(err, UnknownFlag::Short('x'));
    }

    #[test]
    fn parse_first_unknown_wins() {
        let err = parse_argv(&argv(&["pee", "-x", "--foo"])).unwrap_err();
        assert_eq!(err, UnknownFlag::Short('x'));
        let err = parse_argv(&argv(&["pee", "--foo", "-x"])).unwrap_err();
        assert_eq!(err, UnknownFlag::Long(String::from("foo")));
    }

    #[test]
    fn parse_double_dash_treats_rest_as_commands() {
        let r = parse_argv(&argv(&["pee", "--", "--help", "-x"])).unwrap();
        assert_eq!(
            r.commands,
            vec!["--help", "-x"],
            "after `--` everything is a command"
        );
    }

    #[test]
    fn pre_scan_detects_strict() {
        assert_eq!(
            pre_scan_strict_flag(&argv(&["rusty-pee", "--strict"])),
            Some(true)
        );
    }

    #[test]
    fn pre_scan_stops_at_double_dash() {
        assert_eq!(
            pre_scan_strict_flag(&argv(&["rusty-pee", "--", "--strict"])),
            None
        );
    }

    #[test]
    fn format_unknown_short_matches_spec() {
        assert_eq!(
            format_unknown_flag(&UnknownFlag::Short('x')),
            "rusty-pee: invalid option -- 'x'"
        );
    }

    #[test]
    fn format_unknown_long_matches_spec() {
        assert_eq!(
            format_unknown_flag(&UnknownFlag::Long(String::from("foo"))),
            "rusty-pee: unknown option -- 'foo'"
        );
    }

    #[test]
    fn format_spawn_failure_uses_two_word_can_not() {
        // HINT-004: moreutils uses "Can not" (two words), not "Cannot".
        assert_eq!(
            format_spawn_failure("nonexistent"),
            "pee: Can not open pipe to 'nonexistent'"
        );
    }
}
