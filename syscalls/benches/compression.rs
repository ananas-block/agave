mod random_fixtures;

use {
    criterion::{Criterion, criterion_group, criterion_main},
    solana_bn254::compression::prelude::{
        alt_bn128_g1_compress_be, alt_bn128_g1_compress_le, alt_bn128_g1_decompress_be,
        alt_bn128_g1_decompress_le, alt_bn128_g2_compress_be, alt_bn128_g2_compress_le,
        alt_bn128_g2_decompress_be, alt_bn128_g2_decompress_le,
    },
};

use random_fixtures::POOL;

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
