//! Property-based tests. These prove the structural guarantees (round-trip,
//! bijectivity, totality); the known-answer vectors in `vectors.rs` prove
//! conformance to the spec.

use arxid::{deobfuscate, from_base62, obfuscate, to_base62, Arxid, CODE_LEN, MAX_ID};
use proptest::prelude::*;
use std::collections::HashSet;

proptest! {
    #[test]
    fn round_trip_over_the_whole_domain(key: u64, id in 0u64..=MAX_ID) {
        prop_assert_eq!(deobfuscate(obfuscate(id, key), key), id);
    }

    #[test]
    fn output_never_leaves_the_domain(key: u64, id: u64) {
        prop_assert!(obfuscate(id, key) <= MAX_ID);
        prop_assert!(deobfuscate(id, key) <= MAX_ID);
    }

    #[test]
    fn out_of_domain_input_is_reduced_not_rejected(key: u64, id: u64) {
        prop_assert_eq!(obfuscate(id, key), obfuscate(id & MAX_ID, key));
        prop_assert_eq!(deobfuscate(obfuscate(id, key), key), id & MAX_ID);
    }

    #[test]
    fn string_layer_round_trips(key: u64, id in 0u64..=MAX_ID) {
        let arxid = Arxid::new(key);
        let s = arxid.obfuscate_str(id);
        prop_assert_eq!(s.len(), CODE_LEN);
        prop_assert_eq!(arxid.deobfuscate_str(&s), Some(id));
    }

    #[test]
    fn base62_round_trips(n in 0u64..=MAX_ID) {
        prop_assert_eq!(from_base62(&to_base62(n)), Some(n));
    }

    #[test]
    fn base62_decode_never_panics(s in ".{0,16}") {
        let _ = from_base62(&s);
    }

    #[test]
    fn distinct_keys_generally_give_distinct_outputs(id in 0u64..=MAX_ID, a: u64, b: u64) {
        prop_assume!(a != b);
        // Not a hard guarantee for a single point (two permutations can agree
        // somewhere), but the codes must not be trivially key-independent.
        let differ_somewhere = (0..8).any(|d| {
            let probe = (id + d) & MAX_ID;
            obfuscate(probe, a) != obfuscate(probe, b)
        });
        prop_assert!(differ_somewhere);
    }
}

#[test]
fn injective_on_a_sampled_range() {
    let key = 0xD1B5_4A32_D192_ED03;
    let mut seen = HashSet::with_capacity(200_000);
    for id in 0..200_000u64 {
        let code = obfuscate(id, key);
        assert!(code <= MAX_ID);
        assert!(seen.insert(code), "collision at id={id} -> {code}");
    }
}

#[test]
fn injective_across_the_top_of_the_domain() {
    let key = 1;
    let mut seen = HashSet::with_capacity(100_000);
    for id in (MAX_ID - 99_999)..=MAX_ID {
        assert!(seen.insert(obfuscate(id, key)), "collision at id={id}");
    }
}
