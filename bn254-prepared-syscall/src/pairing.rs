use {
    crate::{
        encoding::{ELL_COEFFS_PER_PREPARED_G2, ELL_COEFF_LIMBS, PodG1, PodGt, PodPreparedG2},
        PreparedPairingError, Version,
    },
    ark_bn254::{Bn254, Fq, Fq12, G1Affine, G2Affine},
    ark_ec::{
        bn::{BnConfig, G2Prepared, TwistType},
        pairing::{MillerLoopOutput, Pairing},
    },
    ark_ff::{BigInt, CyclotomicMultSubgroup, Field, One, Zero},
};

type EllCoeff = ark_ec::bn::g2::EllCoeff<ark_bn254::Config>;

pub fn prepare_g2(g2: &G2Affine) -> PodPreparedG2 {
    let prep: G2Prepared<ark_bn254::Config> = (*g2).into();
    assert!(!prep.infinity);
    assert_eq!(prep.ell_coeffs.len(), ELL_COEFFS_PER_PREPARED_G2);
    let mut out = [[0u64; ELL_COEFF_LIMBS]; ELL_COEFFS_PER_PREPARED_G2];
    for (slot, coeff) in out.iter_mut().zip(prep.ell_coeffs.iter()) {
        let src: &[u64; ELL_COEFF_LIMBS] =
            unsafe { &*(coeff as *const EllCoeff as *const [u64; ELL_COEFF_LIMBS]) };
        *slot = *src;
    }
    PodPreparedG2(out)
}

pub fn alt_bn128_pairing_prepared(
    _version: Version,
    g1s: &[PodG1],
    g2_preps: &[PodPreparedG2],
) -> Result<PodGt, PreparedPairingError> {
    debug_assert_eq!(g1s.len(), g2_preps.len());
    if g1s.is_empty() {
        return Ok(fq12_to_pod(&Fq12::one()));
    }

    for pod_g1 in g1s {
        let (x, y) = pod_g1_xy(pod_g1);
        if x.is_zero() && y.is_zero() {
            continue;
        }
        if !G1Affine::new_unchecked(x, y).is_on_curve() {
            return Err(PreparedPairingError::InvalidG1);
        }
    }

    let g2_coeffs: &[[EllCoeff; ELL_COEFFS_PER_PREPARED_G2]] = unsafe {
        core::slice::from_raw_parts(
            g2_preps.as_ptr() as *const [EllCoeff; ELL_COEFFS_PER_PREPARED_G2],
            g2_preps.len(),
        )
    };

    let mut f = Fq12::one();
    let loop_count = <ark_bn254::Config as BnConfig>::ATE_LOOP_COUNT;
    let mut idx = 0usize;

    for i in (1..loop_count.len()).rev() {
        if i != loop_count.len() - 1 {
            f.square_in_place();
        }
        for (pod_g1, coeffs) in g1s.iter().zip(g2_coeffs.iter()) {
            ell(&mut f, &coeffs[idx], pod_g1);
        }
        idx += 1;
        let bit = loop_count[i - 1];
        if bit == 1 || bit == -1 {
            for (pod_g1, coeffs) in g1s.iter().zip(g2_coeffs.iter()) {
                ell(&mut f, &coeffs[idx], pod_g1);
            }
            idx += 1;
        }
    }

    if <ark_bn254::Config as BnConfig>::X_IS_NEGATIVE {
        f.cyclotomic_inverse_in_place();
    }

    for (pod_g1, coeffs) in g1s.iter().zip(g2_coeffs.iter()) {
        ell(&mut f, &coeffs[idx], pod_g1);
    }
    idx += 1;
    for (pod_g1, coeffs) in g1s.iter().zip(g2_coeffs.iter()) {
        ell(&mut f, &coeffs[idx], pod_g1);
    }
    debug_assert_eq!(idx + 1, ELL_COEFFS_PER_PREPARED_G2);

    let gt = Bn254::final_exponentiation(MillerLoopOutput(f))
        .ok_or(PreparedPairingError::FinalExponentiationFailed)?
        .0;
    Ok(fq12_to_pod(&gt))
}

#[inline]
fn pod_g1_xy(p: &PodG1) -> (Fq, Fq) {
    let x = Fq::new_unchecked(BigInt::new([p.0[0], p.0[1], p.0[2], p.0[3]]));
    let y = Fq::new_unchecked(BigInt::new([p.0[4], p.0[5], p.0[6], p.0[7]]));
    (x, y)
}

#[inline]
fn ell(f: &mut Fq12, coeffs: &EllCoeff, p: &PodG1) {
    let (x, y) = pod_g1_xy(p);
    if x.is_zero() && y.is_zero() {
        return;
    }
    let mut c0 = coeffs.0;
    let mut c1 = coeffs.1;
    let c2 = coeffs.2;
    match <ark_bn254::Config as BnConfig>::TWIST_TYPE {
        TwistType::M => {
            let mut c2 = c2;
            c2.mul_assign_by_fp(&y);
            c1.mul_assign_by_fp(&x);
            f.mul_by_014(&c0, &c1, &c2);
        }
        TwistType::D => {
            c0.mul_assign_by_fp(&y);
            c1.mul_assign_by_fp(&x);
            f.mul_by_034(&c0, &c1, &c2);
        }
    }
}

