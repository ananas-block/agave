mod random_fixtures;

use {
    ark_bn254::Fr,
    ark_ff::UniformRand,
    ark_serialize::CanonicalSerialize,
    ark_std::rand::{SeedableRng, rngs::StdRng},
    criterion::{BenchmarkId, Criterion, criterion_group, criterion_main},
    random_fixtures::POOL,
    solana_poseidon::{Endianness, Parameters, hashv},
};

const SEED: u64 = 0xa17b428;
const NS: &[usize] = &[1, 2, 4, 8, 12];

fn fr_le(s: Fr) -> [u8; 32] {
    let mut buf = [0u8; 32];
    s.serialize_uncompressed(&mut buf[..]).expect("Fr serialize");
    buf
}

fn fr_be(s: Fr) -> [u8; 32] {
    let mut buf = fr_le(s);
    buf.reverse();
    buf
}

struct Pool {
    be: Vec<Vec<[u8; 32]>>,
    le: Vec<Vec<[u8; 32]>>,
}

fn build_pool(n: usize) -> Pool {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut be = Vec::with_capacity(POOL);
    let mut le = Vec::with_capacity(POOL);
    for _ in 0..POOL {
        let mut be_entry = Vec::with_capacity(n);
        let mut le_entry = Vec::with_capacity(n);
        for _ in 0..n {
            let s = Fr::rand(&mut rng);
            be_entry.push(fr_be(s));
            le_entry.push(fr_le(s));
        }
        be.push(be_entry);
        le.push(le_entry);
    }
    Pool { be, le }
}

fn ref_pool(entries: &[Vec<[u8; 32]>]) -> Vec<Vec<&[u8]>> {
    entries
        .iter()
        .map(|entry| entry.iter().map(|e| &e[..]).collect())
        .collect()
}

fn bench_poseidon(c: &mut Criterion) {
    let mut group = c.benchmark_group("Poseidon Bn254X5");
    for &n in NS {
        let pool = build_pool(n);
        let be_refs = ref_pool(&pool.be);
        let le_refs = ref_pool(&pool.le);

        let mut i = 0usize;
        group.bench_with_input(BenchmarkId::new("BE", n), &n, |b, _| {
            b.iter(|| {
                let r =
                    hashv(Parameters::Bn254X5, Endianness::BigEndian, &be_refs[i]).unwrap();
                i = (i + 1) % POOL;
                r
            })
        });

        let mut i = 0usize;
        group.bench_with_input(BenchmarkId::new("LE", n), &n, |b, _| {
            b.iter(|| {
                let r =
                    hashv(Parameters::Bn254X5, Endianness::LittleEndian, &le_refs[i]).unwrap();
                i = (i + 1) % POOL;
                r
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_poseidon);
criterion_main!(benches);
