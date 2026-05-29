# Changelog

All notable changes to `rusty-pee` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-05-26

### Added (additive only — no v0.1.x behavior changed)

- Portfolio-wide [Cargo Features Convention](https://github.com/jsh562/rustylib/blob/main/specs/adrs/0006-cargo-features-convention-for-portfolio-ports.md)
  layout per ADR-0006 + `project-instructions.md` §Cargo Feature Surface. rusty-pee applies the minimum convention as a **single-capability port** per spec 00011 §Scope Edge Cases.
- New umbrella features (all `["cli"]` composition for this single-cap port):
  - `full` — kitchen-sink umbrella per FR-002
  - `pee-classic` — required `<port>-classic` umbrella per FR-004; moreutils `pee` drop-in replacement
  - `pee-minimal` — preset bundle per FR-007; explicit minimal-CLI semantic alias
- `default` now aliases to `full` instead of directly to `cli`. Resolved dependency set is identical (`full = ["cli"]`); no observable change for any consumer.
- See [`docs/feature-layout.md`](docs/feature-layout.md) for the zero-leaf rationale.

All v0.1.x feature names are preserved verbatim with identical compositions. `cli = ["dep:clap", "dep:clap_complete", "dep:anyhow", "dep:signal-hook"]` is unchanged. `pee-alias = ["cli"]` is unchanged. `bench = ["dep:criterion"]` is unchanged. `dev-helpers = []` is unchanged. Library consumers using `default-features = false` get the same CLI-stripped build. Users running `cargo install rusty-pee --features pee-alias` continue to work unchanged. Users running `cargo bench --features bench` and `cargo test --features dev-helpers` continue to work unchanged.

### Notes

- See the new `## Cargo Features` section in `README.md` for the
  feature matrix, preset bundles, keep-list workaround, and convention
  authority citations.
- Reference: [ADR-0006](https://github.com/jsh562/rustylib/blob/main/specs/adrs/0006-cargo-features-convention-for-portfolio-ports.md)
  (why this layout) + [`project-instructions.md` §Cargo Feature Surface](https://github.com/jsh562/rustylib/blob/main/project-instructions.md)
  (what the rules are).
- CI matrix expanded per spec 00011 FR-010..FR-014: now includes
  `test-default` (kitchen sink + cross-compile), `test-no-default`
  (bare library + dep-tree audit per SC-001), `test-pee-classic`,
  `test-pee-minimal` (preset bundles per SC-003), `test-keeplist`
  (keep-list workaround per SC-004), and `lint-convention` (vendored
  `tools/feature-lint/run.sh` invocation per FR-052). Tier 4
  (`check-leaf-<leaf>`) is intentionally empty — zero leaves carved
  per docs/feature-layout.md.
- The lint script is **vendored** into `tools/feature-lint/` (synced
  from the umbrella `jsh562/rustylib` repo) so per-port CI workflows
  do not depend on cross-repo `actions/checkout` of the private
  umbrella. Sync precedent set by rusty-figlet v0.2.0 (E011 Phase 2
  iteration 6), rusty-ts v0.2.0 (E011 Phase 3), rusty-sponge v0.2.0
  (E011 Phase 4), and rusty-vipe v0.2.0 (E011 Phase 5).

## [0.1.0] - 2026-05-23

### Added

- CLI binary `rusty-pee`: fan stdin out to N concurrent shell-spawned children (Rust port of moreutils `pee`).
- Default-mode CLI with moreutils semantics plus quality-of-life additions (`--help`, `--version`, `--capture` for argv-ordered child stdout buffering, `completions <shell>` subcommand).
- Strict moreutils-compatibility mode via `--strict`, `RUSTY_PEE_STRICT=1`, or invocation as `pee` (via the `pee-alias` cargo feature). Mirrors moreutils' bitwise-OR exit aggregation and `pee: Can not open pipe to '<cmd>'` error format. Unknown flags emit ONLY the first error per the portfolio STF-003 option A pattern.
- Pipeline-safety contract: when a child closes its stdin mid-stream (`BrokenPipe`), the parent drops it from the live-set and continues feeding survivors. Parent peak memory is `O(BUFSIZ × N)`, not `O(input size)`.
- Exit-code aggregation: Default mode uses `max(child_codes)` (intuitive "worst child wins"); Strict mode uses bitwise OR over `WEXITSTATUS` (byte-equal moreutils 0.69).
- Signal-driven cleanup: SIGINT/SIGTERM/SIGHUP (Unix) and `CTRL_C_EVENT`/`CTRL_BREAK_EVENT`/`CTRL_CLOSE_EVENT` (Windows). All live children receive SIGTERM/`TerminateProcess`, a fixed 1-second grace period, then SIGKILL/forcible termination if still alive.
- Optional `pee` binary alias gated behind the `pee-alias` cargo feature; auto-activates Strict mode via argv[0] auto-detect.
- `completions <shell>` subcommand emitting shell-completion scripts for bash, zsh, fish, and PowerShell.
- Public Rust library API: `PeeBuilder` (with `#[must_use]` chain methods, validation at `build()` time) → `Pee::run<R: Read>(reader)`. Library fans out to N `Box<dyn Write + Send>` sinks instead of subprocesses, so embedders aren't forced into the process-spawn model.
- Library-without-binary build: `default-features = false` drops `clap`, `clap_complete`, `anyhow`, and `signal-hook` from the dependency closure.
- Cross-platform binary distribution: Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64 via `cargo-binstall` metadata pointing at GitHub Release archives.

### Testing posture

Behavioral integration tests use a deterministic `fake-pee-child` helper binary (gated behind the `dev-helpers` Cargo feature; NOT installed by `cargo install`) instead of driving real shell utilities like `grep`/`wc`/`sort` in CI. The fake child performs known transformations (`count`, `echo`, `exit:<N>`, `sleep-per-byte:<ms>`, `emit:<text>`, `report-stdin`) so behavioral assertions are reproducible across CI runners.

### MSRV

Minimum supported Rust version: **1.85**.

Upward deviation from the portfolio's "current stable minus two" rule, forced by Rust edition 2024 (which requires 1.85+).

### Known limitations at v0.1.0

- **Exit-code aggregation diverges from moreutils in Default mode**: rusty-pee uses `max(child_codes)`; moreutils uses bitwise OR. Strict mode preserves byte-exact moreutils behavior.
- **Uncatchable signals (SIGKILL on Unix, hard process termination on Windows)**: live children are not terminated by the parent's signal handler — that's by design (the parent process group inherits the same signal). Documented in `docs/COMPATIBILITY.md`.
- **Hardcoded platform shell** (`/bin/sh` on Unix, `cmd.exe` on Windows): no `--shell <path>` flag at v0.1.0. Forward-review candidate.
- **Negation flags** (`--no-ignore-sigpipe`, `--no-ignore-write-errors`) are not exposed; the defaults are frozen on. Forward-review candidate.
- **SIGTERM→SIGKILL grace period** is hardcoded to 1 second; not configurable in v0.1.0.

### Verified

- Tests passing on Rust 1.85 (MSRV) and current stable.
- Clippy strict (`-D warnings`) clean.
- rustfmt clean.
- `cargo audit` clean.
- Library API consumable with `default-features = false`.

### Compatibility statement

A full Compatibility Matrix lives at [`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md).

[Unreleased]: https://github.com/jsh562/rusty-pee/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/jsh562/rusty-pee/releases/tag/v0.2.0
[0.1.0]: https://github.com/jsh562/rusty-pee/releases/tag/v0.1.0
