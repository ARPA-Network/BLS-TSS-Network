use thiserror::Error;

/// Wrappers around the BLS12-381 curve from the [paired](http://docs.rs/paired) crate
#[cfg(feature = "bls12_381")]
pub mod bls12381;

/// Error which unifies all curve specific errors from different libraries
#[derive(Debug, Error)]
pub enum CurveError {
    #[cfg(feature = "bls12_381")]
    #[error("Bellman Error: {0}")]
    BLS12_381(bls12381::BellmanError),
}
