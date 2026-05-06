// Wallclock micro-bench for the alt_bn128 G1/G2 compress/decompress syscalls
// (`SyscallAltBn128Compression`, syscalls/src/lib.rs:2446-2580). Calls the
// `solana_bn254::compression::prelude::*` functions directly, bypassing the
// InvokeContext / memory-translation layer.
//
// Random uncompressed points are generated via ark-bn254 with the same seed
// as the alt_bn128 benches. Compressed forms are pre-computed once at setup
// by calling the compress functions; those outputs become the inputs for the
// decompress benches. Inputs cycle through a pool per criterion iteration.

mod random_fixtures;

use {
    criterion::{Criterion, criterion_group, criterion_main},
    solana_bn254::compression::prelude::{
        alt_bn128_g1_compress_be, alt_bn128_g1_compress_le, alt_bn128_g1_decompress_be,
        alt_bn128_g1_decompress_le, alt_bn128_g2_compress_be, alt_bn128_g2_compress_le,
        alt_bn128_g2_decompress_be, alt_bn128_g2_decompress_le,
    },
};

const POOL: usize = 128;

fn bench_g1(c: &mut Criterion) {
    let uncompressed = random_fixtures::random_g1_points(POOL);
    let compressed_be: Vec<Vec<u8>> = uncompressed
        .be
        .iter()
        .map(|p| alt_bn128_g1_compress_be(p).unwrap().to_vec())
        .collect();
    let compressed_le: Vec<Vec<u8>> = uncompressed
        .le
        .iter()
        .map(|p| alt_bn128_g1_compress_le(p).unwrap().to_vec())
        .collect();

    let mut group = c.benchmark_group("BN254 G1 compression");
    let mut i = 0usize;
    group.bench_function("Compress/BE", |b| {
        b.iter(|| {
            let r = alt_bn128_g1_compress_be(&uncompressed.be[i]).unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    let mut i = 0usize;
    group.bench_function("Compress/LE", |b| {
        b.iter(|| {
            let r = alt_bn128_g1_compress_le(&uncompressed.le[i]).unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    let mut i = 0usize;
    group.bench_function("Decompress/BE", |b| {
        b.iter(|| {
            let r = alt_bn128_g1_decompress_be(&compressed_be[i]).unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    let mut i = 0usize;
    group.bench_function("Decompress/LE", |b| {
        b.iter(|| {
            let r = alt_bn128_g1_decompress_le(&compressed_le[i]).unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    group.finish();
}

fn bench_g2(c: &mut Criterion) {
    let uncompressed = random_fixtures::random_g2_points(POOL);
    let compressed_be: Vec<Vec<u8>> = uncompressed
        .be
        .iter()
        .map(|p| alt_bn128_g2_compress_be(p).unwrap().to_vec())
        .collect();
    let compressed_le: Vec<Vec<u8>> = uncompressed
        .le
        .iter()
        .map(|p| alt_bn128_g2_compress_le(p).unwrap().to_vec())
        .collect();

    let mut group = c.benchmark_group("BN254 G2 compression");
    let mut i = 0usize;
    group.bench_function("Compress/BE", |b| {
        b.iter(|| {
            let r = alt_bn128_g2_compress_be(&uncompressed.be[i]).unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    let mut i = 0usize;
    group.bench_function("Compress/LE", |b| {
        b.iter(|| {
            let r = alt_bn128_g2_compress_le(&uncompressed.le[i]).unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    let mut i = 0usize;
    group.bench_function("Decompress/BE", |b| {
        b.iter(|| {
            let r = alt_bn128_g2_decompress_be(&compressed_be[i]).unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    let mut i = 0usize;
    group.bench_function("Decompress/LE", |b| {
        b.iter(|| {
            let r = alt_bn128_g2_decompress_le(&compressed_le[i]).unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    group.finish();
}

criterion_group!(benches, bench_g1, bench_g2);
criterion_main!(benches);
