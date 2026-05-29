# rusty-pee

Fan stdin out to N concurrent shell-spawned children. Rust port of moreutils [`pee(1)`](https://joeyh.name/code/moreutils/).

[![crates.io](https://img.shields.io/crates/v/rusty-pee.svg)](https://crates.io/crates/rusty-pee)
[![docs.rs](https://docs.rs/rusty-pee/badge.svg)](https://docs.rs/rusty-pee)
[![CI](https://github.com/jsh562/rusty-pee/actions/workflows/ci.yml/badge.svg)](https://github.com/jsh562/rusty-pee/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue.svg)](#msrv)
[![license: MIT OR Apache-2.0](https://img.shields.io/crates/l/rusty-pee.svg)](#license)

Run one stream through several commands at once: `journalctl | rusty-pee 'grep selinux > a.log' 'wc -l > b.txt'` feeds every byte to each child. It aggregates their exit codes, drops `BrokenPipe` sinks cleanly, & holds memory at `O(BUFSIZ × N)` instead of buffering the whole stream. Default mode adds `--capture`, `--help`, `--version`, & a `completions` subcommand; Strict mode mirrors moreutils' bitwise-OR exit aggregation & stderr layout. Prebuilt binaries ship for five targets (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64).

Part of the [Rusty portfolio](https://jsh562.github.io/rusty-portfolio).

## Install

```sh
cargo install rusty-pee
# or, with prebuilt binaries:
cargo binstall rusty-pee
# or, download directly from GitHub Releases:
# https://github.com/jsh562/rusty-pee/releases
```

To also install a `pee` binary alias (argv[0] auto-detect routes into Strict mode):

```sh
cargo install rusty-pee --features pee-alias
```

## Usage

```sh
# Fan stdin out to multiple sinks that each consume the full stream
journalctl -p err | rusty-pee 'grep selinux > /tmp/sel.log' 'wc -l > /tmp/errcount.txt'

# Run two validators in parallel; aggregate their exit codes
producer | rusty-pee 'validator-A' 'validator-B'

# Capture mode: emit child stdouts in argv order after they exit
echo "alpha" | rusty-pee --capture 'cat' 'cat'

# Strict moreutils-compat mode (drop-in moreutils pee replacement)
producer | rusty-pee --strict 'validator-A' 'validator-B'
RUSTY_PEE_STRICT=1 producer | rusty-pee 'sink-A' 'sink-B'
producer | pee 'sink-A' 'sink-B'         # via pee-alias feature or argv[0] symlink

# Shell completions
rusty-pee completions bash                # > ~/.bash_completion.d/rusty-pee
rusty-pee completions zsh                 # > ~/.zfunc/_rusty-pee
rusty-pee completions fish                # > ~/.config/fish/completions/rusty-pee.fish
rusty-pee completions powershell
```

## Library API

The library fans out to N `Box<dyn Write + Send>` sinks instead of subprocesses. Use it when you want pee's broadcast pattern inside a long-running program without spawning a shell per consumer.

```rust,no_run
use rusty_pee::{PeeBuilder, CompatibilityMode};
use std::io::Cursor;

let input = Cursor::new(b"alpha\nbravo\ncharlie\n".to_vec());
let mut sink_a: Vec<u8> = Vec::new();
let mut sink_b: Vec<u8> = Vec::new();

let mut pee = PeeBuilder::new()
    .sink(Box::new(&mut sink_a))
    .sink(Box::new(&mut sink_b))
    .compat(CompatibilityMode::Default)
    .build()?;

pee.run(input)?;

assert_eq!(sink_a, b"alpha\nbravo\ncharlie\n");
assert_eq!(sink_b, b"alpha\nbravo\ncharlie\n");
# Ok::<(), rusty_pee::Error>(())
```

For library-only consumers without CLI deps see the [Cargo Features](#cargo-features) section.

## Cargo Features

`default` enables `full`, which (for this single-capability port) resolves to the `cli` umbrella. `pee-classic` reproduces v0.1.x bare-port behavior matching upstream moreutils `pee` 1:1. To strip the CLI surface use `default-features = false` or `--no-default-features` & add the features you want.

rusty-pee is a **single-capability port**: its one documented job is "fan a single stdin stream out to N concurrent shell-spawned children, aggregate their exit codes, & surface failures cleanly". No optional feature leaves are carved beyond the required umbrellas; see [`docs/feature-layout.md`](docs/feature-layout.md) for why.

### Feature matrix

| Feature | Description | Umbrella(s) |
|---|---|---|
| `cli` | All CLI-only dependencies (`clap`, `clap_complete`, `anyhow`, `signal-hook`) and the binary entry point, signal-handler install, mode resolver, and Strict-mode pre-scanner. Library consumers strip via `default-features = false`. | `full`, `pee-classic`, `pee-minimal`, `pee-alias` |
| `pee-alias` | Installs an additional `pee` binary alongside `rusty-pee`. Both share source; argv[0] auto-detect routes `pee` invocations into Strict mode. | (standalone, implies `cli`) |
| `bench` | Pulls `criterion` and enables `benches/throughput.rs`. Dev-tooling only; outside the convention's leaf surface. Name preserved verbatim from v0.1.x. | (standalone) |
| `dev-helpers` | Gates the `fake-pee-child` `[[bin]]` used by integration tests. Never installed by `cargo install`; enable with `cargo test --features dev-helpers`. Dev-tooling only. | (standalone) |

### Preset bundles

| Bundle | Composition | Use case |
|---|---|---|
| `pee-classic` | `cli` | Drop-in upstream moreutils `pee` replacement. Strict mode is invoked via `--strict`, `RUSTY_PEE_STRICT`, or `pee-alias` argv[0] auto-detect. |
| `pee-minimal` | `cli` | Explicit minimal-CLI alias for users who prefer the `<port>-minimal` naming convention seen across other portfolio ports. Identical composition to `pee-classic`. |

### Keep-list workaround (Cargo features are union-only)

Cargo features cannot subtract from `default`. To get "everything except a specific feature," disable defaults & enumerate the features you want:

```sh
cargo install rusty-pee --no-default-features --features "cli"
# → bare CLI with no pee-alias binary, no bench tooling.

cargo install rusty-pee --no-default-features --features "cli pee-alias"
# → CLI + the pee alias binary.
```

For the common cases the named [preset bundles](#preset-bundles) are usually sufficient.

### Library-only consumers

```toml
[dependencies]
rusty-pee = { version = "0.2", default-features = false }
```

This strips `clap`, `clap_complete`, `anyhow`, & `signal-hook`. The resulting build pulls only `thiserror` & the target-conditional always-on deps (`libc` on Unix, `windows-sys` on Windows; both required for cross-platform child-process control regardless of feature selection).

### Convention authority

This layout follows the portfolio-wide Cargo Features Convention. The "why" lives in [ADR-0006](https://github.com/jsh562/rustylib/blob/main/specs/adrs/0006-cargo-features-convention-for-portfolio-ports.md); the "what" lives in [`project-instructions.md` §Cargo Feature Surface](https://github.com/jsh562/rustylib/blob/main/project-instructions.md). Every Rusty port from v0.2 onward exposes the same umbrella set (`default` / `full` / `cli` / `<port>-classic`), per-port leaves named in kebab-case, & 2 to 4 preset bundles.

## Compatibility

`rusty-pee` has two modes:

- **Default mode.** clap-styled flag parser. `--capture`, `--help`, `--version`, & the `completions` subcommand are all available. Exit-code aggregation uses `max(child_codes)`.
- **Strict mode** (activated by `--strict`, `RUSTY_PEE_STRICT=1`, or invoking the binary as `pee`). Bitwise OR over `WEXITSTATUS` for exit aggregation. moreutils-format stderr. `--capture`, `--help`, `--version`, & `completions` MUST be rejected. Pinned upstream version: **moreutils 0.69**.

### Pipeline-safety guarantee

Every live sink receives every byte the parent reads, in argv (CLI) / registration (library) order. When a child closes its stdin mid-stream (`BrokenPipe`), the parent drops it from the live-set & keeps feeding survivors. Memory MUST stay bounded at `O(BUFSIZ × N)`, not `O(input size)`.

### Documented intentional divergences

1. **Exit-code aggregation.** Default mode uses `max(child_codes)`; Strict mode preserves moreutils' bitwise OR over `WEXITSTATUS`. Example: children exiting 1 & 2 produce Default exit=2, Strict exit=3.
2. **Hardcoded platform shell.** `/bin/sh -c` (Unix) / `cmd /C` (Windows). No `--shell <path>` flag in v0.1.0.
3. **Unknown-flag stderr in Strict.** Emits only the first unknown-flag error (`rusty-pee: invalid option -- 'X'`). moreutils' POSIX `Getopt::Long` iterates per-character; we don't replicate that for undocumented inputs.
4. **`--capture`, `--help`, `--version`, `completions`.** Default-mode additions; rejected in Strict.
5. **`--no-ignore-sigpipe` / `--no-ignore-write-errors`.** moreutils exposes these to opt OUT of default-on behavior; rusty-pee v0.1.0 freezes the defaults on & provides no negation surface.
6. **SIGTERM-to-SIGKILL grace period.** Hardcoded 1 second (no flag, no env).

See [`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md) for the full per-flag matrix.

### `pee-alias` PATH-collision warning

Building with `--features pee-alias` installs a second binary named `pee` alongside `rusty-pee`. If moreutils is also on the same `PATH`, whichever directory comes first wins. Invoke `rusty-pee` (always unambiguous) or omit the `pee-alias` feature when moreutils is also present.

## What's not shipped

- **`--shell <path>` override.** Hardcoded `/bin/sh -c` on Unix, `cmd /C` on Windows. Add a wrapper script if you need a different shell.
- **`--no-ignore-sigpipe` / `--no-ignore-write-errors` negation flags.** Defaults are frozen on; no negation surface in v0.1.0.
- **Configurable SIGTERM-to-SIGKILL grace period.** Hardcoded 1 second.
- **Source-code derivation from moreutils.** This is a clean-room reimplementation. The moreutils `pee` Perl source is GPL'd & untouched. Same posture as [`uutils/coreutils`](https://github.com/uutils/coreutils).

## MSRV

Rust **1.85** (edition 2024). Re-verified against the portfolio's stable-minus-two policy at each release.

## License

Dual-licensed under [MIT](LICENSE) or [Apache-2.0](LICENSE-APACHE) at your option.
