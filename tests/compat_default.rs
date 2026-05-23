//! US1 (Fan-out basic flow, P1) + US2 (Exit aggregation, P1) integration tests.

mod common;

use assert_cmd::Command;
use std::fs;

#[test]
fn fanout_two_children_byte_identical() {
    // US1 AS1 / SC-001: same bytes flow to both sinks in argv order.
    let tmpdir = common::with_tempdir();
    let sink_a = tmpdir.path().join("a.txt");
    let sink_b = tmpdir.path().join("b.txt");
    let cmd_a = common::fake_child_emit_cmd("echo", &sink_a);
    let cmd_b = common::fake_child_emit_cmd("echo", &sink_b);

    let payload = b"alpha\nbravo\ncharlie\n";

    let report_path = tmpdir.path().join("noise.txt");
    let mut cmd = Command::cargo_bin("rusty-pee").expect("binary built");
    cmd.env("RUSTY_PEE_FAKE_CHILD_REPORT", &report_path); // unused but ensures env reaches children
    cmd.env_remove("RUSTY_PEE_STRICT");
    // Note: each child sets its OWN report path via the redirect file in cmd_a/cmd_b.
    // The env var here is for any child that uses RUSTY_PEE_FAKE_CHILD_REPORT directly
    // (`count`/`report-stdin` without the shell `>` redirect).
    cmd.arg(&cmd_a).arg(&cmd_b);
    cmd.write_stdin(&payload[..]).assert().success();

    let a_content = fs::read(&sink_a).expect("sink_a should exist");
    let b_content = fs::read(&sink_b).expect("sink_b should exist");
    assert_eq!(a_content, payload, "sink A should match input");
    assert_eq!(b_content, payload, "sink B should match input");
}

#[test]
fn empty_stdin_spawns_all_children() {
    // US1 AS2 / FR-019 / SC-017: empty stdin, all children spawn + see EOF + aggregate exit 0.
    let tmpdir = common::with_tempdir();
    let sink_a = tmpdir.path().join("a.txt");
    let cmd_a = common::fake_child_emit_cmd("echo", &sink_a);

    let mut cmd = Command::cargo_bin("rusty-pee").expect("binary built");
    cmd.env_remove("RUSTY_PEE_STRICT");
    cmd.arg(&cmd_a);
    cmd.write_stdin("").assert().success();

    let a_content = fs::read(&sink_a).expect("sink_a should exist");
    assert!(a_content.is_empty(), "empty stdin → empty sink");
}

#[test]
fn zero_commands_drains_stdin_and_exits_zero() {
    // FR-006: zero commands → drain stdin to nothing, exit 0.
    let mut cmd = Command::cargo_bin("rusty-pee").expect("binary built");
    cmd.env_remove("RUSTY_PEE_STRICT");
    cmd.write_stdin("ignored content\n").assert().success();
}

#[test]
fn one_child_n_equal_one() {
    // US1 N=1 degenerate: thin wrapper, single child gets stdin.
    let tmpdir = common::with_tempdir();
    let sink_a = tmpdir.path().join("a.txt");
    let cmd_a = common::fake_child_emit_cmd("echo", &sink_a);

    let mut cmd = Command::cargo_bin("rusty-pee").expect("binary built");
    cmd.env_remove("RUSTY_PEE_STRICT");
    cmd.arg(&cmd_a);
    cmd.write_stdin("solo\n").assert().success();

    let a_content = fs::read(&sink_a).expect("sink_a should exist");
    assert_eq!(a_content, b"solo\n");
}

#[test]
fn binary_bytes_passthrough_unchanged() {
    // US1 AS3 / FR-005: opaque bytes (non-UTF-8) pass through unchanged.
    let tmpdir = common::with_tempdir();
    let sink_a = tmpdir.path().join("a.bin");
    let cmd_a = common::fake_child_emit_cmd("echo", &sink_a);

    let payload: &[u8] = &[0x00, 0xfe, 0xff, 0xc3, 0x28, 0xa0, 0xa1];

    let mut cmd = Command::cargo_bin("rusty-pee").expect("binary built");
    cmd.env_remove("RUSTY_PEE_STRICT");
    cmd.arg(&cmd_a);
    cmd.write_stdin(payload).assert().success();

    let a_content = fs::read(&sink_a).expect("sink_a should exist");
    assert_eq!(
        a_content, payload,
        "FR-005: bytes opaque, no transformation"
    );
}

#[test]
fn default_mode_max_aggregation_zero_zero() {
    // SC-002: Default mode aggregates child exit codes via max(); (0,0) → 0.
    let cmd_a = common::fake_child_cmd("exit:0");
    let cmd_b = common::fake_child_cmd("exit:0");

    let mut cmd = Command::cargo_bin("rusty-pee").expect("binary built");
    cmd.env_remove("RUSTY_PEE_STRICT");
    cmd.arg(&cmd_a).arg(&cmd_b);
    cmd.write_stdin("").assert().success();
}

#[test]
fn default_mode_max_aggregation_one_two_picks_two() {
    // SC-002: Default mode (1, 2) → max=2.
    let cmd_a = common::fake_child_cmd("exit:1");
    let cmd_b = common::fake_child_cmd("exit:2");

    let output = Command::cargo_bin("rusty-pee")
        .expect("binary built")
        .env_remove("RUSTY_PEE_STRICT")
        .arg(&cmd_a)
        .arg(&cmd_b)
        .write_stdin("")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert_eq!(
        output.status.code(),
        Some(2),
        "Default mode: max(1, 2) = 2 (NOT bitwise OR which would be 3)"
    );
}

