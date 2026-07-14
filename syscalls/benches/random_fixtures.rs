#![allow(dead_code)]

use {
    ark_bn254::{Fr, G1Projective, G2Projective},
    ark_ec::{AffineRepr, CurveGroup},
    ark_ff::UniformRand,
    ark_serialize::{CanonicalSerialize, Compress},
    ark_std::rand::{rngs::StdRng, SeedableRng},
};

const SEED: u64 = 0xa17b428;

pub const POOL: usize = 1024;
pub const PAIRING_POOL: usize = 1024;

fn rng() -> StdRng {
    StdRng::seed_from_u64(SEED)
}

// Field-by-field LE serialization matching solana-bn254's `PodG1` /
// `PodG2` byte format (the canonical format that both the arkworks
// pairing path and the gnark FFI accept). 64 zero bytes (resp. 128) for
// the affine identity.
fn g1_le(p: G1Projective) -> [u8; 64] {
    let mut buf = [0u8; 64];
    let aff = p.into_affine();
    if let Some((x, y)) = aff.xy() {
        x.serialize_with_mode(&mut buf[..32], Compress::No)
            .expect("Fq.x");
        y.serialize_with_mode(&mut buf[32..], Compress::No)
            .expect("Fq.y");
    }
    buf
}

fn g2_le(p: G2Projective) -> [u8; 128] {
    let mut buf = [0u8; 128];
    let aff = p.into_affine();
    if let Some((x, y)) = aff.xy() {
        x.serialize_with_mode(&mut buf[..64], Compress::No)
            .expect("Fq2.x");
        y.serialize_with_mode(&mut buf[64..], Compress::No)
            .expect("Fq2.y");
    }
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

pub struct PreparedPairingPool {
    pub g1s: Vec<Vec<solana_bn254_prepared_syscall::PodG1>>,
    pub g2_preps: Vec<Vec<solana_bn254_prepared_syscall::PodPreparedG2>>,
}

impl PreparedPairingPool {
    fn with_capacity(n: usize) -> Self {
        Self {
            g1s: Vec::with_capacity(n),
            g2_preps: Vec::with_capacity(n),
        }
    }
}

pub fn random_pairing_prepared(pool_size: usize, n: usize) -> PreparedPairingPool {
    use solana_bn254_prepared_syscall::{prepare_g2, PodG1, PodPreparedG2};

    fn pod_g1(g: &ark_bn254::G1Affine) -> PodG1 {
        let mut limbs = [0u64; 8];
        if !g.infinity {
            let (x, y) = g.xy().expect("non-infinity has xy");
            limbs[0..4].copy_from_slice(&x.0 .0);
            limbs[4..8].copy_from_slice(&y.0 .0);
        }
        PodG1(limbs)
    }

    let mut r = rng();
    let mut pool = PreparedPairingPool::with_capacity(pool_size);
    for _ in 0..pool_size {
        let mut g1s: Vec<PodG1> = Vec::with_capacity(n);
        let mut g2_preps: Vec<PodPreparedG2> = Vec::with_capacity(n);
        for _ in 0..n {
            let g1 = G1Projective::rand(&mut r).into_affine();
            let g2 = G2Projective::rand(&mut r).into_affine();
            g1s.push(pod_g1(&g1));
            g2_preps.push(prepare_g2(&g2));
        }
        pool.g1s.push(g1s);
        pool.g2_preps.push(g2_preps);
    }
    pool
}

/// Pool of gnark-prepared pairing fixtures. Each entry holds, for `n` pairs,
/// the concatenated G1 LE bytes (`n * 64` total) and the concatenated
/// gnark-prepared G2 lines blobs (`n * 16_896` total). The 1:1 contract
/// between G1[i] and lines[i] is preserved by construction.
pub struct GnarkPreparedPairingPool {
    pub g1s: Vec<Vec<u8>>,
    pub lines: Vec<Vec<u8>>,
}

impl GnarkPreparedPairingPool {
    fn with_capacity(n: usize) -> Self {
        Self {
            g1s: Vec::with_capacity(n),
            lines: Vec::with_capacity(n),
        }
    }
}

/// Build `pool_size` pools of `n` random `(G1, G2)` pairs and precompute the
/// gnark-side `[2][66]LineEvaluationAff` blob for each fixed G2. Every G1
/// gets its OWN freshly-precomputed prepared blob (no sharing across pairs)
/// to match the FFI's strict 1:1 contract.
pub fn random_pairing_gnark_prepared(pool_size: usize, n: usize) -> GnarkPreparedPairingPool {
    use solana_bn254_gnark::sizes::{G1_POINT, G2_POINT, PREPARED_G2};

    let mut r = rng();
    let mut pool = GnarkPreparedPairingPool::with_capacity(pool_size);
    for _ in 0..pool_size {
        let mut g1s_concat = Vec::with_capacity(G1_POINT * n);
        let mut lines_concat = Vec::with_capacity(PREPARED_G2 * n);
        for _ in 0..n {
            let g1 = G1Projective::rand(&mut r);
            let g2 = G2Projective::rand(&mut r);
            g1s_concat.extend_from_slice(&g1_le(g1));
            let g2_bytes = g2_le(g2);
            debug_assert_eq!(g2_bytes.len(), G2_POINT);
            let mut blob = vec![0u8; PREPARED_G2];
            let code = solana_bn254_gnark::g2_precompute_lines(&g2_bytes, &mut blob);
            assert_eq!(code, 0, "g2_precompute_lines returned {code}");
            lines_concat.extend_from_slice(&blob);
        }
        pool.g1s.push(g1s_concat);
        pool.lines.push(lines_concat);
    }
    pool
}

pub fn random_pairing(pool_size: usize, n: usize) -> InputPool {
    assert!(n >= 2, "n must be >= 2");
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
        if !n.is_multiple_of(2) {
            let q = G2Projective::rand(&mut r);
            le.extend_from_slice(&[0u8; 64]);
            le.extend_from_slice(&g2_le(q));
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
