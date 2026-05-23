//! # rusty-pee
//!
//! A Rust port of the moreutils `pee` utility: fan a single stdin stream out
//! to N concurrent shell-spawned children, aggregate their exit codes, and
//! surface failures cleanly.
//!
//! ## Quick start
//!
//! ```no_run
//! use rusty_pee::{PeeBuilder, CompatibilityMode};
//! use std::io::Cursor;
//!
//! // Construct sinks that the builder owns (so they satisfy `'static`).
//! let sink_a: Vec<u8> = Vec::new();
//! let sink_b: Vec<u8> = Vec::new();
//!
//! let mut pee = PeeBuilder::new()
//!     .sink(Box::new(sink_a))
//!     .sink(Box::new(sink_b))
//!     .compat(CompatibilityMode::Default)
//!     .build()?;
//!
//! let input = Cursor::new(b"alpha\nbravo\ncharlie\n".to_vec());
//! pee.run(input)?;
//! # Ok::<(), rusty_pee::Error>(())
//! ```
//!
//! ## Stability (lockstep SemVer)
//!
//! Library and binary share a single crate version. Within `0.x`, minor
//! version bumps may introduce breaking changes per standard Cargo
//! semantics. Every public enum and struct is `#[non_exhaustive]` so
//! variant additions are not breaking changes once `1.0` lands.
//!
//! ## Pipeline-safety contract
//!
//! When a sink errors mid-chunk during [`Pee::run`], every other live sink
//! receives the **complete current chunk** in registration order before the
//! failing sink is dropped from the live-set (mirrors the CLI's
//! `--ignore-write-errors` default — frozen-on per FR-003).

pub mod error;

pub use error::Error;

/// Whether to apply Default-mode ergonomic extensions or Strict moreutils parity.
///
/// # Examples
///
/// ```
/// use rusty_pee::CompatibilityMode;
///
/// assert_eq!(CompatibilityMode::default(), CompatibilityMode::Default);
/// // Strict mode rejects `--capture`, `--help`, `--version`, and completions.
/// let _ = CompatibilityMode::Strict;
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompatibilityMode {
    /// Default mode: `--help`, `--version`, `--capture`, `completions` subcommand all honored.
    /// Exit aggregation uses `max(child_codes)`.
    #[default]
    Default,
    /// Strict mode: byte-equal moreutils stderr for documented inputs;
    /// exit aggregation uses bitwise OR (matches moreutils `close_pipes()`).
    Strict,
}

/// Default fan-out chunk size (64 KiB per AD-015). Not user-configurable in v0.1.0.
pub const BUFSIZ: usize = 64 * 1024;

/// Runtime engine for one pee invocation. Constructed via [`PeeBuilder`].
///
/// # Examples
///
/// ```no_run
/// # use rusty_pee::{PeeBuilder, CompatibilityMode};
/// # use std::io::Cursor;
/// let mut pee = PeeBuilder::new().build()?;
/// let input = Cursor::new(b"hello".to_vec());
/// pee.run(input)?;
/// # Ok::<(), rusty_pee::Error>(())
/// ```
#[non_exhaustive]
pub struct Pee {
    sinks: Vec<Box<dyn std::io::Write + Send>>,
    compat: CompatibilityMode,
    capture: bool,
    ignore_write_errors: bool,
}

impl std::fmt::Debug for Pee {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pee")
            .field("sinks_count", &self.sinks.len())
            .field("compat", &self.compat)
            .field("capture", &self.capture)
            .field("ignore_write_errors", &self.ignore_write_errors)
            .finish()
    }
}

/// Builder for [`Pee`]. All chain methods are `#[must_use]`.
///
/// # Examples
///
/// ```
/// use rusty_pee::{PeeBuilder, CompatibilityMode};
///
/// let pee = PeeBuilder::new()
///     .compat(CompatibilityMode::Default)
///     .build()
///     .expect("default builder always succeeds");
/// # let _ = pee;
/// ```
#[non_exhaustive]
pub struct PeeBuilder {
    sinks: Vec<Box<dyn std::io::Write + Send>>,
    compat: CompatibilityMode,
    capture: bool,
    ignore_write_errors: bool,
}

