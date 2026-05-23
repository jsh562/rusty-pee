# rusty-pee Design Notes

Architectural overview pointing back to the canonical spec/plan:

- [spec.md](../../rusty/specs/00004-pee-port/spec.md) — feature requirements
- [plan.md](../../rusty/specs/00004-pee-port/plan.md) — architecture decisions + requirement coverage map

## Upstream Dependency Status

E003 (reusable port-ci.yml workflow) does NOT exist in the umbrella repo at the time this port was scaffolded. Inline CI / release workflows are duplicated from `rusty-vipe/.github/workflows/{ci,release}.yml` as a pragmatic-path approach, tracked as tech debt for back-port when E003 ships.

## Component Map

See plan.md §Architecture (Mermaid C4) for the full diagram. Quick reference:

| Component | File | Purpose |
|---|---|---|
| CLI Frontend | `src/cli.rs` | clap derive `Cli` struct |
| Mode Resolver | `src/mode.rs` | `CompatibilityMode` precedence ladder |
| Strict Parser | `src/strict.rs` | Hand-rolled argv parser + moreutils-format errors |
| Spawner | `src/spawner.rs` | Per-command shell-wrapping spawn + `Stdio::piped()` stdin |
| Fan-Out Loop | `src/fanout.rs` | BUFSIZ-chunk write loop + live-set management |
| Capture Buffer | `src/capture.rs` | `--capture` mode: replace child stdout, argv-ordered emission |
| Exit Aggregator | `src/aggregate.rs` | Default `max()` / Strict bitwise OR |
| Signal Handler | `src/signal.rs` | Signal-driven child cleanup with 1s grace + SIGKILL |
| Library API | `src/builder.rs` | `Pee` + `PeeBuilder` (sink-based) |
| Completions | `src/completions.rs` | `completions <shell>` subcommand |

## fake-pee-child Contract

The deterministic helper binary `fake-pee-child` (gated behind the `dev-helpers` Cargo feature) supports these transforms per FR-026:

| Transform | Behavior |
|---|---|
| `count` | Write line count of stdin to `$RUSTY_PEE_FAKE_CHILD_REPORT`; exit 0 |
| `echo` | Echo stdin to stdout verbatim; exit 0 |
| `exit:<N>` | Consume stdin (don't echo), exit with code N |
| `sleep-per-byte:<ms>` | Read stdin one byte at a time, sleeping <ms> per byte (backpressure test) |
| `emit:<text>` | Write the given literal to stdout, ignoring stdin; exit 0 |
| `report-stdin` | Write stdin verbatim to `$RUSTY_PEE_FAKE_CHILD_REPORT`; exit 0 |

Tests find this binary via `env!("CARGO_BIN_EXE_fake-pee-child")` at test build time. It is NEVER installed by `cargo install rusty-pee` (AD-013).

## Test-Only Env Vars

| Env var | Purpose |
|---|---|
| `RUSTY_PEE_FAKE_CHILD_REPORT` | Side-channel report file path for `count`/`report-stdin` transforms |
| `RUSTY_PEE_STRICT` | Activates Strict mode (FR-012) |

## See Also

- README.md — user-facing surface
- docs/COMPATIBILITY.md — full moreutils-divergence matrix
- specs/00004-pee-port/{spec,plan,tasks,research,analysis-report}.md
