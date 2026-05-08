#![cfg(feature = "agave-unstable-api")]
#![allow(clippy::arithmetic_side_effects)]

pub use crate::{
    encoding::{ELL_COEFFS_PER_PREPARED_G2, PodG1, PodGt, PodPreparedG2},
    pairing::{alt_bn128_pairing_prepared, prepare_g2},
};

pub(crate) mod encoding;
pub(crate) mod pairing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    V0,
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum PreparedPairingError {
    #[error("G1 point not on curve")]
    InvalidG1,
    #[error("final exponentiation failed (degenerate input)")]
    FinalExponentiationFailed,
}