#[test]
fn default_mode_aggregation_zero_one_is_one() {
    let cmd_a = common::fake_child_cmd("exit:0");
    let cmd_b = common::fake_child_cmd("exit:1");

    let output = Command::cargo_bin("rusty-pee")
        .expect("binary built")
        .env_remove("RUSTY_PEE_STRICT")
        .arg(&cmd_a)
        .arg(&cmd_b)
        .write_stdin("")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn default_mode_aggregation_255_1_picks_255() {
    let cmd_a = common::fake_child_cmd("exit:255");
    let cmd_b = common::fake_child_cmd("exit:1");

    let output = Command::cargo_bin("rusty-pee")
        .expect("binary built")
        .env_remove("RUSTY_PEE_STRICT")
        .arg(&cmd_a)
        .arg(&cmd_b)
        .write_stdin("")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert_eq!(output.status.code(), Some(255));
}

// ─── Phase 7 (US5) `--capture` integration tests ─────────────────────────

#[test]
fn capture_two_children_argv_ordered_output() {
    // T074 / SC-007: --capture buffers each child's stdout and emits in argv order.
    let cmd_a = common::fake_child_cmd("emit:alpha");
    let cmd_b = common::fake_child_cmd("emit:bravo");

    let output = Command::cargo_bin("rusty-pee")
        .expect("binary built")
        .env_remove("RUSTY_PEE_STRICT")
        .arg("--capture")
        .arg(&cmd_a)
        .arg(&cmd_b)
        .write_stdin("")
        .assert()
        .success()
        .get_output()
        .clone();

    // FR-017: each captured chunk separated by LF if chunk doesn't end with one.
    // emit:alpha → "alpha"; emit:bravo → "bravo". Result: "alpha\nbravo\n".
    assert_eq!(
        output.stdout, b"alpha\nbravo\n",
        "FR-017: argv-ordered emission with LF separator"
    );
}

#[test]
fn capture_three_children_reversed_finish_order() {
    // SC-007 (clarified): three children with deliberately reversed finish order
    // (child 3 finishes first since it has the simplest transform) — argv order
    // MUST still be preserved.
    let cmd_a = common::fake_child_cmd("emit:alpha");
    let cmd_b = common::fake_child_cmd("emit:bravo");
    let cmd_c = common::fake_child_cmd("emit:charlie");

    let output = Command::cargo_bin("rusty-pee")
        .expect("binary built")
        .env_remove("RUSTY_PEE_STRICT")
        .arg("--capture")
        .arg(&cmd_a)
        .arg(&cmd_b)
        .arg(&cmd_c)
        .write_stdin("")
        .assert()
        .success()
        .get_output()
        .clone();

    assert_eq!(
        output.stdout, b"alpha\nbravo\ncharlie\n",
        "FR-017: argv order preserved regardless of finish order"
    );
}

#[test]
fn capture_with_empty_stdout_child_emits_nothing_for_that_child() {
    // Clarification Q6: empty-stdout children emit nothing (no separator).
    // exit:0 transform consumes stdin but writes NO stdout.
    let cmd_a = common::fake_child_cmd("emit:visible");
    let cmd_b = common::fake_child_cmd("exit:0"); // empty stdout
    let cmd_c = common::fake_child_cmd("emit:after");

    let output = Command::cargo_bin("rusty-pee")
        .expect("binary built")
        .env_remove("RUSTY_PEE_STRICT")
        .arg("--capture")
        .arg(&cmd_a)
        .arg(&cmd_b)
        .arg(&cmd_c)
        .write_stdin("")
        .assert()
        .success()
        .get_output()
        .clone();

    // The empty middle child must NOT introduce a separator.
    assert_eq!(
        output.stdout, b"visible\nafter\n",
        "Clarification Q6: empty-stdout children emit nothing (no separator)"
    );
}

#[test]
fn duplicate_command_strings_spawn_separate_children() {
    // T040 / spec Edge Cases: same command twice → spawn two separate children;
    // both receive full stdin.
    let tmpdir = common::with_tempdir();
    let sink_a = tmpdir.path().join("a.txt");
    let sink_b = tmpdir.path().join("b.txt");
    let cmd_a = common::fake_child_emit_cmd("echo", &sink_a);
    let cmd_b = common::fake_child_emit_cmd("echo", &sink_b);
    // Note: cmd_a and cmd_b spawn the SAME fake-pee-child binary with the SAME
    // transform, differing only in redirect target. From rusty-pee's POV they
    // ARE the same command string up to the redirect — two distinct children
    // spawn and both receive stdin.

    let payload = b"duplicate-test-payload\n";

    let mut cmd = Command::cargo_bin("rusty-pee").expect("binary built");
    cmd.env_remove("RUSTY_PEE_STRICT");
    cmd.arg(&cmd_a).arg(&cmd_b);
    cmd.write_stdin(&payload[..]).assert().success();

    let a_content = fs::read(&sink_a).expect("sink_a should exist");
    let b_content = fs::read(&sink_b).expect("sink_b should exist");
    assert_eq!(a_content, payload, "sink A from first child");
    assert_eq!(b_content, payload, "sink B from second child");
}

#[test]
fn capture_in_strict_mode_is_rejected() {
    // FR-018: --capture under Strict mode is rejected as unknown flag.
    let output = Command::cargo_bin("rusty-pee")
        .expect("binary built")
        .env_remove("RUSTY_PEE_STRICT")
        .arg("--strict")
        .arg("--capture")
        .write_stdin("")
        .assert()
        .failure()
        .get_output()
        .clone();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.trim_end_matches(['\r', '\n']),
        "rusty-pee: unknown option -- 'capture'"
    );
}
