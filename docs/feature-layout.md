# rusty-pee — v0.2.0 Feature Layout

**Status**: implementation draft for the v0.2.0 Cargo features convention
backfill (spec 00011, Phase 6 — rusty-pee).

**Authority**:
- `specs/adrs/0006-cargo-features-convention-for-portfolio-ports.md` (why)
- `project-instructions.md` §Cargo Feature Surface (what)
- This document — the per-port carving + WHY for each leaf, per HINT-003
  + HINT-009 of spec 00011.

**Reference port**: rusty-figlet v0.2.0 — see `../../rusty-figlet/docs/feature-layout.md`
(FROZEN reference port) for the format anchor. rusty-pee conforms to the
same shape with the minimum-convention surface dictated by its
single-capability scope. The companion sibling ports rusty-sponge v0.2.0
(see `../../rusty-sponge/docs/feature-layout.md`, E011 Phase 4) and
rusty-vipe v0.2.0 (see `../../rusty-vipe/docs/feature-layout.md`, E011
Phase 5) established the zero-leaf precedent for single-capability
moreutils ports.

**Iteration model**: v0.2.0 is a **purely additive** SemVer-minor release.
Every v0.1.x feature name and composition is preserved verbatim; new
umbrellas (`full`, `pee-classic`, `pee-minimal`) are layered on top
without renaming or narrowing the existing `cli` / `default` / `pee-alias`
/ `bench` / `dev-helpers` features. Library and binary API surfaces are
unchanged.

## Single-capability port — spec 00011 §Scope Edge Cases

rusty-pee is a **single-capability port**: it has exactly one documented
capability — fan a single stdin stream out to N concurrent shell-spawned
children, aggregate their exit codes, and surface failures cleanly (a
Rust port of moreutils `pee`). Spec 00011 §Scope Edge Cases dictates that
single-capability ports apply the **minimum convention**:

> ports with only one capability adopt the minimum convention:
> `full = ["cli"]` and `<port>-classic = ["cli"]` are the required
> umbrellas; ZERO leaves carved beyond those required umbrellas.

This document records the carving exercise and the explicit decision
to NOT split orthogonal sub-capabilities into leaves — every additional
behavior of `rusty-pee` (Default-mode `max(child_codes)` exit aggregation,
Strict moreutils bitwise-OR exit aggregation, `--capture` argv-ordered
child stdout buffering, pipeline-safety / mid-chunk `BrokenPipe`
handling, cross-platform signal cleanup, `completions` subcommand) is
part of the single core capability surface and removing any of them
would break either the documented public CLI / library contract or the
pipeline-safety guarantee that is the entire raison d'être of the tool.

## Source-tree walk

`src/` modules (v0.1.0, post-Phase-1 baseline):

| Module                | Always-on? | CLI-only deps                                       | Notes                                                                       |
|-----------------------|-----------:|-----------------------------------------------------|-----------------------------------------------------------------------------|
| `error.rs`            | yes        | (thiserror — always-on)                             | `Error` enum; library + binary need it.                                     |
| `fanout.rs`           | yes        | none                                                | Core fan-out write loop; mid-chunk BrokenPipe semantics.                    |
| `aggregate.rs`        | yes        | none                                                | Default-mode `max(child_codes)` + Strict-mode bitwise-OR aggregation.       |
| `capture.rs`          | yes        | none                                                | `--capture` argv-ordered child stdout buffering.                            |
| `spawner.rs`          | yes        | none                                                | `/bin/sh -c` (Unix) / `cmd /C` (Windows) shell child spawning.              |
| `mode.rs`             | yes        | none                                                | CompatibilityMode resolver (`--strict` > env > argv[0]).                    |
| `lib.rs`              | yes        | none                                                | Public API (`PeeBuilder`, `Pee`, `CompatibilityMode`).                      |
| `cli.rs`              | no — `cli` | clap                                                | clap-derive `Cli` struct + `Subcommand::Completions`.                       |
| `strict.rs`           | no — `cli` | (clap_complete + clap pulled by `cli`)              | Hand-rolled Strict-mode argv pre-scanner + byte-equal moreutils dispatcher. |
| `main.rs`             | no — `cli` | clap, clap_complete, anyhow, signal-hook            | Binary entry; gated by `required-features = ["cli"]`.                       |
| `bin/pee.rs`          | no — `pee-alias` | (inherits `cli`)                              | `pee` alias binary; gated by `required-features = ["pee-alias"]`.           |

