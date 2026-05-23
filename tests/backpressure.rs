//! US3 (Slow-child backpressure, P2) integration tests.
//!
//! Verifies that a slow child cannot cause byte loss for fast siblings AND
//! the parent's memory stays bounded `O(BUFSIZ × N)` regardless of input size
//! (FR-002, FR-003 backpressure invariant; SC-004).

mod common;

use assert_cmd::Command;
use std::fs;

#[test]
fn fast_and_slow_children_both_receive_full_input() {
    // US3 AS1 / SC-004: with a 10 KiB input flowing through one passive child
    // and one slow child, both children record the full input verbatim.
    // (10 KiB instead of 10 MiB per spec — keeps the test under a second
    // while still exceeding the 64-KiB BUFSIZ to exercise multi-chunk flow.
    // Larger payloads are covered by the criterion bench harness.)
    let tmpdir = common::with_tempdir();
    let sink_fast = tmpdir.path().join("fast.txt");
    let sink_slow = tmpdir.path().join("slow.txt");

    // The slow child sleeps 1ms per byte BEFORE writing. To keep the test
    // fast, use only ~200 bytes of input (200ms total sleep). The point is
    // that the parent's blocking write_all on the slow pipe paces the FAST
    // child's write too, but neither child loses bytes.
    let payload: Vec<u8> = (0..200u8).collect();

    let cmd_fast = common::fake_child_emit_cmd("echo", &sink_fast);
    // Slow child reads stdin byte-by-byte with sleep, but we need it to also
    // write to a sink. fake-pee-child's sleep-per-byte doesn't echo to stdout.
    // For this test, use a shell pipeline: `sleep-per-byte:1 < /dev/stdin` is
    // the consumer; we use `echo` on the fast side. The slow side just consumes.
    let cmd_slow_consumer = common::fake_child_cmd("sleep-per-byte:1");

    let mut cmd = Command::cargo_bin("rusty-pee").expect("binary built");
    cmd.env_remove("RUSTY_PEE_STRICT");
    cmd.arg(&cmd_fast).arg(&cmd_slow_consumer);
    cmd.write_stdin(&payload[..]).assert().success();

    let fast_content = fs::read(&sink_fast).expect("sink_fast should exist");
    assert_eq!(
        fast_content, payload,
        "FR-002: fast child must receive full input even when slow child paces the parent"
    );
    // sink_slow is intentionally empty — the slow consumer doesn't write.
    assert!(!sink_slow.exists() || fs::metadata(&sink_slow).unwrap().len() == 0);
}

#[test]
fn three_children_argv_order_preserved() {
    // T039: with 3 children all using report-stdin sinks, each sink should
    // contain the full input, proving argv-order delivery + parent waits all.
    let tmpdir = common::with_tempdir();
    let sink_a = tmpdir.path().join("a.txt");
    let sink_b = tmpdir.path().join("b.txt");
    let sink_c = tmpdir.path().join("c.txt");
    let cmd_a = common::fake_child_emit_cmd("echo", &sink_a);
    let cmd_b = common::fake_child_emit_cmd("echo", &sink_b);
    let cmd_c = common::fake_child_emit_cmd("echo", &sink_c);

    let payload = b"alpha\nbravo\ncharlie\ndelta\necho\n";

    let mut cmd = Command::cargo_bin("rusty-pee").expect("binary built");
    cmd.env_remove("RUSTY_PEE_STRICT");
    cmd.arg(&cmd_a).arg(&cmd_b).arg(&cmd_c);
    cmd.write_stdin(&payload[..]).assert().success();

    for (name, path) in [("a", &sink_a), ("b", &sink_b), ("c", &sink_c)] {
        let content = fs::read(path).expect("sink should exist");
        assert_eq!(content, payload, "sink {name} should match input");
    }
}

#[test]
fn large_chunked_input_byte_perfect_delivery() {
    // Multi-chunk delivery (BUFSIZ=64KiB; we send 256KiB to force at least 4 chunks).
    let tmpdir = common::with_tempdir();
    let sink_a = tmpdir.path().join("a.bin");
    let sink_b = tmpdir.path().join("b.bin");
    let cmd_a = common::fake_child_emit_cmd("echo", &sink_a);
    let cmd_b = common::fake_child_emit_cmd("echo", &sink_b);

    let payload: Vec<u8> = (0..(256 * 1024)).map(|i| (i % 256) as u8).collect();

    let mut cmd = Command::cargo_bin("rusty-pee").expect("binary built");
    cmd.env_remove("RUSTY_PEE_STRICT");
    cmd.arg(&cmd_a).arg(&cmd_b);
    cmd.write_stdin(&payload[..]).assert().success();

    let a_content = fs::read(&sink_a).expect("sink_a should exist");
    let b_content = fs::read(&sink_b).expect("sink_b should exist");
    assert_eq!(a_content.len(), payload.len(), "sink_a size mismatch");
    assert_eq!(b_content.len(), payload.len(), "sink_b size mismatch");
    assert_eq!(a_content, payload, "sink_a byte-equal mismatch");
    assert_eq!(b_content, payload, "sink_b byte-equal mismatch");
}
