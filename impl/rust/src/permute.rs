//! The core permutation: a balanced Feistel network with an ARX round function,
//! format-preserving over a 40-bit domain.
//!
//! This module is the normative core of [SPEC.md](https://github.com/lucasolopes/arxid/blob/main/spec/SPEC.md)
//! sections 1 to 6. It is pure integer arithmetic: no allocation, no `std`, no
//! panics on any input.

/// Width of the permutation domain, in bits (SPEC.md section 1).
pub const WIDTH_BITS: u32 = 40;

/// Number of Feistel rounds (SPEC.md section 3.2).
///
/// Calibrated: 4 is the smallest round count that reaches exact 0.5000 avalanche
/// with full 40/40 bit coverage. Frozen by spec v1 - changing it changes every
/// output.
pub const ROUNDS: usize = 4;

/// Largest value in the domain: `2^40 - 1 = 1_099_511_627_775`.
pub const MAX_ID: u64 = (1u64 << WIDTH_BITS) - 1;

/// The golden-ratio constant used by the key schedule (SPEC.md section 4).
const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

/// Derives the 32-bit round subkey from the 64-bit master key (SPEC.md section 4).
///
/// All arithmetic wraps modulo 2^64; the result is the low 32 bits of
/// `x ^ (x >> 32)`.
#[inline]
fn subkey(key: u64, round: usize) -> u32 {
    let x = key.rotate_left((round as u32) * 7 + 1) ^ GOLDEN.wrapping_mul(round as u64 + 1);
    (x ^ (x >> 32)) as u32
}

/// ARX round function: mixes one `half_bits`-wide half with the round subkey
/// (SPEC.md section 5).
///
/// Every add wraps modulo 2^32 and every rotation is over 32 bits. The mask down
/// to `half_bits` happens only at the very end; masking earlier changes the
/// result.
#[inline]
fn round_fn(r: u32, key: u64, round: usize, half_bits: u32) -> u32 {
    let mask = (1u32 << half_bits) - 1;
    let rk = subkey(key, round);
    let mut x = r.wrapping_add(rk);
    x ^= x.rotate_left(7);
    x = x.wrapping_add(x.rotate_left(13));
    x ^= x.rotate_left(17);
    x & mask
}

/// Feistel forward pass over `width` bits with `rounds` rounds.
///
/// Generalized over width and round count so the same code path that produces
/// real output can also be exercised at small widths (where exhaustive
/// bijectivity is checkable). [`obfuscate`] is this function at
/// `(ROUNDS, WIDTH_BITS)`.
///
/// `width` must be even and at most 64. Bits of `input` above `width` are
/// ignored.
///
/// ```
/// # use arxid::permute::{feistel_n, feistel_n_inv};
/// let x = feistel_n(12345, 0xDEAD_BEEF, 4, 40);
/// assert_eq!(feistel_n_inv(x, 0xDEAD_BEEF, 4, 40), 12345);
/// ```
#[inline]
pub fn feistel_n(input: u64, key: u64, rounds: usize, width: u32) -> u64 {
    let half = width / 2;
    let mask = (1u64 << half) - 1;
    let mut l = ((input >> half) & mask) as u32;
    let mut r = (input & mask) as u32;
    for round in 0..rounds {
        let f = round_fn(r, key, round, half);
        let new_l = r;
        let new_r = l ^ f;
        l = new_l;
        r = new_r;
    }
    ((l as u64) << half) | (r as u64)
}

/// Feistel inverse pass: the exact inverse of [`feistel_n`] for the same
/// `(key, rounds, width)`.
///
/// ```
/// # use arxid::permute::{feistel_n, feistel_n_inv};
/// let key = 1;
/// for id in 0..64u64 {
///     assert_eq!(feistel_n_inv(feistel_n(id, key, 4, 40), key, 4, 40), id);
/// }
/// ```
#[inline]
pub fn feistel_n_inv(input: u64, key: u64, rounds: usize, width: u32) -> u64 {
    let half = width / 2;
    let mask = (1u64 << half) - 1;
    let mut l = ((input >> half) & mask) as u32;
    let mut r = (input & mask) as u32;
    for round in (0..rounds).rev() {
        let r_prev = l;
        let f = round_fn(r_prev, key, round, half);
        let l_prev = r ^ f;
        l = l_prev;
        r = r_prev;
    }
    ((l as u64) << half) | (r as u64)
}