impl std::fmt::Debug for PeeBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeeBuilder")
            .field("sinks_count", &self.sinks.len())
            .field("compat", &self.compat)
            .field("capture", &self.capture)
            .field("ignore_write_errors", &self.ignore_write_errors)
            .finish()
    }
}

impl Default for PeeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PeeBuilder {
    /// Construct a new builder defaulting to zero sinks, `Default` compat,
    /// `capture(false)`, and `ignore_write_errors(true)`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sinks: Vec::new(),
            compat: CompatibilityMode::Default,
            capture: false,
            ignore_write_errors: true,
        }
    }

    /// Register a sink. Sinks receive the same bytes in registration order.
    #[must_use]
    pub fn sink(mut self, sink: Box<dyn std::io::Write + Send>) -> Self {
        self.sinks.push(sink);
        self
    }

    /// Set the compatibility mode.
    #[must_use]
    pub fn compat(mut self, compat: CompatibilityMode) -> Self {
        self.compat = compat;
        self
    }

    /// Set capture-mode flag. Library no-op (sinks already capture by definition);
    /// retained for parity with the CLI flag and validated against Strict mode at build time.
    #[must_use]
    pub fn capture(mut self, capture: bool) -> Self {
        self.capture = capture;
        self
    }

    /// Set the ignore-write-errors policy. Default: `true` (matches moreutils
    /// `--ignore-write-errors` default-on behavior). When `false`, the first
    /// sink write error halts the fan-out and is surfaced as
    /// `Error::SinkWriteFailed`.
    #[must_use]
    pub fn ignore_write_errors(mut self, ignore: bool) -> Self {
        self.ignore_write_errors = ignore;
        self
    }

    /// Validate and build a [`Pee`].
    pub fn build(self) -> Result<Pee, Error> {
        // FR-022: Strict + capture(true) is rejected at build time.
        if self.compat == CompatibilityMode::Strict && self.capture {
            return Err(Error::CompatibilityViolation(
                "--capture not honored in Strict mode",
            ));
        }
        Ok(Pee {
            sinks: self.sinks,
            compat: self.compat,
            capture: self.capture,
            ignore_write_errors: self.ignore_write_errors,
        })
    }
}

impl Pee {
    /// Read from `reader` and fan out each chunk to every registered sink in
    /// registration order. On sink `BrokenPipe`, the sink is dropped from the
    /// live-set and the fan-out continues to the remaining sinks (FR-003 + FR-036).
    ///
    /// **Writer-untouched invariant** (FR-022): every other live sink receives
    /// the COMPLETE current chunk in registration order before the failing
    /// sink is dropped.
    ///
    /// **Zero-sink case** (FR-006): drains the reader to completion and returns Ok.
    pub fn run<R: std::io::Read>(&mut self, mut reader: R) -> Result<(), Error> {
        let _ = (self.compat, self.capture); // hold while we implement

        if self.sinks.is_empty() {
            // Drain reader to nothing.
            let mut buf = [0u8; BUFSIZ];
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
            }
            return Ok(());
        }

        let mut buf = vec![0u8; BUFSIZ];
        // Live-set: indices into self.sinks that haven't errored yet.
        let mut live: Vec<usize> = (0..self.sinks.len()).collect();

        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            // FR-036: write the full chunk to each live sink in registration
            // order before dropping any that error.
            let mut to_drop: Vec<usize> = Vec::new();
            for &idx in &live {
                use std::io::Write;
                match self.sinks[idx].write_all(&buf[..n]) {
                    Ok(()) => {}
                    Err(e)
                        if e.kind() == std::io::ErrorKind::BrokenPipe
                            && self.ignore_write_errors =>
                    {
                        to_drop.push(idx);
                    }
                    Err(e) if self.ignore_write_errors => {
                        // Non-BrokenPipe write errors with ignore-on: still drop the sink silently.
                        to_drop.push(idx);
                        let _ = e;
                    }
                    Err(e) => {
                        return Err(Error::SinkWriteFailed {
                            sink_index: idx,
                            source: e,
                        });
                    }
                }
            }
            live.retain(|i| !to_drop.contains(i));
            if live.is_empty() {
                // All sinks closed; drain reader to nothing.
                loop {
                    let n = reader.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                }
                break;
            }
        }

        // Flush surviving sinks.
        for &idx in &live {
            use std::io::Write;
            let _ = self.sinks[idx].flush();
        }
        Ok(())
    }
}

