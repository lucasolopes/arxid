//! Generates the canonical test vectors by running the reference implementation.
//!
//! The vectors are never written by hand. Regenerate them with:
//!
//! ```text
//! cargo run --example gen_vectors > ../../vectors/vectors.json
//! ```
//!
//! Changing the output of this program means changing the algorithm, which
//! requires a new spec version. See SPEC.md section 10.

use arxid::{to_base62, Arxid, MAX_ID};

/// Keys exercised by the vectors: the two degenerate ones, the golden constant,
/// an arbitrary large key, and a key with only the top bit set.
///
/// The last one is deliberate: a port that truncated the key to 32 bits would
/// treat it as 0 and produce the wrong answer. `u64::MAX` would be a poor
/// choice here because the key schedule maps `key` and `!key` to the same
/// permutation (see SPEC.md section 2), so its vectors would just duplicate
/// those of key 0.
const KEYS: [u64; 5] = [
    0,
    1,
    0x9E37_79B9_7F4A_7C15,
    0xD1B5_4A32_D192_ED03,
    0x8000_0000_0000_0000,
];

/// Domain edges plus a few ordinary values.
const IDS: [u64; 10] = [
    0,
    1,
    2,
    42,
    1_000,
    12_345,
    1_000_000,
    MAX_ID / 2,
    MAX_ID - 1,
    MAX_ID,
];

/// The key used for the consecutive-id run that demonstrates non-enumerability.
const SEQUENCE_KEY: u64 = 0x9E37_79B9_7F4A_7C15;

fn main() {
    let mut rows: Vec<(u64, u64)> = Vec::new();
    for key in KEYS {
        for id in IDS {
            rows.push((key, id));
        }
    }
    for id in 100..=110 {
        rows.push((SEQUENCE_KEY, id));
    }

    println!("[");
    for (i, &(key, id)) in rows.iter().enumerate() {
        let arxid = Arxid::new(key);
        let obfuscated = arxid.obfuscate(id);
        let encoded = to_base62(obfuscated);
        let comma = if i + 1 == rows.len() { "" } else { "," };
        println!(
            "  {{ \"key\": {key}, \"id\": {id}, \"obfuscated\": {obfuscated}, \"encoded\": \"{encoded}\" }}{comma}"
        );
    }
    println!("]");
}
