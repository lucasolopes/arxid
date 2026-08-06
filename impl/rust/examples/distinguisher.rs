//! Measures how many queries separate the permutation from a random one.
//!
//! This is the experiment that retired spec v1. Its 4-round construction is
//! distinguishable from a random permutation with roughly 2^13 chosen queries.
//! The defect is gone by 5 rounds; arxid ships 6 (spec v2), which shows no
//! measurable bias out to the full 2^20 slice an attacker can actually reach.
//!
//! ## The statistic
//!
//! Fix one half of the input and vary the other over `m` values. Writing the
//! Feistel intermediates `R0..R4` (SPEC.md section 3.2), with `R0` fixed:
//!
//! ```text
//!   R1 = L0 XOR F0(R0)      -- F0(R0) is constant, so R1 is a bijection of L0
//!   R2 = R0 XOR F1(R1)      -- F1 is a random function: collides at 2^-20
//!   L4 = R3 = R1 XOR F2(R2)
//! ```
//!
//! so `L4 XOR L0 = F0(R0) XOR F2(R2)`, a constant XOR `F2(R2)`. Collisions in
//! that statistic therefore inherit the collisions of `R2` on top of the
//! background rate, and the count comes out about twice what a random
//! permutation gives. Counting collisions is O(m log m), where counting the
//! equivalent pairs directly would be O(m^2).
//!
//! ## Reachability
//!
//! Encode-side, the attack needs `m` ids sharing their low 20 bits, i.e.
//! congruent mod 2^20. A deployment whose ids stay under a million has at most
//! one per class, so it cannot supply the structure at all.
//!
//! Decode-side it is free: the attacker picks codes, and all 2^20 codes sharing
//! a high half are valid 7-character strings. That direction only needs the
//! application to reveal the decoded id for a submitted code - which is exactly
//! why leaking internal ids matters.
//!
//! ```text
//! cargo run --release --example distinguisher
//! ```

use arxid::permute::{feistel_n, feistel_n_inv};

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A true random permutation on `2^width` points, as the ground-truth control.
fn random_perm(width: u32, seed: u64) -> Vec<u32> {
    let n = 1usize << width;
    let mut p: Vec<u32> = (0..n as u32).collect();
    let mut s = seed;
    for i in (1..n).rev() {
        let j = (splitmix64(&mut s) % (i as u64 + 1)) as usize;
        p.swap(i, j);
    }
    p
}

fn collision_pairs(mut vals: Vec<u32>) -> u64 {
    vals.sort_unstable();
    let mut total = 0u64;
    let mut i = 0usize;
    while i < vals.len() {
        let mut j = i + 1;
        while j < vals.len() && vals[j] == vals[i] {
            j += 1;
        }
        let run = (j - i) as u64;
        total += run * (run - 1) / 2;
        i = j;
    }
    total
}

/// Encode direction: fix the low half, vary the high half, collide `Lout XOR Lin`.
fn forward(width: u32, low: u64, m: usize, f: &dyn Fn(u64) -> u64) -> u64 {
    let half = width / 2;
    collision_pairs(
        (0..m as u64)
            .map(|hi| ((f((hi << half) | low) >> half) ^ hi) as u32)
            .collect(),
    )
}

/// Decode direction. A Feistel inverse exchanges the two halves, so the
/// mirrored statistic fixes the HIGH half and collides the low halves.
fn inverse(key: u64, rounds: usize, high: u64, m: usize) -> u64 {
    collision_pairs(
        (0..m as u64)
            .map(|lo| {
                let id = feistel_n_inv((high << 20) | lo, key, rounds, 40);
                ((id & 0xF_FFFF) ^ lo) as u32
            })
            .collect(),
    )
}

fn main() {
    // --- methodology check against a genuine random permutation ---
    println!("Methodology check at width 24, full slice m = 2^12, 20 trials each.");
    let width = 24u32;
    let m = 1usize << (width / 2);
    let baseline = (m as f64) * (m as f64 - 1.0) / 2.0 / f64::from(1u32 << (width / 2));
    println!("  random-permutation baseline: {baseline:.1}\n");

    let mut s = 0x1234_5678_9ABC_DEF0u64;
    for (label, rounds, control) in [
        ("true random permutation", 0usize, true),
        ("4 rounds (spec v1)", 4, false),
        ("5 rounds", 5, false),
        ("6 rounds (spec v2)", 6, false),
        ("8 rounds", 8, false),
    ] {
        let trials = 20;
        let mut total = 0u64;
        for _ in 0..trials {
            let key = splitmix64(&mut s);
            let low = splitmix64(&mut s) % (1u64 << (width / 2));
            total += if control {
                let table = random_perm(width, key);
                forward(width, low, m, &|x| u64::from(table[x as usize]))
            } else {
                forward(width, low, m, &|x| feistel_n(x, key, rounds, width))
            };
        }
        let mean = total as f64 / f64::from(trials);
        println!("  {label:26} {mean:9.1}   ratio {:.3}", mean / baseline);
    }

    // --- the real width, decode direction (the reachable one) ---
    // Pushed all the way to m = 2^20: with the high half fixed, every one of the
    // 2^20 low halves is a valid code the attacker can submit, so that is the
    // real ceiling of this attack, not an arbitrary cutoff.
    println!("\nDecode direction at the real 40-bit width, out to the full 2^20 slice.");
    println!("The attacker chooses codes, so this structure is always available.\n");
    println!("        m   baseline   4 rounds (v1)   ratio   6 rounds (v2)   ratio   separation");

    let mut s = 0x5A5A_1234_9999_ABCDu64;
    for logm in [10u32, 12, 13, 14, 16, 18, 20] {
        let m = 1usize << logm;
        let baseline = (m as f64) * (m as f64 - 1.0) / 2.0 / f64::from(1u32 << 20);
        // Fewer trials at the biggest slices: each is 2^20 Feistel inversions.
        let trials: u32 = if logm >= 20 {
            40
        } else if logm >= 18 {
            100
        } else {
            200
        };
        let (mut t4, mut t6) = (0u64, 0u64);
        for _ in 0..trials {
            let key = splitmix64(&mut s);
            let high = splitmix64(&mut s) % (1u64 << 20);
            t4 += inverse(key, 4, high, m);
            t6 += inverse(key, 6, high, m);
        }
        let m4 = t4 as f64 / f64::from(trials);
        let m6 = t6 as f64 / f64::from(trials);
        println!(
            "  {m:7}   {baseline:8.2}   {m4:13.2}   {:.3}   {m6:13.2}   {:.3}   {:6.1} sigma",
            m4 / baseline,
            m6 / baseline,
            (m4 - baseline) / baseline.sqrt()
        );
    }
    println!("\n  Separation is in units of one standard deviation for a single structure.");
    println!("  Spec v1 crosses 3 sigma around m = 2^13. Spec v2 stays at ratio ~1.00");
    println!("  all the way to the 2^20 wall.");
}
