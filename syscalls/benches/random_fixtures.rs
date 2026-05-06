// Shared by multiple bench binaries; each only uses a subset of helpers.
#![allow(dead_code)]

// Random valid BN254 inputs generated via ark-bn254 with a fixed seed.
//
// Each public function returns an `InputPool` of `pool_size` independent
// fixtures. The bench cycles through them with `i = (i + 1) % pool_size` so
// every criterion sample sees a fresh input — same pattern as the historical
// `altbn128_syscall_benches/sdk/benches/bn_syscalls.rs` (2022). Reusing one
// fixed input would let cache warmth and branch prediction bias the numbers.
//
// arkworks serializes G1/G2 in LE (c0 before c1 for Fq2), which matches
// `solana_bn254::target_arch::convert_endianness` LE expectations. The BE form
// is derived by chunked byte-reversal (32-byte chunks for G1 / scalars,
// 64-byte chunks for G2).

// `rand` is re-exported by ark-std at the version arkworks expects (currently
// rand 0.8 series). Using the workspace's rand 0.9 directly conflicts with the
// `RngCore` trait bound on `UniformRand::rand`.
use {
    ark_bn254::{Fr, G1Projective, G2Projective},
    ark_ec::CurveGroup,
    ark_ff::UniformRand,
    ark_serialize::CanonicalSerialize,
    ark_std::rand::{rngs::StdRng, SeedableRng},
};

const SEED: u64 = 0xa17b428;

fn rng() -> StdRng {
    StdRng::seed_from_u64(SEED)
}

fn g1_le(p: G1Projective) -> [u8; 64] {
    let mut buf = [0u8; 64];
    p.into_affine()
        .serialize_uncompressed(&mut buf[..])
        .expect("G1 serialize");
    buf
}

fn g2_le(p: G2Projective) -> [u8; 128] {
    let mut buf = [0u8; 128];
    p.into_affine()
        .serialize_uncompressed(&mut buf[..])
        .expect("G2 serialize");
    buf
}

fn fr_le(s: Fr) -> [u8; 32] {
    let mut buf = [0u8; 32];
    s.serialize_uncompressed(&mut buf[..])
        .expect("Fr serialize");
    buf
}

fn reverse_chunks(le: &[u8], chunk: usize) -> Vec<u8> {
    le.chunks_exact(chunk)
        .flat_map(|c| c.iter().rev().copied())
        .collect()
}

pub struct InputPool {
    pub be: Vec<Vec<u8>>,
    pub le: Vec<Vec<u8>>,
}

impl InputPool {
    fn with_capacity(n: usize) -> Self {
        Self {
            be: Vec::with_capacity(n),
            le: Vec::with_capacity(n),
        }
    }
}

/// Pool of single random G1 points (one 64-byte point per entry).
pub fn random_g1_points(pool_size: usize) -> InputPool {
    let mut r = rng();
    let mut pool = InputPool::with_capacity(pool_size);
    for _ in 0..pool_size {
        let p = G1Projective::rand(&mut r);
        let le = g1_le(p).to_vec();
        pool.be.push(reverse_chunks(&le, 32));
        pool.le.push(le);
    }
    pool
}

/// Pool of single random G2 points (one 128-byte point per entry).
pub fn random_g2_points(pool_size: usize) -> InputPool {
    let mut r = rng();
    let mut pool = InputPool::with_capacity(pool_size);
    for _ in 0..pool_size {
        let p = G2Projective::rand(&mut r);
        let le = g2_le(p).to_vec();
        pool.be.push(reverse_chunks(&le, 64));
        pool.le.push(le);
    }
    pool
}

pub fn random_g1_add(pool_size: usize) -> InputPool {
    let mut r = rng();
    let mut pool = InputPool::with_capacity(pool_size);
    for _ in 0..pool_size {
        let p1 = G1Projective::rand(&mut r);
        let p2 = G1Projective::rand(&mut r);
        let mut le = Vec::with_capacity(128);
        le.extend_from_slice(&g1_le(p1));
        le.extend_from_slice(&g1_le(p2));
        pool.be.push(reverse_chunks(&le, 32));
        pool.le.push(le);
    }
    pool
}

pub fn random_g1_mul(pool_size: usize) -> InputPool {
    let mut r = rng();
    let mut pool = InputPool::with_capacity(pool_size);
    for _ in 0..pool_size {
        let p = G1Projective::rand(&mut r);
        let s = Fr::rand(&mut r);
        let mut le = Vec::with_capacity(96);
        le.extend_from_slice(&g1_le(p));
        le.extend_from_slice(&fr_le(s));
        pool.be.push(reverse_chunks(&le, 32));
        pool.le.push(le);
    }
    pool
}

pub fn random_g2_add(pool_size: usize) -> InputPool {
    let mut r = rng();
    let mut pool = InputPool::with_capacity(pool_size);
    for _ in 0..pool_size {
        let p1 = G2Projective::rand(&mut r);
        let p2 = G2Projective::rand(&mut r);
        let mut le = Vec::with_capacity(256);
        le.extend_from_slice(&g2_le(p1));
        le.extend_from_slice(&g2_le(p2));
        pool.be.push(reverse_chunks(&le, 64));
        pool.le.push(le);
    }
    pool
}

pub fn random_g2_mul(pool_size: usize) -> InputPool {
    let mut r = rng();
    let mut pool = InputPool::with_capacity(pool_size);
    for _ in 0..pool_size {
        let p = G2Projective::rand(&mut r);
        let s = Fr::rand(&mut r);
        let mut le = Vec::with_capacity(160);
        le.extend_from_slice(&g2_le(p));
        le.extend_from_slice(&fr_le(s));
        let mut be = Vec::with_capacity(160);
        be.extend_from_slice(&reverse_chunks(&le[..128], 64));
        be.extend_from_slice(&reverse_chunks(&le[128..], 32));
        pool.be.push(be);
        pool.le.push(le);
    }
    pool
}

/// Build a pool of `pool_size` inputs, each containing `n` pairs whose pairing
/// product equals the Fq12 identity (output byte = `0x01`). Requires even
/// `n >= 2`. Construction: pairs come in canceling twos `(P, Q), (-P, Q)` so
/// that `e(P,Q) · e(-P,Q) = e(O,Q) = 1`.
///
/// Mirrors real on-chain workloads (BLS sig verify, Groth16 verify) where the
/// host inputs are valid relations and the syscall is expected to return 1.
pub fn random_pairing(pool_size: usize, n: usize) -> InputPool {
    assert!(n >= 2 && n.is_multiple_of(2), "n must be even and >= 2");
    let mut r = rng();
    let mut pool = InputPool::with_capacity(pool_size);
    for _ in 0..pool_size {
        let mut le = Vec::with_capacity(192 * n);
        for _ in 0..(n / 2) {
            let p = G1Projective::rand(&mut r);
            let q = G2Projective::rand(&mut r);
            let neg_p = -p;
            for g1 in [p, neg_p] {
                le.extend_from_slice(&g1_le(g1));
                le.extend_from_slice(&g2_le(q));
            }
        }
        let mut be = Vec::with_capacity(192 * n);
        for pair in le.chunks_exact(192) {
            be.extend_from_slice(&reverse_chunks(&pair[..64], 32));
            be.extend_from_slice(&reverse_chunks(&pair[64..], 64));
        }
        pool.be.push(be);
        pool.le.push(le);
    }
    pool
}