// CLI / mode / signal / strict / spawner / fanout / aggregate / capture
// internals are gated behind `cli` because they pull clap, signal-hook, and
// other binary-only deps. Library callers configure compat mode via the builder.
#[cfg(feature = "cli")]
pub mod aggregate;
#[cfg(feature = "cli")]
pub mod capture;
#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "cli")]
pub mod fanout;
#[cfg(feature = "cli")]
pub mod mode;
#[cfg(feature = "cli")]
pub mod spawner;
#[cfg(feature = "cli")]
pub mod strict;

/// Binary entry-point helper used by both `src/main.rs` and `src/bin/pee.rs`.
///
/// Per FR-007/FR-008/AD-002/AD-003: Default-mode exit aggregation uses
/// `max(child_codes)`; Strict mode uses bitwise OR.
#[cfg(feature = "cli")]
pub fn run() -> std::process::ExitCode {
    use clap::Parser;
    use std::ffi::OsString;
    use std::process::ExitCode;

    let raw_argv: Vec<OsString> = std::env::args_os().collect();

    // Pre-clap detection of `--strict` / `--no-strict` + env + argv[0] for
    // Strict-mode dispatch. Strict mode bypasses clap entirely.
    let pre_strict = strict::pre_scan_strict_flag(&raw_argv);
    let env_strict = std::env::var_os("RUSTY_PEE_STRICT");
    let argv0 = raw_argv.first().cloned();
    let resolved_mode = mode::resolve(pre_strict, env_strict.as_deref(), argv0.as_deref());

    if resolved_mode == CompatibilityMode::Strict {
        return strict::run(&raw_argv);
    }

    let cli_args = match cli::Cli::try_parse() {
        Ok(args) => args,
        Err(e) => {
            e.print().ok();
            return match e.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    ExitCode::SUCCESS
                }
                _ => ExitCode::from(2),
            };
        }
    };

    // Completions subcommand (Default only).
    if let Some(cli::Subcommand::Completions { shell }) = cli_args.command {
        use clap::CommandFactory;
        let mut cmd = cli::Cli::command();
        let name = cmd.get_name().to_string();
        clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
        return ExitCode::SUCCESS;
    }

    // FR-006: zero commands → drain stdin and exit 0.
    if cli_args.commands.is_empty() {
        use std::io::Read;
        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        let mut buf = [0u8; BUFSIZ];
        loop {
            match handle.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => {}
                Err(e) => {
                    eprintln!("rusty-pee: stdin read error: {e}");
                    return ExitCode::from(1);
                }
            }
        }
        return ExitCode::SUCCESS;
    }

    // Capture mode (FR-017): replace each child's stdout with `Stdio::piped()`,
    // drain in parallel via worker threads, emit captured chunks in argv order.
    let statuses = if cli_args.capture {
        let children = match capture::spawn_all_piped(&cli_args.commands) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("rusty-pee: failed to spawn child: {e}");
                return ExitCode::from(127);
            }
        };
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        match capture::run_with_capture(stdin.lock(), children, &mut out) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("rusty-pee: capture error: {e}");
                return ExitCode::from(1);
            }
        }
    } else {
        // Default fan-out (FR-002, FR-003, FR-004) — children inherit parent stdio.
        let mut children = Vec::with_capacity(cli_args.commands.len());
        for cmd in &cli_args.commands {
            match spawner::spawn_one(cmd) {
                Ok(c) => children.push(c),
                Err(e) => {
                    eprintln!("rusty-pee: failed to spawn child '{cmd}': {e}");
                    for mut c in children.into_iter() {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                    return ExitCode::from(127);
                }
            }
        }
        let stdin = std::io::stdin();
        match fanout::run(stdin.lock(), children) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("rusty-pee: fan-out error: {e}");
                return ExitCode::from(1);
            }
        }
    };

    // Aggregate exit codes (FR-007). Default mode reaches here only; Strict
    // mode was already dispatched to strict::run() above.
    let codes: Vec<i32> = statuses.iter().map(|s| s.code().unwrap_or(1)).collect();
    let aggregated = aggregate::default_max(&codes);

    let byte = if (0..=255).contains(&aggregated) {
        aggregated as u8
    } else {
        1u8
    };
    ExitCode::from(byte)
}
