mod random_fixtures;

use {
    criterion::{BenchmarkId, Criterion, criterion_group, criterion_main},
    solana_bn254::versioned::{
        Endianness, VersionedG1Addition, VersionedG1Multiplication, VersionedG2Addition,
        VersionedG2Multiplication, VersionedPairing, alt_bn128_versioned_g1_addition,
        alt_bn128_versioned_g1_multiplication, alt_bn128_versioned_g2_addition,
        alt_bn128_versioned_g2_multiplication, alt_bn128_versioned_pairing,
    },
};

use random_fixtures::{PAIRING_POOL, POOL};

fn bench_g1_random(c: &mut Criterion) {
    let add = random_fixtures::random_g1_add(POOL);
    let mul = random_fixtures::random_g1_mul(POOL);
    let mut group = c.benchmark_group("BN254 G1 random");
    let mut i = 0usize;
    group.bench_function(BenchmarkId::new("Addition", "BE"), |b| {
        b.iter(|| {
            let r =
                alt_bn128_versioned_g1_addition(VersionedG1Addition::V0, &add.be[i], Endianness::BE)
                    .unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    let mut i = 0usize;
    group.bench_function(BenchmarkId::new("Addition", "LE"), |b| {
        b.iter(|| {
            let r =
                alt_bn128_versioned_g1_addition(VersionedG1Addition::V0, &add.le[i], Endianness::LE)
                    .unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    let mut i = 0usize;
    group.bench_function(BenchmarkId::new("Multiplication", "BE"), |b| {
        b.iter(|| {
            let r = alt_bn128_versioned_g1_multiplication(
                VersionedG1Multiplication::V1,
                &mul.be[i],
                Endianness::BE,
            )
            .unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    let mut i = 0usize;
    group.bench_function(BenchmarkId::new("Multiplication", "LE"), |b| {
        b.iter(|| {
            let r = alt_bn128_versioned_g1_multiplication(
                VersionedG1Multiplication::V1,
                &mul.le[i],
                Endianness::LE,
            )
            .unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    group.finish();
}

fn bench_g2_random(c: &mut Criterion) {
    let add = random_fixtures::random_g2_add(POOL);
    let mul = random_fixtures::random_g2_mul(POOL);
    let mut group = c.benchmark_group("BN254 G2 random");
    let mut i = 0usize;
    group.bench_function(BenchmarkId::new("Addition", "BE"), |b| {
        b.iter(|| {
            let r =
                alt_bn128_versioned_g2_addition(VersionedG2Addition::V0, &add.be[i], Endianness::BE)
                    .unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    let mut i = 0usize;
    group.bench_function(BenchmarkId::new("Addition", "LE"), |b| {
        b.iter(|| {
            let r =
                alt_bn128_versioned_g2_addition(VersionedG2Addition::V0, &add.le[i], Endianness::LE)
                    .unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    let mut i = 0usize;
    group.bench_function(BenchmarkId::new("Multiplication", "BE"), |b| {
        b.iter(|| {
            let r = alt_bn128_versioned_g2_multiplication(
                VersionedG2Multiplication::V0,
                &mul.be[i],
                Endianness::BE,
            )
            .unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    let mut i = 0usize;
    group.bench_function(BenchmarkId::new("Multiplication", "LE"), |b| {
        b.iter(|| {
            let r = alt_bn128_versioned_g2_multiplication(
                VersionedG2Multiplication::V0,
                &mul.le[i],
                Endianness::LE,
            )
            .unwrap();
            i = (i + 1) % POOL;
            r
        })
    });
    group.finish();
}

fn bench_pairing_random(c: &mut Criterion) {
    const NS: &[usize] = &[2, 4, 8, 16];

    let mut group = c.benchmark_group("BN254 Pairing random");
    for &n in NS {
        let pool = random_fixtures::random_pairing(PAIRING_POOL, n);

        for (be, le) in pool.be.iter().zip(pool.le.iter()) {
            let be_out =
                alt_bn128_versioned_pairing(VersionedPairing::V1, be, Endianness::BE).unwrap();
            assert_eq!(be_out.last(), Some(&0x01), "BE pairing must equal identity");
            let le_out =
                alt_bn128_versioned_pairing(VersionedPairing::V1, le, Endianness::LE).unwrap();
            assert_eq!(le_out.first(), Some(&0x01), "LE pairing must equal identity");
        }

        let mut i = 0usize;
        group.bench_with_input(BenchmarkId::new("BE", n), &n, |b, _| {
            b.iter(|| {
                let r =
                    alt_bn128_versioned_pairing(VersionedPairing::V1, &pool.be[i], Endianness::BE)
                        .unwrap();
                i = (i + 1) % PAIRING_POOL;
                r
            })
        });
        let mut i = 0usize;
        group.bench_with_input(BenchmarkId::new("LE", n), &n, |b, _| {
            b.iter(|| {
                let r =
                    alt_bn128_versioned_pairing(VersionedPairing::V1, &pool.le[i], Endianness::LE)
                        .unwrap();
                i = (i + 1) % PAIRING_POOL;
                r
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_g1_random, bench_g2_random, bench_pairing_random,);
criterion_main!(benches);
