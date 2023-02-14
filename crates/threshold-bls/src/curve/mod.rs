use thiserror::Error;

use crate::group::{Curve, PairingCurve};

/// Wrappers around the BLS12-381 curve from the [paired](http://docs.rs/paired) crate
#[cfg(feature = "bls12_381")]
pub mod bls12381;

#[cfg(feature = "bn254")]
pub mod bn254;

/// Error which unifies all curve specific errors from different libraries
#[derive(Debug, Error)]
pub enum CurveError {
    #[cfg(feature = "bls12_381")]
    #[error("Bellman Error: {0}")]
    BLS12_381(bls12381::BLS12Error),

    #[cfg(feature = "bn254")]
    #[error("Bellman Error: {0}")]
    BN254(bn254::BNError),
}

#[derive(Debug, Error)]
/// Error type
pub enum BLSError {
    /// Error
    #[error("signature verification failed")]
    VerificationFailed,

    /// An IO error
    #[error("io error {0}")]
    IoError(#[from] std::io::Error),

    /// Error while hashing
    #[error("error in hasher {0}")]
    HashingError(#[from] Box<dyn std::error::Error>),

    /// Personalization string cannot be larger than 8 bytes
    #[error("domain length is too large: {0}")]
    DomainTooLarge(usize),

    /// Hashing to curve failed
    #[error("Could not hash to curve")]
    HashToCurveError,

    /// There must be the same number of keys and messages
    #[error("there must be the same number of keys and messages")]
    UnevenNumKeysMessages,

    /// Serialization error in Zexe
    #[error(transparent)]
    SerializationError(#[from] ark_serialize::SerializationError),
}

pub trait CurveType {
    type G1Curve: Curve;
    type G2Curve: Curve;
    type PairingCurve: PairingCurve;
}