## Leaf-carving criteria (HINT-009)

A capability becomes a leaf when ALL of the following hold:

1. It is **self-containable** — gated cleanly via `#[cfg(feature = "<leaf>")]`
   at the module or top-level item boundary (HINT-004).
2. Either (a) it has a **sole optional dependency** that no other leaf needs
   (HINT-005), OR (b) it is a pure-cfg-gate of an internal module worth
   exposing as a knob.
3. Disabling it does NOT break any always-on library/CLI surface.

A capability does NOT become a leaf when:

- It is foundational (fan-out write loop, exit-code aggregation, shell
  child spawning, signal-driven cleanup) — disabling it would break
  the headline pipeline-safety guarantee or the cross-platform
  signal-handling contract.
- It is part of the single documented capability surface (Default mode,
  Strict mode, `--capture`, completions subcommand).
- It would create more than ~6 leaves (FR-007 + HINT-003 envelope).

## v0.2.0 Carved Leaves

**ZERO new leaves carved at v0.2.0**. Every capability inside rusty-pee
is either:

1. Foundational always-on library code (fan-out write loop with
   mid-chunk BrokenPipe semantics, exit-code aggregation, shell child
   spawning, capture buffering, mode resolution) — cannot be stripped
   without breaking the public surface or the pipeline-safety guarantee.
2. Already gated by the v0.1.x `cli` umbrella (clap-derived argument
   parsing, completions subcommand, signal handler install, Strict-mode
   pre-scanner).
3. Already gated by the v0.1.x `pee-alias` feature (the second `pee`
   binary entry).
4. A dev-tooling feature (`bench` → criterion benches; `dev-helpers` →
   `fake-pee-child` test bin) outside the convention's runtime-capability
   purview.

### Leaves intentionally NOT carved

The following candidate leaves were considered + rejected:

- **`signal`**: signal handler install + cleanup-on-exit dispatch lives
  alongside `dep:signal-hook` (Unix) and `windows-sys` (Windows,
  target-conditional always-on). It is part of the pipeline-safety
  contract — without it, a Ctrl-C mid-fan-out could leave half-fed
  children running with broken pipes. Stripping this would silently
  break the headline promise. Rejected per HINT-009 criterion 3.
- **`capture`**: The `--capture` Default-mode flag buffers child stdouts
  in argv order. It is ~40 lines in `src/capture.rs` + a Default-mode
  CLI flag in `cli.rs`. Splitting it into its own leaf would orphan no
  external dep (it's pure-Rust over `std::io::Read`) and require
  separate compat-flag plumbing in `mode.rs`. No carving signal — it's
  part of the documented Default-mode surface.
- **`completions`**: Could be carved as `["dep:clap_complete"]`, but
  per spec 00011 §Scope Edge Cases minimum-convention single-capability
  ports declare ZERO new leaves. `clap_complete` is bundled into the
  v0.1.x `cli` umbrella verbatim. Carving it would either rename `cli`
  (breaking SemVer additivity) or duplicate the surface.
- **`strict-compat`**: rusty-pee's Strict mode dispatches inline in
  `lib.rs::run()` via `mode::resolve` and the hand-rolled getopt mirror
  in `src/strict.rs`. Both are gated by the `cli` umbrella in v0.1.x
  (since they consume `clap` + `clap_complete`). Carving out a separate
  `strict-compat` leaf would require splitting `strict.rs` away from
  `cli.rs`, which is more refactoring than the additive v0.2.0 release
  allows. The capability survives untouched inside the existing `cli`
  composition. (Note: rusty-figlet carves `strict-compat` because its
  Strict-mode parser is dep-free hand-rolled getopt; rusty-pee's Strict
  dispatcher consumes `clap` for the `--strict` flag itself, so it
  cannot stand alone without `cli`.)
- **`pee-alias`**: This v0.1.x feature ships a second binary named
  `pee`. It IS retained verbatim per the v0.2.0 SemVer additive
  contract — but it is NOT one of the 2 required preset bundles per
  FR-007 (those are `pee-classic` and `pee-minimal` below).
  Documented separately as an installation-time convenience knob.
- **`bench`**: The v0.1.x `bench` feature is a tooling / benchmark
  scaffold (criterion benches under `benches/throughput.rs`), not a
  runtime capability leaf. It remains a dev-tooling feature outside
  the convention's purview (the vendored `tools/feature-lint/lint.sh`
  allowlist skips `bench` from leaf-CI-matrix and phantom-leaf checks)
  and is retained verbatim from v0.1.0.
