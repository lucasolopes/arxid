#![allow(missing_docs)]

//! Head-to-head: arxid's ARX round function against a structurally identical
//! Feistel network whose round function is HMAC-SHA256.
//!
//! Fairness notes:
//! - Both benches take the SAME input class: an integer id from a wrapping
//!   counter over arxid's 40-bit domain, black-boxed in and out.
//! - `feistel_hmac` holds the network structure constant: same balanced
//!   Feistel, same 4 rounds, same 40-bit width, same base62 step at the end.
//!   The ONLY thing that changes is the round function, ARX -> HMAC-SHA256.
//!   That isolates the actual claim being measured.
//! - Both therefore produce the same shape of output: a 7-character code.
//!
//! The ratio between the two matters more than either absolute number: they
//! run on the same machine in the same harness, so the gap survives whatever
//! CPU you are on.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use arxid::codec;
use arxid::permute::{self, MAX_ID};

type HmacSha256 = Hmac<Sha256>;

const KEY: u64 = 0x9E37_79B9_7F4A_7C15;

const FH_WIDTH_BITS: u32 = permute::WIDTH_BITS;
const FH_ROUNDS: usize = permute::ROUNDS;

/// The HMAC-SHA256 round function: HMAC(key, round_byte || R_bytes) truncated
/// to the half width. Same role arxid's ARX mix plays, different primitive.
fn fh_round_fn(key: &[u8], round: usize, half_bits: u32, r: u32) -> u32 {
    let mask: u32 = if half_bits >= 32 {
        u32::MAX
    } else {
        (1u32 << half_bits) - 1
    };
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(&[round as u8]);
    mac.update(&r.to_be_bytes());
    let tag = mac.finalize().into_bytes();
    let word = u32::from_be_bytes([tag[0], tag[1], tag[2], tag[3]]);
    word & mask
}

/// Encodes an id through the HMAC-round Feistel, then the same base62 step
/// arxid uses, so both sides produce comparable opaque strings.
fn feistel_hmac_encode(id: u64, key: &[u8]) -> String {
    let half = FH_WIDTH_BITS / 2;
    let mask = (1u64 << half) - 1;
    let id = id & ((1u64 << FH_WIDTH_BITS) - 1);
    let mut l = ((id >> half) & mask) as u32;
    let mut r = (id & mask) as u32;
    for round in 0..FH_ROUNDS {
        let f = fh_round_fn(key, round, half, r);
        let new_l = r;
        let new_r = (l ^ f) & (mask as u32);
        l = new_l;
        r = new_r;
    }
    let permuted = ((l as u64) << half) | (r as u64);
    codec::to_base62(permuted)
}

fn bench_arxid(c: &mut Criterion) {
    let mut group = c.benchmark_group("arxid");

    group.bench_function("encode", |b| {
        let mut id = 0u64;
        b.iter(|| {
            id = id.wrapping_add(1) & MAX_ID;
            let code = permute::obfuscate(black_box(id), KEY);
            black_box(codec::to_base62(black_box(code)))
        })
    });

    group.bench_function("decode", |b| {
        let mut id = 0u64;
        b.iter(|| {
            id = id.wrapping_add(1) & MAX_ID;
            let code = permute::obfuscate(id, KEY);
            let s = codec::to_base62(code);
            let n = codec::from_base62(black_box(&s)).expect("round-trips");
            black_box(permute::deobfuscate(black_box(n), KEY))
        })
    });

    group.finish();
}

fn bench_feistel_hmac(c: &mut Criterion) {
    let key = KEY.to_be_bytes();
    let mut group = c.benchmark_group("feistel_hmac");

    group.bench_function("encode", |b| {
        let mut id = 0u64;
        b.iter(|| {
            id = id.wrapping_add(1) & MAX_ID;
            black_box(feistel_hmac_encode(black_box(id), &key))
        })
    });

    group.finish();
}

criterion_group!(benches, bench_arxid, bench_feistel_hmac);
criterion_main!(benches);
