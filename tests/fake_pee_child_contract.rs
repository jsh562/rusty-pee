//! T112 / FR-026 / SC-029: assert the fake-pee-child source documents every
//! supported transform. Reads the source file via `include_str!`.

#[test]
fn fake_pee_child_documents_every_transform() {
    const SOURCE: &str = include_str!("bin/fake_pee_child.rs");
    for transform in &[
        "count",
        "echo",
        "exit:",
        "sleep-per-byte:",
        "emit:",
        "report-stdin",
        "noop",
    ] {
        assert!(
            SOURCE.contains(transform),
            "FR-026: fake-pee-child source must mention transform {transform:?}"
        );
    }
}