- **`dev-helpers`**: The v0.1.x `dev-helpers` feature gates the
  `fake-pee-child` `[[bin]]` used by integration tests. Like `bench`,
  this is dev-tooling outside the convention's purview. Retained
  verbatim; the feature-lint allowlist treats it as a dev-tooling
  name (`dev-helpers` is an explicit allowlist entry).

## Preset bundles (FR-007 — 2 required for single-capability ports)

Per spec 00011 §Scope Edge Cases + FR-007, even single-capability ports
declare 2 preset bundles to give the keep-list workaround documentation
something concrete to point at.

### `pee-classic` (REQUIRED — bare port, 1:1 with moreutils `pee`)

```toml
pee-classic = ["cli"]
```

- Includes `cli` so the binary exists.
- Single-capability port; the `cli` umbrella IS the bare-port surface.
- Use case: `cargo install rusty-pee --no-default-features --features pee-classic`
  for a moreutils-`pee` drop-in replacement (Strict mode is invoked via
  the `--strict` flag, `RUSTY_PEE_STRICT` env var, or `pee-alias`
  binary name — none of these require additional features).

### `pee-minimal`

```toml
pee-minimal = ["cli"]
```

- Same composition as `pee-classic` (single-capability port has no
  smaller subset to carve).
- Use case: explicit "minimal CLI install" alias for users who prefer
  the `<port>-minimal` naming convention seen across other Rusty ports
  (figlet-minimal, pwgen-minimal, ts-minimal, sponge-minimal,
  vipe-minimal).
  Documented as an intentional semantic alias rather than a distinct
  composition.

### `pee-alias` (v0.1.x feature retained, NOT a convention preset)

`pee-alias = ["cli"]` from v0.1.0 ships an additional `pee` binary
alongside `rusty-pee`. It is retained verbatim per the v0.2.0 SemVer
additive contract — but it is NOT one of the 2 required preset bundles
per FR-007 (those are `pee-classic` and `pee-minimal` above).
`pee-alias` is documented separately as an installation-time
convenience knob, not a capability subset.

### `bench` (v0.1.x dev-tooling feature retained, NOT a convention preset)

`bench = ["dep:criterion"]` from v0.1.0 enables `benches/throughput.rs`.
It is dev-tooling, not a runtime capability — outside the convention's
purview per the vendored feature-lint allowlist.

### `dev-helpers` (v0.1.x dev-tooling feature retained, NOT a convention preset)

`dev-helpers = []` from v0.1.0 gates the `fake-pee-child` `[[bin]]` used
by integration tests. It is dev-tooling — never installed by
`cargo install`, used only by `cargo test --features dev-helpers`.
Outside the convention's purview; matches the `test-util` / `dev-helpers`
family of dev-tooling names already in the vendored feature-lint
allowlist.

## Cross-port glossary candidates

