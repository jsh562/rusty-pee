//! `pee` binary alias entry point (gated behind the `pee-alias` Cargo feature).
//!
//! Shares the same body as [`rusty_pee::run`]; argv[0] auto-detect inside
//! `run()` routes invocations as `pee` into Strict mode per FR-012.

fn main() -> std::process::ExitCode {
    rusty_pee::run()
}
