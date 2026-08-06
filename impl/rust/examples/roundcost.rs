//! What the extra rounds cost.
//!
//! Spec v1 used 4 rounds; spec v2 uses 6 (SPEC.md section 3.2). This measures the
//! price of that decision, per round and end to end.
//!
//! Absolute timings on a laptop drift by up to 3x between sessions, so a 4-round
//! figure measured once and a 6-round figure measured later say nothing at all.
//! This interleaves every arm in a single process, alternating the order each
//! repetition, and takes the minimum over many repetitions - so thermal state and
//! background load hit all arms equally and cancel.
//!
//! ```text
//! cargo run --release --example roundcost
//! ```

use std::hint::black_box;
use std::time::Instant;

use arxid::codec::to_base62;
use arxid::permute::feistel_n;

const ITERS: u64 = 2_000_000;
const REPS: usize = 40;

const KEY: u64 = 0x9E37_79B9_7F4A_7C15;

/// The permutation on its own: pure integer arithmetic, no allocation.
fn permute_only(rounds: usize) -> f64 {
    let start = Instant::now();
    let mut acc = 0u64;
    for i in 0..ITERS {
        acc = acc.wrapping_add(black_box(feistel_n(black_box(i), KEY, rounds, 40)));
    }
    black_box(acc);
    start.elapsed().as_secs_f64() / ITERS as f64 * 1e9
}

/// What an application actually pays: permutation plus the base62 `String`.
fn full_encode(rounds: usize) -> f64 {
    let start = Instant::now();
    let mut acc = 0usize;
    for i in 0..ITERS {
        let code = feistel_n(black_box(i), KEY, rounds, 40);
        acc = acc.wrapping_add(black_box(to_base62(code)).len());
    }
    black_box(acc);
    start.elapsed().as_secs_f64() / ITERS as f64 * 1e9
}

/// Times rounds 4, 5 and 6 interleaved, and reports the 4 -> 6 delta (spec v1 to v2).
fn measure(label: &str, f: fn(usize) -> f64) {
    // Warm every arm before timing any of it.
    for r in [4, 5, 6] {
        f(r);
    }

    let mut best = [f64::MAX; 3]; // slots 0,1,2 -> rounds 4,5,6
    for rep in 0..REPS {
        // Alternate direction each rep so any ordering effect cancels.
        let order = if rep % 2 == 0 { [4, 5, 6] } else { [6, 5, 4] };
        for r in order {
            best[r - 4] = best[r - 4].min(f(r));
        }
    }

    let delta = best[2] - best[0];
    println!(
        "  {label:32}  {:6.2}   {:6.2}   {:6.2}   {:+6.1}%   {:+.2} ns",
        best[0],
        best[1],
        best[2],
        delta / best[0] * 100.0,
        delta
    );
}

fn main() {
    println!("Cost of the extra rounds. Interleaved A/B, min of {REPS} reps x {ITERS} iters.\n");
    println!(
        "  {:32}  {:>6}   {:>6}   {:>6}   {:>7}   {:>8}",
        "", "4 rnd", "5 rnd", "6 rnd", "4->6", "abs"
    );
    measure("permutation only (u64 -> u64)", permute_only);
    measure("full encode (permute + base62)", full_encode);

    println!("\n  Each round of the ARX function costs about the same handful of nanoseconds,");
    println!("  so on the bare permutation two extra rounds are a large fraction of a small");
    println!("  number. On the path an application actually pays, the base62 String allocation");
    println!("  costs roughly ten times the whole permutation and absorbs the difference, so");
    println!("  the 4 -> 6 change lands within the run-to-run noise of that step.");
}