No leaves carved → no cross-port glossary contributions from rusty-pee
in this iteration. If a future minor release adds an orthogonal
capability (e.g., a `metrics` leaf for per-child timing telemetry), the
leaf would be a candidate for promotion to `docs/feature-vocabulary.md`
per FR-053.

## CI matrix shape (FR-010..FR-014)

Per plan §Per-Port v0.2.0 CI Matrix, scaled to a zero-leaf port:

- **Tier 1 — `test-default`**: full DDR-003 cross-compile matrix
  (5 targets). Post-v0.2.0 `default = ["full"]` and `full = ["cli"]`,
  so the kitchen-sink test resolves to the same set as v0.1.0
  `default = ["cli"]` — no regression in coverage.
- **Tier 2 — `test-no-default`**: Linux x86_64 only. `cargo test
  --no-default-features --lib` + dep-tree audit (SC-001 evidence).
- **Tier 3 — `test-<bundle>`**: one job per preset bundle. Linux only.
  - `test-pee-classic`
  - `test-pee-minimal`
- **Tier 4 — `check-leaf-<leaf>`**: SKIPPED. Zero leaves → no
  per-leaf compile-check jobs. A placeholder comment in `ci.yml`
  documents why this tier is empty. The `bench` + `dev-helpers`
  features are in the vendored feature-lint allowlist (dev-tooling)
  and do not require a Tier-4 entry.
- **Tier 5 — `lint-convention`**: single Linux job invoking the
  vendored `tools/feature-lint/run.sh` script.

Per FR-014, bundle/lint jobs are constrained to Linux x86_64.

## Vendored feature-lint

Per spec 00011 §Phase 2 iteration 6 precedent (rusty-figlet vendored
the lint script because the umbrella `jsh562/rustylib` is private and
cross-repo `actions/checkout` cannot reach it), rusty-pee vendors
`tools/feature-lint/{lint.sh,run.sh,README.md}` from the umbrella into
the port repo. The vendored copy is byte-equal to the umbrella source
of truth as of the freeze commit (post the dev-tooling-allowlist +
benches/tests-search + additive-CHANGELOG-support fixes from rusty-ts
v0.2.0 / E011 Phase 3 iteration 2 and the path-sanitization fixes from
rusty-sponge v0.2.0 / E011 Phase 4).

## Why no new leaves — explicit rationale

Spec 00011 §Scope Edge Cases anticipates this case verbatim:

> Some ports have only one orthogonal capability. Those ports adopt the
> minimum convention: `full = ["cli"]` and `<port>-classic = ["cli"]`
> as aliases; the convention SHAPE is consistent across the portfolio
> even when the per-port leaf carving yields zero leaves.

rusty-pee deliberately chooses the zero-leaf path because:

1. The pipeline-safety guarantee (every live sink receives every byte
   the parent reads, in argv order, until natural close-or-EOF) is the
   entire reason this tool exists. Carving any of its supporting
   machinery (signal handlers, fan-out loop, exit aggregation, shell
   spawner) into an opt-out leaf would silently change the FR-006
   contract for users who turned that leaf off.
2. The cost of carving a speculative leaf (cfg-gate scaffolding,
   per-leaf CI matrix entry, README/CHANGELOG row, glossary candidacy)
   outweighs the value when no orthogonal capability exists to gate.
3. The portfolio-wide convention shape (umbrella set, README "Cargo
   Features" section, lint compliance) is preserved verbatim — a
   downstream library consumer reading the README for rusty-pee
   gets the same one-glance feature matrix UX as one reading
   rusty-figlet, rusty-ts, rusty-sponge, or rusty-vipe.
4. v0.2.0 is **purely additive**. Every v0.1.x feature is preserved
   verbatim; no SemVer break. Future minor releases can add leaves
   without breaking the v0.2.0 contract: a hypothetical `metrics`
   v0.3.0 feature would slot in as `metrics = ["dep:tracing"]`
   alongside the existing umbrellas with zero migration cost.
