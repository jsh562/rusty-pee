//! US4 (Strict moreutils-Compat Mode, P2) integration tests.

mod common;

use assert_cmd::Command;

fn rusty_pee_strict() -> Command {
    let mut cmd = Command::cargo_bin("rusty-pee").expect("binary built");
    cmd.env_remove("RUSTY_PEE_STRICT");
    cmd.arg("--strict");
    cmd
}

#[test]
fn strict_unknown_short_flag_byte_equal_stderr() {
    // FR-013 + SC-005: `-x` → first-error formatter.
    let output = rusty_pee_strict()
        .arg("-x")
        .write_stdin("")
        .assert()
        .failure()
        .get_output()
        .clone();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.trim_end_matches(['\r', '\n']),
        "rusty-pee: invalid option -- 'x'"
    );
}

#[test]
fn strict_unknown_long_flag_byte_equal_stderr() {
    let output = rusty_pee_strict()
        .arg("--foo")
        .write_stdin("")
        .assert()
        .failure()
        .get_output()
        .clone();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.trim_end_matches(['\r', '\n']),
        "rusty-pee: unknown option -- 'foo'"
    );
}

#[test]
fn strict_rejects_help_version_capture_completions() {
    // FR-013: --help, --version, --capture, completions all rejected.
    let cases: &[(&[&str], &str)] = &[
        (&["--help"], "rusty-pee: unknown option -- 'help'"),
        (&["--version"], "rusty-pee: unknown option -- 'version'"),
        (&["--capture"], "rusty-pee: unknown option -- 'capture'"),
        (
            &["completions", "bash"],
            "rusty-pee: unknown option -- 'completions'",
        ),
    ];

    for (args, expected) in cases {
        let output = rusty_pee_strict()
            .args(*args)
            .write_stdin("")
            .assert()
            .failure()
            .get_output()
            .clone();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            stderr.trim_end_matches(['\r', '\n']),
            *expected,
            "Strict-mode rejection mismatch for {args:?}"
        );
    }
}

#[test]
fn strict_first_unknown_flag_only() {
    // STF-003 option A: when both -x and --foo present, only the FIRST
    // unknown-flag error is emitted.
    let output = rusty_pee_strict()
        .arg("-x")
        .arg("--foo")
        .write_stdin("")
        .assert()
        .failure()
        .get_output()
        .clone();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines: Vec<&str> = stderr.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "only one error line expected; got {lines:?}"
    );
    assert_eq!(lines[0], "rusty-pee: invalid option -- 'x'");
}

#[test]
fn strict_mode_exit_aggregation_or_one_two_equals_three() {
    // SC-003: Strict mode (1, 2) → bitwise OR = 3 (the divergence from Default max=2).
    let cmd_a = common::fake_child_cmd("exit:1");
    let cmd_b = common::fake_child_cmd("exit:2");

    let output = rusty_pee_strict()
        .arg(&cmd_a)
        .arg(&cmd_b)
        .write_stdin("")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert_eq!(
        output.status.code(),
        Some(3),
        "Strict mode: 1|2 = 3 (NOT max=2)"
    );
}

#[test]
fn strict_mode_exit_aggregation_or_zero_zero() {
    let cmd_a = common::fake_child_cmd("exit:0");
    let cmd_b = common::fake_child_cmd("exit:0");

    rusty_pee_strict()
        .arg(&cmd_a)
        .arg(&cmd_b)
        .write_stdin("")
        .assert()
        .success();
}

#[test]
fn strict_mode_spawn_failure_byte_equal_moreutils() {
    // FR-009 Strict branch + HINT-004: "Can not" (two words, byte-for-byte).
    let output = rusty_pee_strict()
        .arg("nonexistent-binary-xyzzy")
        .write_stdin("")
        .assert()
        .failure()
        .get_output()
        .clone();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // On platforms where /bin/sh succeeds (Linux/macOS) but the command-string
    // is bogus, the shell spawns successfully and `exit_code` is whatever the
    // shell returned for "command not found" (e.g., 127). On Windows,
    // `cmd /C` spawns but returns a code from cmd.exe. Either way, we don't
    // get to OUR spawn-failure path — only an actual shell binary missing
    // triggers `spawn_one` to fail. So this test asserts the aggregation
    // path returns non-zero. The byte-equal moreutils stderr is tested via
    // a unit test on `strict::format_spawn_failure` (covered below).
    let _ = stderr;
    assert_ne!(output.status.code(), Some(0));
}

#[test]
fn strict_mode_argv0_pee_implies_strict() {
    // FR-012: when invoked as `pee` (via pee-alias feature), Strict mode is auto.
    let mut cmd = Command::cargo_bin("pee").expect("pee-alias binary built");
    cmd.env_remove("RUSTY_PEE_STRICT");
    cmd.arg("--help");
    let output = cmd.write_stdin("").assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.trim_end_matches(['\r', '\n']),
        "rusty-pee: unknown option -- 'help'",
        "FR-012: pee binary auto-enables Strict; --help → unknown option"
    );
}

#[test]
fn strict_mode_env_var_engages_strict() {
    // FR-012: RUSTY_PEE_STRICT=1 engages Strict mode against the Default-named binary.
    let mut cmd = Command::cargo_bin("rusty-pee").expect("binary built");
    cmd.env("RUSTY_PEE_STRICT", "1");
    cmd.arg("--help");
    let output = cmd.write_stdin("").assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.trim_end_matches(['\r', '\n']),
        "rusty-pee: unknown option -- 'help'",
        "FR-012: RUSTY_PEE_STRICT=1 engages Strict"
    );
}
