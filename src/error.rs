//! Library-level error type for `rusty_pee`.
//!
//! Uses `thiserror` to produce typed errors per AD-012; the binary boundary
//! wraps these in `anyhow::Result` for human-readable diagnostics.

/// Errors returned by the `rusty_pee` library API.
///
/// Per FR-028 + FR-029, the enum carries the enum-level `#[non_exhaustive]`
/// marker (protects future variant additions) AND each non-unit variant
/// individually carries `#[non_exhaustive]` (protects future field additions
/// inside variants).
///
/// # Examples
///
/// ```
/// use rusty_pee::Error;
///
/// let e = Error::CompatibilityViolation("--capture not honored in Strict mode");
/// assert_eq!(e.to_string(), "compatibility violation: --capture not honored in Strict mode");
/// ```
#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A Default-mode-only setting was passed to Strict mode (or vice versa).
    /// Returned from `PeeBuilder::build()` only — before any IO occurs, so
    /// no sinks are touched.
    #[error("compatibility violation: {0}")]
    CompatibilityViolation(&'static str),

    /// The builder was configured into an impossible state (e.g., explicit
    /// settings whose combination is logically invalid).
    /// Returned from `PeeBuilder::build()` only.
    #[error("invalid builder configuration: {0}")]
    InvalidBuilderConfiguration(&'static str),

    /// A sink's `write` failed mid-stream. Surfaced only when the builder
    /// was configured with `ignore_write_errors(false)`. Carries the
    /// registration index of the failing sink and the underlying io error.
    ///
    /// **Observable state**: surviving sinks have received the complete
    /// current chunk in registration order (per FR-036). The failing sink
    /// is dropped from the live-set.
    #[error("sink {sink_index} write failed: {source}")]
    #[non_exhaustive]
    SinkWriteFailed {
        sink_index: usize,
        #[source]
        source: std::io::Error,
    },

    /// An underlying IO error from the reader or process layer.
    ///
    /// **Observable state**: may leave sinks in a partially-written state
    /// for the current chunk.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
