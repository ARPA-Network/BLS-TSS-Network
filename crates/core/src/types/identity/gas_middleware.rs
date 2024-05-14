use async_trait::async_trait;
use ethers_core::{
    types::{transaction::eip2718::TypedTransaction, BlockId, U256},
    utils::parse_units,
};
use ethers_providers::{Middleware, MiddlewareError};
use log::info;
use thiserror::Error;

/// (Modified on https://github.com/gakonst/ethers-rs/blob/51fe937f6515689b17a3a83b74a05984ad3a7f11/examples/middleware/examples/create_custom_middleware.rs#L23)
/// This custom middleware increases the gas value of transactions sent through an ethers-rs
/// provider by a specified percentage and will be called for each transaction before it is sent.
/// This can be useful if you want to ensure that transactions have a higher gas value than the
/// estimated, in order to improve the chances of them not to run out of gas when landing on-chain.
#[derive(Debug)]
pub struct GasMiddleware<M> {
    inner: M,
    /// This value is used to raise the gas value before sending transactions
    contingency: U256,
}

/// Contingency is expressed with 4 units
/// e.g.
/// 50% => 1 + 0.5  => 15000
/// 20% => 1 + 0.2  => 12000
/// 1%  => 1 + 0.01 => 10100
const CONTINGENCY_UNITS: usize = 4;

impl<M> GasMiddleware<M>
where
    M: Middleware,
{
    /// Creates an instance of GasMiddleware
    /// `ìnner` the inner Middleware
    /// `perc` This is an unsigned integer representing the percentage increase in the amount of gas
    /// to be used for the transaction. The percentage is relative to the gas value specified in the
    /// transaction. Valid contingency values are in range 1..=50. Otherwise a custom middleware
    /// error is raised.
    pub fn new(inner: M, perc: u32) -> Result<Self, GasMiddlewareError<M>> {
        let contingency = match perc {
            0 => Err(GasMiddlewareError::TooLowContingency(perc))?,
            51.. => Err(GasMiddlewareError::TooHighContingency(perc))?,
            1..=50 => {
                let decimals = 2;
                let perc = U256::from(perc) * U256::exp10(decimals); // e.g. 50 => 5000
                let one = parse_units(1, CONTINGENCY_UNITS).unwrap();
                let one = U256::from(one);
                one + perc // e.g. 50% => 1 + 0.5 => 10000 + 5000 => 15000
            }
        };

        Ok(Self { inner, contingency })
    }
}

/// Let's implement the `Middleware` trait for our custom middleware.
/// All trait functions are derived automatically, so we just need to
/// override the needed functions.
#[async_trait]
impl<M> Middleware for GasMiddleware<M>
where
    M: Middleware,
{
    type Error = GasMiddlewareError<M>;
    type Provider = M::Provider;
    type Inner = M;

    fn inner(&self) -> &M {
        &self.inner
    }

    async fn fill_transaction(
        &self,
        tx: &mut TypedTransaction,
        block: Option<BlockId>,
    ) -> Result<(), Self::Error> {
        // Delegate the call to the inner middleware to get an estimate of the gas
        self.inner()
            .fill_transaction(tx, block)
            .await
            .map_err(MiddlewareError::from_err)?;

        let curr_gas: U256 = match tx.gas() {
            Some(gas) => gas.to_owned(),
            None => Err(GasMiddlewareError::NoGasSetForTransaction)?,
        };

        info!("Original transaction gas: {curr_gas:?} wei");
        let units: U256 = U256::exp10(CONTINGENCY_UNITS);
        let raised_gas: U256 = (curr_gas * self.contingency) / units;
        tx.set_gas(raised_gas);
        info!("Raised transaction gas: {raised_gas:?} wei");

        Ok(())
    }
}

/// This example demonstrates how to handle errors in custom middlewares. It shows how to define
/// custom error types, use them in middleware implementations, and how to propagate the errors
/// through the middleware chain. This is intended for developers who want to create custom
/// middlewares that can handle and propagate errors in a consistent and robust way.
#[derive(Error, Debug)]
pub enum GasMiddlewareError<M: Middleware> {
    /// Thrown when the internal middleware errors
    #[error("{0}")]
    MiddlewareError(M::Error),
    /// Specific errors of this GasMiddleware.
    /// Please refer to the `thiserror` crate for
    /// further docs.
    #[error("{0}")]
    TooHighContingency(u32),
    #[error("{0}")]
    TooLowContingency(u32),
    #[error("Cannot raise gas! Gas value not provided for this transaction.")]
    NoGasSetForTransaction,
}

impl<M: Middleware> MiddlewareError for GasMiddlewareError<M> {
    type Inner = M::Error;

    fn from_err(src: M::Error) -> Self {
        GasMiddlewareError::MiddlewareError(src)
    }

    fn as_inner(&self) -> Option<&Self::Inner> {
        match self {
            GasMiddlewareError::MiddlewareError(e) => Some(e),
            _ => None,
        }
    }
}