/// Obfuscates `id` under `key`. Inputs outside `[0, MAX_ID]` are reduced with
/// `& MAX_ID`; the function is total and never panics.
///
/// ```
/// # use arxid::permute::{obfuscate, deobfuscate, MAX_ID};
/// let key = 0x9E37_79B9_7F4A_7C15;
/// let code = obfuscate(42, key);
/// assert!(code <= MAX_ID);
/// assert_eq!(deobfuscate(code, key), 42);
/// ```
pub fn obfuscate(id: u64, key: u64) -> u64 {
    feistel_n(id & MAX_ID, key, ROUNDS, WIDTH_BITS)
}

/// Recovers the original id from an obfuscated `code` under `key`. Inputs
/// outside `[0, MAX_ID]` are reduced with `& MAX_ID`; the function is total and
/// never panics.
///
/// ```
/// # use arxid::permute::{obfuscate, deobfuscate};
/// let key = 7;
/// assert_eq!(deobfuscate(obfuscate(1_000_000, key), key), 1_000_000);
/// ```
pub fn deobfuscate(code: u64, key: u64) -> u64 {
    feistel_n_inv(code & MAX_ID, key, ROUNDS, WIDTH_BITS)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "alloc")]
    use alloc::{vec, vec::Vec};

    use super::*;

    #[test]
    fn round_trip_edges() {
        let key = 0x9E37_79B9_7F4A_7C15;
        for id in [0u64, 1, 2, 42, 1000, MAX_ID / 2, MAX_ID - 1, MAX_ID] {
            let code = obfuscate(id, key);
            assert!(code <= MAX_ID, "code out of range: {code}");
            assert_eq!(deobfuscate(code, key), id, "round-trip failed for id={id}");
        }
    }

    #[test]
    fn out_of_domain_input_is_reduced_not_rejected() {
        let key = 3;
        assert_eq!(obfuscate(u64::MAX, key), obfuscate(MAX_ID, key));
        assert_eq!(deobfuscate(obfuscate(MAX_ID + 1, key), key), 0);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn bijectivity_on_small_width() {
        let key = 0xDEAD_BEEF_CAFE_BABE;
        let n = 1u64 << 20;
        let mut seen = vec![false; n as usize];
        for id in 0..n {
            let c = feistel_n(id, key, ROUNDS, 20);
            assert!(c < n);
            assert!(!seen[c as usize], "collision at id={id} -> {c}");
            seen[c as usize] = true;
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn distinct_keys_give_distinct_outputs() {
        let ids = [0u64, 1, 42, 12345, MAX_ID];
        let keys = [0u64, 1, GOLDEN, 0x0123_4567_89AB_CDEF];
        for id in ids {
            let mut outs: Vec<u64> = keys.map(|k| obfuscate(id, k)).to_vec();
            outs.sort_unstable();
            let before = outs.len();
            outs.dedup();
            assert_eq!(outs.len(), before, "keys collided on id={id}");
        }
    }

    #[test]
    fn sequential_ids_are_not_enumerable() {
        let key = 0x0123_4567_89AB_CDEF;
        let a = obfuscate(100, key);
        let b = obfuscate(101, key);
        assert!(
            a.abs_diff(b) > 1,
            "neighboring codes are sequential: {a} {b}"
        );
    }

    #[test]
    fn complement_keys_are_equivalent_by_design() {
        // SPEC.md section 2.1: the key schedule is not injective. `key` and
        // `!key` derive the same subkeys and therefore the same permutation.
        // This is frozen behavior, not a defect: a port that "fixes" it stops
        // conforming. Pinned here so nobody repairs it by accident.
        for key in [0u64, 1, GOLDEN, 0xD1B5_4A32_D192_ED03, 1 << 63] {
            for round in 0..ROUNDS {
                assert_eq!(subkey(key, round), subkey(!key, round));
            }
            for id in [0u64, 1, 42, 12345, MAX_ID] {
                assert_eq!(obfuscate(id, key), obfuscate(id, !key));
            }
        }
    }

    #[test]
    fn subkey_matches_spec_section_4() {
        // Hand-derived from SPEC.md section 4 for key = 1, round = 0:
        //   rotl64(1, 1) = 2; GOLDEN * 1 = 0x9E3779B97F4A7C15
        //   x = 2 ^ 0x9E3779B97F4A7C15 = 0x9E3779B97F4A7C17
        //   x >> 32 = 0x9E3779B9
        //   (x ^ (x >> 32)) as u32 = 0x7F4A7C17 ^ 0x9E3779B9 = 0xE17D05AE
        assert_eq!(subkey(1, 0), 0xE17D_05AE);
    }
}
