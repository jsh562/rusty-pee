//! US6 (Library API for Embedding, P2) integration tests.

use rusty_pee::{CompatibilityMode, Error, Pee, PeeBuilder};
use std::io::Cursor;

#[test]
fn builder_default_succeeds() {
    let _ = PeeBuilder::new().build().expect("default builder ok");
}

#[test]
fn builder_rejects_strict_plus_capture() {
    // FR-022 + SC-019: Strict + capture(true) → CompatibilityViolation at build time.
    let result = PeeBuilder::new()
        .compat(CompatibilityMode::Strict)
        .capture(true)
        .build();
    assert!(
        matches!(result, Err(Error::CompatibilityViolation(_))),
        "got {result:?}"
    );
}

#[test]
fn builder_strict_without_capture_succeeds() {
    let _ = PeeBuilder::new()
        .compat(CompatibilityMode::Strict)
        .capture(false)
        .build()
        .expect("Strict + capture(false) should succeed");
}

#[test]
fn pee_run_zero_sinks_drains_reader() {
    // FR-006 library-side analog: zero sinks → drain reader, return Ok.
    let mut pee = PeeBuilder::new().build().expect("builder ok");
    let input = Cursor::new(b"some bytes".to_vec());
    pee.run(input).expect("zero-sinks run should succeed");
}

#[test]
fn pee_run_one_sink_passthrough() {
    // T087 / FR-015: single sink receives full reader bytes.
    let sink = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let sink_clone = std::sync::Arc::clone(&sink);
    let wrapper = SinkAdapter { inner: sink_clone };
    let mut pee = PeeBuilder::new()
        .sink(Box::new(wrapper))
        .build()
        .expect("builder ok");
    let input = Cursor::new(b"hello world\n".to_vec());
    pee.run(input).expect("run should succeed");
    let captured = sink.lock().unwrap();
    assert_eq!(&*captured, b"hello world\n");
}

#[test]
fn pee_run_two_sinks_byte_identical() {
    // T087 / FR-015 / SC-009: both sinks receive identical full reader bytes.
    let sink_a = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let sink_b = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let mut pee = PeeBuilder::new()
        .sink(Box::new(SinkAdapter {
            inner: std::sync::Arc::clone(&sink_a),
        }))
        .sink(Box::new(SinkAdapter {
            inner: std::sync::Arc::clone(&sink_b),
        }))
        .build()
        .expect("builder ok");
    let payload: Vec<u8> = (0..1024u32).flat_map(|i| (i as u8).to_le_bytes()).collect();
    let input = Cursor::new(payload.clone());
    pee.run(input).expect("run should succeed");

    let captured_a = sink_a.lock().unwrap();
    let captured_b = sink_b.lock().unwrap();
    assert_eq!(*captured_a, payload, "sink A");
    assert_eq!(*captured_b, payload, "sink B");
}

#[test]
fn send_sync_compile_time_assertion() {
    // T090 / T091 / FR-031: thread-safety contracts.
    use static_assertions::assert_impl_all;
    // Pee holds Box<dyn Write + Send> sinks → Send but not necessarily Sync.
    assert_impl_all!(Pee: Send);
    // PeeBuilder also Send (holds sinks similarly).
    assert_impl_all!(PeeBuilder: Send);
    // CompatibilityMode + Error both Send + Sync.
    assert_impl_all!(CompatibilityMode: Send, Sync, Copy);
    assert_impl_all!(Error: Send, Sync);
}

#[test]
fn default_features_off_excludes_cli_deps() {
    // T092 / FR-030 / SC-008: cargo tree --no-default-features must not show
    // clap/clap_complete/anyhow/signal-hook.
    let output = std::process::Command::new("cargo")
        .args(["tree", "--no-default-features", "--prefix", "none"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo tree should run");
    assert!(output.status.success(), "cargo tree must succeed");
    let tree = String::from_utf8_lossy(&output.stdout);
    for forbidden in ["clap ", "clap_complete", "anyhow ", "signal-hook"] {
        assert!(
            !tree.contains(forbidden),
            "FR-030 / SC-008: no-default-features tree must not contain {forbidden:?}\n\
             full tree:\n{tree}"
        );
    }
}

/// Adapter that lets us hold a shared sink for assertion AFTER `Pee::run`
/// has moved its `Box<dyn Write + Send>`. Real consumers would use `Vec<u8>`
/// or `File` directly.
struct SinkAdapter {
    inner: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
}

impl std::io::Write for SinkAdapter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.inner.lock().unwrap();
        guard.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
