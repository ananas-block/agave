use bytemuck_derive::{Pod, Zeroable as DeriveZeroable};

pub const FQ_LIMBS: usize = 4;
pub(crate) const FQ2_LIMBS: usize = 2 * FQ_LIMBS;
pub const ELL_COEFF_LIMBS: usize = 3 * FQ2_LIMBS;
pub(crate) const FQ12_LIMBS: usize = 12 * FQ_LIMBS;

pub const ELL_COEFFS_PER_PREPARED_G2: usize = 87;

#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, DeriveZeroable)]
pub struct PodG1(pub [u64; 2 * FQ_LIMBS]);

#[repr(C, align(8))]
#[derive(Clone, Copy, Pod, DeriveZeroable)]
pub struct PodPreparedG2(pub [[u64; ELL_COEFF_LIMBS]; ELL_COEFFS_PER_PREPARED_G2]);

impl core::fmt::Debug for PodPreparedG2 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PodPreparedG2(<{} limbs>)", FQ_LIMBS * 6 * ELL_COEFFS_PER_PREPARED_G2)
    }
}

#[repr(C, align(8))]
#[derive(Clone, Copy, PartialEq, Eq, Pod, DeriveZeroable)]
pub struct PodGt(pub [u64; FQ12_LIMBS]);

impl core::fmt::Debug for PodGt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PodGt(<{} limbs>)", FQ12_LIMBS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pod_sizes() {
        assert_eq!(core::mem::size_of::<PodG1>(), 64);
        assert_eq!(core::mem::size_of::<PodPreparedG2>(), 16_704);
        assert_eq!(core::mem::size_of::<PodGt>(), 384);
    }
}
