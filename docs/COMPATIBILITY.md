# Compatibility Matrix — `rusty-pee` vs moreutils `pee`

> Pinned baseline: moreutils 0.69 (Ubuntu 24.04 LTS, same baseline as `rusty-sponge` and `rusty-vipe`).

> **Stub document — fleshed out during the Polish phase (T130).** Placeholder structure below.

## Flag matrix

| Flag / Form | Default mode | Strict mode |
|---|---|---|
| `--capture` | Rusty extension — buffer each child's stdout, emit in argv order after wait | Rejected as unknown flag |
| `--help` | clap-rendered help (exit 0) | Rejected as unknown long-form flag |
| `--version` | clap-rendered version (exit 0) | Rejected as unknown long-form flag |
| `--strict` | Activates Strict mode (Rusty extension) | Consumed pre-parse (no-op) |
| `--no-strict` | Explicit Default override; highest precedence | Consumed pre-parse (no-op) |
| `completions <shell>` | Subcommand: emit completion script | Treated as positional / rejected |
| Positional command strings | Spawned via `/bin/sh -c` / `cmd /C` | Same |
| `--` separator | Standard clap end-of-options | Standard end-of-options |

## Exit-code aggregation matrix

| Children exit codes | Default mode (max) | Strict mode (bitwise OR) | moreutils pee |
|---|---|---|---|
| (0, 0) | 0 | 0 | 0 |
| (0, 1) | 1 | 1 | 1 |
| (1, 0) | 1 | 1 | 1 |
| (2, 1) | 2 | 3 | 3 |
| (1, 2) | 2 | 3 | 3 |
| (255, 1) | 255 | 255 | 255 |
| (signal-killed, 0) | 1 | 1 | 1 |

## Intentional divergences from moreutils

1. **Default-mode exit-code aggregation uses `max()`** — moreutils uses bitwise OR. Strict mode preserves moreutils' bitwise-OR exactly.
2. **Hardcoded `/bin/sh` (Unix) and `cmd /C` (Windows)** — no env-driven (`$SHELL`/`%COMSPEC%`) or flag-driven (`--shell <path>`) shell selection in v0.1.0.
3. **`--ignore-sigpipe` and `--ignore-write-errors` defaults are frozen on** — moreutils exposes negation flags (`--no-ignore-*`); rusty-pee v0.1.0 does not.
4. **SIGTERM→SIGKILL grace period hardcoded to 1 second** — not configurable.
5. **First-error-only unknown-flag stderr in Strict mode** — moreutils' POSIX `Getopt::Long` iterates per-character.
6. **`--capture`, `--help`, `--version`, `completions` subcommand** — Rusty additions.

## Atomic-safety guarantee scope

pee is fundamentally a non-atomic operation — children write to their own outputs (files via shell redirection, downstream pipes, etc.) and the parent has no visibility into those side effects. Key invariants:

- **Byte-perfect delivery to survivors** (FR-002, FR-003) — every byte the parent reads reaches every live child. Children dropped from the live-set after `BrokenPipe` keep whatever prefix they accepted.
- **Backpressure-bounded memory** (FR-002) — `O(BUFSIZ × N)`, not `O(input size)`, on every platform.
- **Signal-driven child cleanup** (FR-010) — all live children are terminated within 1s grace + SIGKILL fallback.
- **Uncatchable signals** — parent dies; children inherit the same signal via process-group delivery (Unix) / `CTRL_*_EVENT` propagation (Windows). May leak in rare cases; documented limitation.

## Known limitations at v0.1.0

- See [`CHANGELOG.md`](../CHANGELOG.md) §"Known limitations at v0.1.0".

---

**Generation note.** Hand-authored — the flag surface is small enough that hand-maintenance plus the byte-equal `compat_strict.rs` tests provide equivalent drift protection.