#[inline]
fn fq12_to_pod(f: &Fq12) -> PodGt {
    unsafe { *(f as *const Fq12 as *const PodGt) }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        ark_ec::{pairing::PairingOutput, AffineRepr, CurveGroup, PrimeGroup},
        ark_ff::UniformRand,
        ark_std::test_rng,
    };

    fn pod_g1_from_affine(g: &G1Affine) -> PodG1 {
        let mut limbs = [0u64; 8];
        if !g.is_zero() {
            let (x, y) = g.xy().unwrap();
            limbs[0..4].copy_from_slice(&x.0 .0);
            limbs[4..8].copy_from_slice(&y.0 .0);
        }
        PodG1(limbs)
    }

    fn random_pairs(n: usize) -> (Vec<G1Affine>, Vec<G2Affine>) {
        let mut rng = test_rng();
        let g1g = ark_bn254::G1Projective::generator();
        let g2g = ark_bn254::G2Projective::generator();
        let g1s: Vec<G1Affine> = (0..n)
            .map(|_| (g1g * ark_bn254::Fr::rand(&mut rng)).into_affine())
            .collect();
        let g2s: Vec<G2Affine> = (0..n)
            .map(|_| (g2g * ark_bn254::Fr::rand(&mut rng)).into_affine())
            .collect();
        (g1s, g2s)
    }

    fn fq12_from_pod(p: &PodGt) -> Fq12 {
        unsafe { *(p as *const PodGt as *const Fq12) }
    }

    #[test]
    fn empty_returns_one() {
        let result = alt_bn128_pairing_prepared(Version::V0, &[], &[]).unwrap();
        assert_eq!(fq12_from_pod(&result), Fq12::one());
    }

    #[test]
    fn matches_ark_multi_pairing() {
        let (g1s, g2s) = random_pairs(4);
        let pod_g1s: Vec<PodG1> = g1s.iter().map(pod_g1_from_affine).collect();
        let pod_g2s: Vec<PodPreparedG2> = g2s.iter().map(prepare_g2).collect();

        let our = fq12_from_pod(
            &alt_bn128_pairing_prepared(Version::V0, &pod_g1s, &pod_g2s).unwrap(),
        );

        let expected: PairingOutput<Bn254> =
            Bn254::multi_pairing(g1s.iter().copied(), g2s.iter().copied());
        assert_eq!(our, expected.0);
    }

    #[test]
    fn bilinearity_returns_gt_identity() {
        let mut rng = test_rng();
        let a = ark_bn254::Fr::rand(&mut rng);
        let p = ark_bn254::G1Projective::generator() * ark_bn254::Fr::rand(&mut rng);
        let q = ark_bn254::G2Projective::generator() * ark_bn254::Fr::rand(&mut rng);
        let ap = (p * a).into_affine();
        let p = p.into_affine();
        let q_neg_a = -((q * a).into_affine());
        let q = q.into_affine();

        let pod_g1s = vec![pod_g1_from_affine(&ap), pod_g1_from_affine(&p)];
        let pod_g2s = vec![prepare_g2(&q), prepare_g2(&q_neg_a)];
        let result = fq12_from_pod(
            &alt_bn128_pairing_prepared(Version::V0, &pod_g1s, &pod_g2s).unwrap(),
        );
        assert_eq!(result, Fq12::one());
    }

    #[test]
    fn ark_layout_matches_pods() {
        assert_eq!(core::mem::size_of::<Fq12>(), core::mem::size_of::<PodGt>());
        assert_eq!(core::mem::align_of::<Fq12>(), core::mem::align_of::<PodGt>());
        assert_eq!(core::mem::size_of::<EllCoeff>(), ELL_COEFF_LIMBS * 8);
        assert_eq!(core::mem::align_of::<EllCoeff>(), 8);

        let f: Fq12 = Fq12::one() + Fq12::one();
        let pod = fq12_to_pod(&f);
        let back: Fq12 = unsafe { *(&pod as *const PodGt as *const Fq12) };
        assert_eq!(back, f);
    }

    #[test]
    fn rejects_g1_off_curve() {
        let mut limbs = [0u64; 8];
        limbs[0] = 1;
        limbs[4] = 1;
        let g2 = ark_bn254::G2Projective::generator().into_affine();
        let pod_g2 = prepare_g2(&g2);
        assert_eq!(
            alt_bn128_pairing_prepared(Version::V0, &[PodG1(limbs)], &[pod_g2]),
            Err(PreparedPairingError::InvalidG1)
        );
    }
}
