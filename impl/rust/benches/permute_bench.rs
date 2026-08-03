#![allow(missing_docs)]

//! Raw throughput of the permutation and of the base62 layer.
//!
//! `permute/obfuscate` is the number to quote for "raw integer transform": it
//! is the bare u64 -> u64 permutation with no string work at all.
//!
//! Ids come from a wrapping counter over the domain and are black-boxed on the
//! way in and out, so nothing is const-folded away.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use arxid::codec;
use arxid::permute::{self, MAX_ID};

const KEY: u64 = 0x9E37_79B9_7F4A_7C15;

fn bench_permute(c: &mut Criterion) {
    let mut group = c.benchmark_group("permute");

    // The raw integer transform: u64 -> u64, no allocation, no encoding.
    group.bench_function("obfuscate", |b| {
        let mut id = 0u64;
        b.iter(|| {
            id = id.wrapping_add(1) & MAX_ID;
            black_box(permute::obfuscate(black_box(id), black_box(KEY)))
        })
    });

    group.bench_function("deobfuscate", |b| {
        let mut id = 0u64;
        b.iter(|| {
            id = id.wrapping_add(1) & MAX_ID;
            let code = permute::obfuscate(id, KEY);
            black_box(permute::deobfuscate(black_box(code), black_box(KEY)))
        })
    });

    group.bench_function("roundtrip", |b| {
        let mut id = 0u64;
        b.iter(|| {
            id = id.wrapping_add(1) & MAX_ID;
            let code = permute::obfuscate(black_box(id), KEY);
            black_box(permute::deobfuscate(black_box(code), KEY))
        })
    });

    group.finish();
}

fn bench_codec(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec");

    group.bench_function("to_base62", |b| {
        let mut n = 0u64;
        b.iter(|| {
            n = n.wrapping_add(1) & MAX_ID;
            black_box(codec::to_base62(black_box(n)))
        })
    });

    group.bench_function("from_base62", |b| {
        let s = codec::to_base62(1_234_567_890);
        b.iter(|| black_box(codec::from_base62(black_box(&s))))
    });

    group.finish();
}

criterion_group!(benches, bench_permute, bench_codec);
criterion_main!(benches);
