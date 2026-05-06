// Wallclock micro-bench for the Poseidon hash syscall (`SyscallPoseidon`,
// syscalls/src/lib.rs:2363-2425). Calls `solana_poseidon::hashv` directly,
// bypassing the InvokeContext / memory-translation layer.
//
// Inputs are random Fr field elements generated via ark-bn254 with the same
// seed as the alt_bn128 benches, then cycled through a pool per criterion
// iteration to avoid cache-warmth bias.

use {
    ark_bn254::Fr,
    ark_ff::UniformRand,
    ark_serialize::CanonicalSerialize,
    ark_std::rand::{SeedableRng, rngs::StdRng},
    criterion::{BenchmarkId, Criterion, criterion_group, criterion_main},
    solana_poseidon::{Endianness, Parameters, hashv},
};

const SEED: u64 = 0xa17b428;
const POOL: usize = 128;
// SyscallPoseidon caps inputs at 12 (syscalls/src/lib.rs:2377).
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

fn bench_poseidon(c: &mut Criterion) {
    let mut group = c.benchmark_group("Poseidon Bn254X5");
    for &n in NS {
        let pool = build_pool(n);

        let mut i = 0usize;
        group.bench_with_input(BenchmarkId::new("BE", n), &n, |b, _| {
            b.iter(|| {
                let inputs: Vec<&[u8]> = pool.be[i].iter().map(|e| &e[..]).collect();
                let r = hashv(Parameters::Bn254X5, Endianness::BigEndian, &inputs).unwrap();
                i = (i + 1) % POOL;
                r
            })
        });

        let mut i = 0usize;
        group.bench_with_input(BenchmarkId::new("LE", n), &n, |b, _| {
            b.iter(|| {
                let inputs: Vec<&[u8]> = pool.le[i].iter().map(|e| &e[..]).collect();
                let r = hashv(Parameters::Bn254X5, Endianness::LittleEndian, &inputs).unwrap();
                i = (i + 1) % POOL;
                r
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_poseidon);
criterion_main!(benches);
