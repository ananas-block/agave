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
    const NS: &[usize] = &[2, 3, 4, 8, 16];

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

fn bench_pairing_prepared(c: &mut Criterion) {
    use solana_bn254_prepared_syscall::{alt_bn128_pairing_prepared, Version};

    const NS: &[usize] = &[2, 3, 4, 8, 16];
    const POOL: usize = 64;

    let mut group = c.benchmark_group("BN254 prepared pairing");
    for &n in NS {
        let pool = random_fixtures::random_pairing_prepared(POOL, n);

        for (g1s, g2_preps) in pool.g1s.iter().zip(pool.g2_preps.iter()) {
            let _ = alt_bn128_pairing_prepared(Version::V0, g1s, g2_preps).unwrap();
        }

        let mut i = 0usize;
        group.bench_with_input(BenchmarkId::new("LE", n), &n, |b, _| {
            b.iter(|| {
                let r = alt_bn128_pairing_prepared(
                    Version::V0,
                    &pool.g1s[i],
                    &pool.g2_preps[i],
                )
                .unwrap();
                i = (i + 1) % POOL;
                r
            })
        });
    }
    group.finish();
}

fn bench_pairing_gnark(c: &mut Criterion) {
    const NS: &[usize] = &[2, 3, 4, 8, 16];

    let mut group = c.benchmark_group("BN254 Pairing gnark");
    for &n in NS {
        // Reuses the existing telescoping LE pool — gnark's full-pairing FFI
        // accepts the same `n * 192`-byte LE concatenation that the standard
        // alt_bn128 pairing path already produces.
        let pool = random_fixtures::random_pairing(PAIRING_POOL, n);

        for le in pool.le.iter() {
            let mut out = [0u8; solana_bn254_gnark::sizes::PAIRING_OUTPUT];
            let code = solana_bn254_gnark::pairing(le, &mut out);
            assert_eq!(code, 0, "gnark pairing returned {code}");
            assert_eq!(out[0], 0x01, "telescoping inputs must pair to identity");
        }

        let mut out = [0u8; solana_bn254_gnark::sizes::PAIRING_OUTPUT];
        let mut i = 0usize;
        group.bench_with_input(BenchmarkId::new("LE", n), &n, |b, _| {
            b.iter(|| {
                let code = solana_bn254_gnark::pairing(&pool.le[i], &mut out);
                i = (i + 1) % PAIRING_POOL;
                code
            })
        });
    }
    group.finish();
}

fn bench_pairing_prepared_gnark(c: &mut Criterion) {
    const NS: &[usize] = &[2, 3, 4, 8, 16];
    const POOL: usize = 64;

    let mut group = c.benchmark_group("BN254 prepared pairing gnark");
    for &n in NS {
        let pool = random_fixtures::random_pairing_gnark_prepared(POOL, n);

        for (g1s, lines) in pool.g1s.iter().zip(pool.lines.iter()) {
            let mut gt = [0u8; solana_bn254_gnark::sizes::GT];
            let code = solana_bn254_gnark::pairing_prepared(g1s, lines, &mut gt);
            assert_eq!(code, 0, "gnark prepared pairing returned {code}");
        }

        let mut gt = [0u8; solana_bn254_gnark::sizes::GT];
        let mut i = 0usize;
        group.bench_with_input(BenchmarkId::new("LE", n), &n, |b, _| {
            b.iter(|| {
                let code =
                    solana_bn254_gnark::pairing_prepared(&pool.g1s[i], &pool.lines[i], &mut gt);
                i = (i + 1) % POOL;
                code
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_g1_random,
    bench_g2_random,
    bench_pairing_random,
    bench_pairing_prepared,
    bench_pairing_gnark,
    bench_pairing_prepared_gnark,
);
criterion_main!(benches);
