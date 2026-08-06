//! Measures how often consecutive ids map to adjacent codes.
//!
//! Spec v1 claimed, in its README and in three test files, that consecutive ids
//! never produce consecutive codes. The claim was wrong twice over:
//!
//! 1. It is not a property of a good permutation. An ideal permutation produces
//!    such pairs about twice over its whole domain, and a construction that
//!    guaranteed zero would be distinguishable from random for that reason.
//! 2. The v1 4-round construction produced them hundreds to thousands of times
//!    more often than chance, which leaks input ordering - the one thing the
//!    library exists to hide.
//!
//! This example reproduces both facts, and shows that 5 rounds already sits at
//! the expected rate, as does arxid's 6 (spec v2). Run with:
//!
//! ```text
//! cargo run --release --example adjacency
//! ```

use arxid::permute::feistel_n;

/// Deterministic RNG so the output is reproducible across runs and machines.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Counts ids `i` in `0..scan` where `f(i)` and `f(i+1)` differ by exactly 1.
fn adjacency(width: u32, rounds: usize, key: u64, scan: u64) -> usize {
    let mut count = 0usize;
    let mut prev = feistel_n(0, key, rounds, width);
    for i in 1..scan {
        let cur = feistel_n(i, key, rounds, width);
        if cur.abs_diff(prev) == 1 {
            count += 1;
        }
        prev = cur;
    }
    count
}

fn main() {
    println!("adjacent-output pairs for consecutive ids\n");

    println!("Exhaustive over the full domain at reduced widths.");
    println!("A random permutation gives ~2.00 regardless of width.\n");
    println!("  width   keys   4 rounds (v1)   5 rounds   6 rounds (v2)");
    for width in [16u32, 20, 24, 28] {
        let keys = if width >= 28 { 6 } else { 12 };
        let scan = 1u64 << width;
        let mut s = 0xABCD_EF01_2345_6789u64;
        let mut acc = [0usize; 3];
        for _ in 0..keys {
            let key = splitmix64(&mut s);
            for (slot, rounds) in [4usize, 5, 6].iter().enumerate() {
                acc[slot] += adjacency(width, *rounds, key, scan);
            }
        }
        let mean = |v: usize| v as f64 / f64::from(keys);
        println!(
            "  {width:5}   {keys:4}   {:13.2}   {:13.2}   {:8.2}",
            mean(acc[0]),
            mean(acc[1]),
            mean(acc[2])
        );
    }

    println!("\nAt the real 40-bit width, scanning a prefix (2^40 is not scannable).");
    println!("Expected for a random permutation over M ids: 2*M / 2^40.\n");
    println!("      scanned M   expected(rand)   4 rounds (v1)   5 rounds   6 rounds (v2)");
    for log_scan in [24u32, 28] {
        let scan = 1u64 << log_scan;
        let keys = 8u32;
        let expected = 2.0 * scan as f64 / (1u64 << 40) as f64 * f64::from(keys);
        let mut s = 0x0F1E_2D3C_4B5A_6978u64;
        let mut acc = [0usize; 3];
        for _ in 0..keys {
            let key = splitmix64(&mut s);
            for (slot, rounds) in [4usize, 5, 6].iter().enumerate() {
                acc[slot] += adjacency(40, *rounds, key, scan);
            }
        }
        println!(
            "  2^{log_scan} = {scan:<10}      {expected:9.4}   {:13}   {:13}   {:8}   (totals, {keys} keys)",
            acc[0], acc[1], acc[2]
        );
    }

    // A concrete pair, found rather than hardcoded. The published 0.1.0 crate
    // has its own (it used a different key schedule, so its counterexamples are
    // not these): key 0x652BFD48C7ED0458 maps ids 48326508 and 48326509 to the
    // adjacent codes "5kusgvr" and "5kusgvs".
    println!("\nSearching for a concrete 4-round pair at the real width:");
    let mut s = 0x0F1E_2D3C_4B5A_6978u64;
    let mut found = false;
    'search: for _ in 0..8 {
        let key = splitmix64(&mut s);
        let mut prev = feistel_n(0, key, 4, 40);
        for id in 1..(1u64 << 27) {
            let cur = feistel_n(id, key, 4, 40);
            if cur.abs_diff(prev) == 1 {
                println!("  key 0x{key:016X}");
                println!(
                    "    4 rounds: id {} -> {prev}, id {id} -> {cur}   (adjacent)",
                    id - 1
                );
                println!(
                    "    6 rounds: id {} -> {}, id {id} -> {}   (differ by {})",
                    id - 1,
                    feistel_n(id - 1, key, 6, 40),
                    feistel_n(id, key, 6, 40),
                    feistel_n(id, key, 6, 40).abs_diff(feistel_n(id - 1, key, 6, 40))
                );
                found = true;
                break 'search;
            }
            prev = cur;
        }
    }
    if !found {
        println!("  none in 8 keys x 2^27 ids this run");
    }
}
