//! Criterion benches for `rusty-pee`. Gated behind the `bench` feature.
//!
//! **STUB** — full criterion harness lands in Polish (T121/T122).

#[cfg(feature = "bench")]
fn main() {
    eprintln!("rusty-pee: bench harness not yet implemented (Polish phase)");
}

#[cfg(not(feature = "bench"))]
fn main() {
    eprintln!("rusty-pee: rebuild with --features bench to run throughput benches");
}
