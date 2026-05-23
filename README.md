# rusty-pee

A Rust port of the moreutils `pee` utility: fan a single stdin stream out to N concurrent shell-spawned children, aggregate their exit codes, and surface failures cleanly.

[![crates.io](https://img.shields.io/crates/v/rusty-pee.svg)](https://crates.io/crates/rusty-pee)
[![docs.rs](https://docs.rs/rusty-pee/badge.svg)](https://docs.rs/rusty-pee)
[![license: MIT OR Apache-2.0](https://img.shields.io/crates/l/rusty-pee.svg)](#license)

## Install

```sh
cargo install rusty-pee
# or, no toolchain required:
cargo binstall rusty-pee
```

Prebuilt binaries ship for the five DDR-003 targets (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64).

## Usage

```sh
# Default mode — fan stdin out to N children
journalctl -p err | rusty-pee 'grep selinux > /tmp/sel.log' 'wc -l > /tmp/errcount.txt'

# Capture mode — emit child stdouts in argv order after they exit
echo "alpha" | rusty-pee --capture 'cat' 'cat'

# Strict mode — byte-equal moreutils stderr + bitwise-OR exit aggregation
rusty-pee --strict 'validator-A' 'validator-B'

# When installed via the `pee-alias` Cargo feature, the `pee` binary
# auto-activates Strict mode (argv[0] auto-detect).
```

## Cargo Features

Five supported build configurations:

| Feature | Default | Includes |
|---|---|---|
| `cli` | ✓ | clap, clap_complete, anyhow, signal-hook; the `rusty-pee` binary |
| `pee-alias` | ✗ | also produces a second `pee` binary (auto-Strict via argv[0]) |
| `bench` | ✗ | criterion harness in `benches/throughput.rs` |
| `dev-helpers` | ✗ | gates the `fake-pee-child` test helper binary |
| `default-features = false` | n/a | library-only build; no clap/anyhow/clap_complete/signal-hook |

## Library API

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

The library fans out to N `Box<dyn Write + Send>` sinks instead of subprocesses — embed the fan-out pattern without spawning processes.

## Compatibility statement (vs moreutils pee)

Pinned upstream version: **moreutils 0.69** (same baseline as the rest of the rusty-* portfolio).

**Pipeline-safety guarantee**: every live sink receives every byte the parent reads, in argv (CLI) / registration (library) order. When a child closes its stdin mid-stream (`BrokenPipe`), the parent drops it from the live-set and continues feeding survivors. Memory is bounded — `O(BUFSIZ × N)`, not `O(input size)`.

**Documented intentional divergences from moreutils pee**:

1. **Default-mode exit-code aggregation uses `max(child_codes)`** instead of moreutils' bitwise OR over `WEXITSTATUS`. Strict mode preserves moreutils' bitwise-OR exactly. Example: children exiting 1 and 2 → Default exit=2, Strict exit=3.
2. **Hardcoded platform shell**: `/bin/sh -c` (Unix) / `cmd /C` (Windows). No `--shell <path>` flag in v0.1.0.
3. **Unknown-flag stderr format in Strict mode** emits ONLY the first unknown-flag error (`rusty-pee: invalid option -- 'X'` or `rusty-pee: unknown option -- 'foo'`). moreutils' POSIX `Getopt::Long` iterates per-character; we don't replicate that for undocumented inputs.
4. **`--capture`, `--help`, `--version`, `completions` subcommand** — not present in moreutils. Default-mode additions; rejected in Strict mode.
5. **`--no-ignore-sigpipe` / `--no-ignore-write-errors` negation flags** — moreutils exposes these to opt OUT of the default-on behavior; rusty-pee v0.1.0 freezes the defaults on and provides no negation surface.
6. **Configurable SIGTERM→SIGKILL grace period** — v0.1.0 hardcodes 1 second (no flag, no env).

**`pee-alias` PATH-collision warning.** Building with `--features pee-alias` installs a second binary named `pee` alongside `rusty-pee`. If moreutils is also installed on the same `PATH`, whichever directory comes first wins. Invoke `rusty-pee` (always unambiguous) or omit the `pee-alias` feature when moreutils is also present.

See [`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md) for the full per-flag matrix.

## Stability (lockstep SemVer)

Library and binary share a single crate version. Within `0.x`, minor version bumps may introduce breaking changes per standard Cargo semantics. Every public enum and struct is `#[non_exhaustive]` so variant additions are not breaking changes once `1.0` lands.

## MSRV

Minimum supported Rust version: **1.85** (edition 2024 floor). This is an upward deviation from the portfolio's "current stable minus two" rule, forced by edition 2024.

## License

Dual-licensed under either [MIT](LICENSE) or [Apache-2.0](LICENSE-APACHE) at your option.
