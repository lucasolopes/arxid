//! Validates the reference implementation against the canonical test vectors.
//!
//! This is the interop contract. Round-trip tests are symmetric and hide
//! width/wrapping bugs; only these known-answer vectors catch them. Every
//! implementation in every language runs this same file.

use std::fs;
use std::path::PathBuf;

use arxid::{from_base62, to_base62, Arxid, CODE_LEN, MAX_ID};
use serde_json::Value;

fn vectors_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vectors/vectors.json")
}

fn load() -> Vec<Value> {
    let path = vectors_path();
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read the canonical vectors at {}: {e}. \
             Regenerate with `cargo run --example gen_vectors > ../../vectors/vectors.json`.",
            path.display()
        )
    });
    match serde_json::from_str(&raw).expect("vectors.json is not valid JSON") {
        Value::Array(rows) => rows,
        other => panic!("vectors.json must be a JSON array, got {other:?}"),
    }
}

fn field_u64(row: &Value, name: &str, index: usize) -> u64 {
    row.get(name)
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("vector #{index} is missing the u64 field `{name}`: {row}"))
}

fn field_str<'a>(row: &'a Value, name: &str, index: usize) -> &'a str {
    row.get(name)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("vector #{index} is missing the string field `{name}`: {row}"))
}

#[test]
fn every_vector_matches_in_both_directions() {
    let rows = load();
    assert!(!rows.is_empty(), "vectors.json is empty");

    for (i, row) in rows.iter().enumerate() {
        let key = field_u64(row, "key", i);
        let id = field_u64(row, "id", i);
        let expected_obfuscated = field_u64(row, "obfuscated", i);
        let expected_encoded = field_str(row, "encoded", i);

        let arxid = Arxid::new(key);

        assert_eq!(
            arxid.obfuscate(id),
            expected_obfuscated,
            "vector #{i}: obfuscate({id}) under key {key}"
        );
        assert_eq!(
            arxid.deobfuscate(expected_obfuscated),
            id,
            "vector #{i}: deobfuscate({expected_obfuscated}) under key {key}"
        );
        assert_eq!(
            to_base62(expected_obfuscated),
            expected_encoded,
            "vector #{i}: to_base62({expected_obfuscated})"
        );
        assert_eq!(
            from_base62(expected_encoded),
            Some(expected_obfuscated),
            "vector #{i}: from_base62({expected_encoded:?})"
        );
        assert_eq!(
            arxid.obfuscate_str(id),
            expected_encoded,
            "vector #{i}: obfuscate_str({id}) under key {key}"
        );
        assert_eq!(
            arxid.deobfuscate_str(expected_encoded),
            Some(id),
            "vector #{i}: deobfuscate_str({expected_encoded:?}) under key {key}"
        );
    }
}

#[test]
fn vectors_are_well_formed_and_cover_the_required_cases() {
    let rows = load();

    let mut keys: Vec<u64> = Vec::new();
    let mut ids: Vec<u64> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let key = field_u64(row, "key", i);
        let id = field_u64(row, "id", i);
        let obfuscated = field_u64(row, "obfuscated", i);
        let encoded = field_str(row, "encoded", i);

        assert!(id <= MAX_ID, "vector #{i}: id {id} is outside the domain");
        assert!(
            obfuscated <= MAX_ID,
            "vector #{i}: obfuscated {obfuscated} is outside the domain"
        );
        assert_eq!(
            encoded.len(),
            CODE_LEN,
            "vector #{i}: encoded {encoded:?} is not {CODE_LEN} characters"
        );

        keys.push(key);
        ids.push(id);
    }

    keys.sort_unstable();
    keys.dedup();
    assert!(
        keys.len() >= 4,
        "the vectors must exercise at least 4 distinct keys, found {}",
        keys.len()
    );
    for required in [0u64, 1, 0x9E37_79B9_7F4A_7C15] {
        assert!(keys.contains(&required), "missing required key {required}");
    }

    for required in [0u64, 1, 2, MAX_ID, MAX_ID - 1, MAX_ID / 2] {
        assert!(
            ids.contains(&required),
            "missing required edge id {required}"
        );
    }
}

#[test]
fn the_consecutive_run_does_not_preserve_input_order() {
    let rows = load();

    // The run of consecutive ids exists so ports exercise a stretch of
    // neighbouring inputs. What is checked is that the outputs carry no trace
    // of the input ordering.
    //
    // What is NOT checked, deliberately: that neighbouring ids never produce
    // adjacent codes. Spec v1 asserted that here and it was false - see
    // SPEC.md section 11 and `examples/adjacency.rs`. A construction that
    // truly guaranteed it would be distinguishable from a random permutation
    // for that very reason.
    let mut run: Vec<(u64, u64)> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| (field_u64(row, "id", i), field_u64(row, "obfuscated", i)))
        .filter(|(id, _)| (100..=110).contains(id))
        .collect();
    run.sort_unstable();
    run.dedup();

    assert!(
        run.len() >= 11,
        "the vectors must contain a consecutive run of ids 100..=110, found {}",
        run.len()
    );

    let ascending = run.windows(2).filter(|w| w[0].1 < w[1].1).count();
    assert!(
        ascending > 0 && ascending < run.len() - 1,
        "the run is monotonic ({ascending}/{} ascending steps), which would leak the input order",
        run.len() - 1
    );
}
