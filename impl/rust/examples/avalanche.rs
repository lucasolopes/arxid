//! Measures the avalanche / strict avalanche criterion (SAC) of the permutation.
//!
//! This is the measurement behind the claim that 4 rounds is the smallest round
//! count that closes diffusion. Run it with:
//!
//! ```text
//! cargo run --release --example avalanche
//! ```
//!
//! For each input bit `i`, flip it and count which output bits change.
//!
//! - **Avalanche**: mean fraction of output bits that flip. The ideal is 0.5:
//!   flipping any input bit should flip each output bit half the time.
//! - **Coverage**: how many of the 40x40 (input bit, output bit) pairs are
//!   reachable at all. Anything below 1600/1600 means some input bit cannot
//!   influence some output bit, which is a structural failure, not a
//!   statistical one.
//!
//! This measures a statistical property. It is not a cryptographic security
//! argument, and this construction has had no independent cryptographic review.
//! See SPEC.md section 9.

use arxid::permute::{feistel_n, MAX_ID, WIDTH_BITS};

const SAMPLES: u64 = 50_000;
const KEY: u64 = 0x9E37_79B9_7F4A_7C15;

/// SplitMix64, so the sample is pseudo-random but fully reproducible without
/// pulling in a dependency.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

fn main() {
    let width = WIDTH_BITS as usize;

    println!("avalanche over {SAMPLES} sampled ids, key {KEY:#018x}, width {width} bits\n");
    println!(
        "{:>6}  {:>10}  {:>12}  {:>10}",
        "rounds", "avalanche", "coverage", "worst pair"
    );

    for rounds in 1..=6usize {
        // flips[i][j] = how many times flipping input bit i flipped output bit j
        let mut flips = vec![vec![0u64; width]; width];
        let mut total_flipped: u64 = 0;

        let mut rng = SplitMix64(0x0123_4567_89AB_CDEF);
        for _ in 0..SAMPLES {
            let id = rng.next() & MAX_ID;
            let base = feistel_n(id, KEY, rounds, WIDTH_BITS);
            for (i, row) in flips.iter_mut().enumerate() {
                let flipped = feistel_n(id ^ (1u64 << i), KEY, rounds, WIDTH_BITS);
                let diff = base ^ flipped;
                total_flipped += diff.count_ones() as u64;
                for (j, cell) in row.iter_mut().enumerate() {
                    if diff & (1u64 << j) != 0 {
                        *cell += 1;
                    }
                }
            }
        }

        let pairs = (width * width) as u64;
        let avalanche = total_flipped as f64 / (SAMPLES * pairs) as f64;

        let reachable = flips.iter().flatten().filter(|&&c| c > 0).count();

        // The worst individual (i, j) bias: how far the least balanced pair sits
        // from the ideal 0.5.
        let worst = flips
            .iter()
            .flatten()
            .map(|&c| ((c as f64 / SAMPLES as f64) - 0.5).abs())
            .fold(0.0f64, f64::max);

        println!("{rounds:>6}  {avalanche:>10.4}  {reachable:>6}/{pairs:<5}  {worst:>10.4}");
    }

    println!(
        "\nideal: avalanche 0.5000, coverage {0}/{0}, worst pair 0.0000",
        width * width
    );
}
