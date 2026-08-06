//! Emits a large batch of randomized known-answer rows, for differential
//! testing against another implementation.
//!
//! The committed vectors in `/vectors/vectors.json` are fixed and small: they
//! pin the cases a port is most likely to get wrong, but they are the same 71
//! rows on every run. This program produces a fresh, seed-driven batch so CI
//! samples a different slice of the key and id space each time. A port that
//! agrees on the committed vectors but diverges on some untested key still gets
//! caught.
//!
//! ```text
//! cargo run --release --example gen_fuzz -- <seed> <count> > rows.json
//! ```
//!
//! Both arguments are optional (defaults: seed 0, 100000 rows). The seed is
//! echoed into the output so a failing CI run is reproducible locally.

use arxid::{to_base62, Arxid, MAX_ID};

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args.next().and_then(|a| a.parse().ok()).unwrap_or(0);
    let count: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(100_000);

    let mut state = seed ^ 0xA5A5_5A5A_C3C3_3C3C;

    println!("{{ \"seed\": {seed}, \"count\": {count}, \"rows\": [");
    for i in 0..count {
        let key = splitmix64(&mut state);

        // Mix uniformly random ids with small ones and domain edges: a port
        // that mishandles the top of the domain or the 32/40-bit boundary
        // fails on the edges, not on the bulk.
        let id = match i % 8 {
            0 => splitmix64(&mut state) % 1_000,
            1 => MAX_ID - (splitmix64(&mut state) % 1_000),
            2 => splitmix64(&mut state) % (1 << 20),
            3 => (1u64 << 32) + (splitmix64(&mut state) % 1_000),
            _ => splitmix64(&mut state) & MAX_ID,
        };

        let arxid = Arxid::new(key);
        let obfuscated = arxid.obfuscate(id);
        let comma = if i + 1 == count { "" } else { "," };
        println!(
            "  {{ \"key\": {key}, \"id\": {id}, \"obfuscated\": {obfuscated}, \"encoded\": \"{}\" }}{comma}",
            to_base62(obfuscated)
        );
    }
    println!("] }}");
}
