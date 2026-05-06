mod random_fixtures;
mod test_vectors;

use {
    criterion::{BenchmarkId, Criterion, criterion_group, criterion_main},
    solana_bn254::versioned::{
        Endianness, VersionedG1Addition, VersionedG1Multiplication, VersionedG2Addition,
        VersionedG2Multiplication, VersionedPairing, alt_bn128_versioned_g1_addition,
        alt_bn128_versioned_g1_multiplication, alt_bn128_versioned_g2_addition,
        alt_bn128_versioned_g2_multiplication, alt_bn128_versioned_pairing,
    },
    test_vectors::*,
};

// `solana_bn254::versioned::Endianness` is `!Copy`, so we re-construct it inside
// each closure via this helper rather than capturing one value.
fn endianness(label: &str) -> Endianness {
    match label {
        "BE" => Endianness::BE,
        "LE" => Endianness::LE,
        _ => unreachable!(),
    }
}

fn bench_g1_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("BN254 G1");
    for (label, add, mul) in [
        ("BE", INPUT_BE_G1_ADD, INPUT_BE_G1_MUL),
        ("LE", INPUT_LE_G1_ADD, INPUT_LE_G1_MUL),
    ] {
        group.bench_function(BenchmarkId::new("Addition", label), |b| {
            b.iter(|| {
                alt_bn128_versioned_g1_addition(VersionedG1Addition::V0, add, endianness(label))
                    .unwrap()
            })
        });
        group.bench_function(BenchmarkId::new("Multiplication", label), |b| {
            b.iter(|| {
                alt_bn128_versioned_g1_multiplication(
                    VersionedG1Multiplication::V1,
                    mul,
                    endianness(label),
                )
                .unwrap()
            })
        });
    }
    group.finish();
}

fn bench_g2_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("BN254 G2");
    for (label, add, mul) in [
        ("BE", INPUT_BE_G2_ADD, INPUT_BE_G2_MUL),
        ("LE", INPUT_LE_G2_ADD, INPUT_LE_G2_MUL),
    ] {
        group.bench_function(BenchmarkId::new("Addition", label), |b| {
            b.iter(|| {
                alt_bn128_versioned_g2_addition(VersionedG2Addition::V0, add, endianness(label))
                    .unwrap()
            })
        });
        group.bench_function(BenchmarkId::new("Multiplication", label), |b| {
            b.iter(|| {
                alt_bn128_versioned_g2_multiplication(
                    VersionedG2Multiplication::V0,
                    mul,
                    endianness(label),
                )
                .unwrap()
            })
        });
    }
    group.finish();
}

fn bench_pairing(c: &mut Criterion) {
    let mut group = c.benchmark_group("BN254 Pairing");
    for &n in &[1usize, 2, 4, 8, 16] {
        let be: Vec<u8> = INPUT_BE_PAIRING_ONE_PAIR.repeat(n);
        let le: Vec<u8> = INPUT_LE_PAIRING_ONE_PAIR.repeat(n);
        group.bench_with_input(BenchmarkId::new("BE", n), &n, |b, _| {
            b.iter(|| {
                alt_bn128_versioned_pairing(VersionedPairing::V1, &be, Endianness::BE).unwrap()
            })
        });
        group.bench_with_input(BenchmarkId::new("LE", n), &n, |b, _| {
            b.iter(|| {
                alt_bn128_versioned_pairing(VersionedPairing::V1, &le, Endianness::LE).unwrap()
            })
        });
    }
    group.finish();
}

// Pool size for cycled-input benches. 128 is large enough that no criterion
// sample (defaults to 100 samples × thousands of iters) reuses an input within
// a single sample window, but small enough to keep memory negligible.
const POOL: usize = 128;

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
    const N: usize = 4;
    // Smaller pool: each entry is N*192 = 768 B; pool of 32 is enough to
    // cycle past any single criterion sample window.
    const PAIRING_POOL: usize = 32;
    let pool = random_fixtures::random_pairing(PAIRING_POOL, N);

    // Sanity-check every pool entry: random fixtures are constructed to
    // produce the Fq12 identity. If any assertion fires, the bench is
    // measuring a different code path than real proof verification.
    for (be, le) in pool.be.iter().zip(pool.le.iter()) {
        let be_out = alt_bn128_versioned_pairing(VersionedPairing::V1, be, Endianness::BE).unwrap();
        assert_eq!(be_out.last(), Some(&0x01), "BE pairing must equal identity");
        let le_out = alt_bn128_versioned_pairing(VersionedPairing::V1, le, Endianness::LE).unwrap();
        assert_eq!(le_out.first(), Some(&0x01), "LE pairing must equal identity");
    }

    let mut group = c.benchmark_group("BN254 Pairing random");
    let mut i = 0usize;
    group.bench_with_input(BenchmarkId::new("BE", N), &N, |b, _| {
        b.iter(|| {
            let r = alt_bn128_versioned_pairing(VersionedPairing::V1, &pool.be[i], Endianness::BE)
                .unwrap();
            i = (i + 1) % PAIRING_POOL;
            r
        })
    });
    let mut i = 0usize;
    group.bench_with_input(BenchmarkId::new("LE", N), &N, |b, _| {
        b.iter(|| {
            let r = alt_bn128_versioned_pairing(VersionedPairing::V1, &pool.le[i], Endianness::LE)
                .unwrap();
            i = (i + 1) % PAIRING_POOL;
            r
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_g1_ops,
    bench_g2_ops,
    bench_pairing,
    bench_g1_random,
    bench_g2_random,
    bench_pairing_random,
);
criterion_main!(benches);
