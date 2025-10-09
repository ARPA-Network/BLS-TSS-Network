use alloy::eips::eip1559::Eip1559Estimation;
use alloy::eips::eip4844::BLOB_TX_MIN_BLOB_GASPRICE;
use alloy::eips::BlockNumberOrTag;
use alloy::network::{Network, TransactionBuilder, TransactionBuilder4844};
use alloy::primitives::utils::parse_units;
use alloy::primitives::U256;
use alloy::providers::fillers::{FillerControlFlow, TxFiller};
use alloy::providers::{Provider, SendableTx};
use alloy::transports::{RpcError, TransportResult};
use futures_util::FutureExt;
use log::info;
use std::future::IntoFuture;
use thiserror::Error;

/// An enum over the different types of gas fillable.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GasFillable {
    Legacy {
        gas_limit: u64,
        gas_price: u128,
    },
    Eip1559 {
        gas_limit: u64,
        estimate: Eip1559Estimation,
    },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GasMiddleware {
    /// This value is used to raise the gas value before sending transactions
    contingency: U256,
}

/// Contingency is expressed with 4 units
/// e.g.
/// 50% => 1 + 0.5  => 15000
/// 20% => 1 + 0.2  => 12000
/// 1%  => 1 + 0.01 => 10100
const CONTINGENCY_UNITS: u8 = 4;

#[derive(Error, Debug)]
pub enum GasMiddlewareError {
    // /// Thrown when the internal middleware errors
    // #[error("{0}")]
    // MiddlewareError(String),
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

impl GasMiddleware {
    /// Creates an instance of GasMiddleware
    /// `ìnner` the inner Middleware
    /// `perc` This is an unsigned integer representing the percentage increase in the amount of gas
    /// to be used for the transaction. The percentage is relative to the gas value specified in the
    /// transaction. Valid contingency values are in range 1..=50. Otherwise a custom middleware
    /// error is raised.
    pub fn new(perc: u32) -> Result<Self, GasMiddlewareError> {
        let contingency = match perc {
            0 => Err(GasMiddlewareError::TooLowContingency(perc))?,
            51.. => Err(GasMiddlewareError::TooHighContingency(perc))?,
            1..=50 => {
                let decimals = 2;
                let perc = U256::from(perc) * U256::from(10).pow(U256::from(decimals)); // e.g. 50 => 5000
                let one: U256 = parse_units("1", CONTINGENCY_UNITS).unwrap().into();
                one + perc // e.g. 50% => 1 + 0.5 => 10000 + 5000 => 15000
            }
        };

        Ok(Self { contingency })
    }

    fn raise_gas_limit(&self, gas_limit: u64) -> u64 {
        info!("Original transaction gas: {gas_limit:?} wei");
        let units: U256 = U256::from(10).pow(U256::from(CONTINGENCY_UNITS));
        let raised_gas_limit: U256 = (U256::from(gas_limit) * self.contingency) / units;
        info!("Raised transaction gas: {raised_gas_limit:?} wei");
        raised_gas_limit.to::<u64>()
    }

    async fn prepare_legacy<P, N>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<GasFillable>
    where
        P: Provider<N>,
        N: Network,
    {
        let gas_price_fut = tx.gas_price().map_or_else(
            || provider.get_gas_price().right_future(),
            |gas_price| async move { Ok(gas_price) }.left_future(),
        );

        let gas_limit_fut = tx.gas_limit().map_or_else(
            || {
                provider
                    .estimate_gas(tx.clone())
                    .into_future()
                    .right_future()
            },
            |gas_limit| async move { Ok(gas_limit) }.left_future(),
        );

        let (gas_price, gas_limit) = futures_util::try_join!(gas_price_fut, gas_limit_fut)?;

        Ok(GasFillable::Legacy {
            gas_limit,
            gas_price,
        })
    }

    async fn prepare_1559<P, N>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<GasFillable>
    where
        P: Provider<N>,
        N: Network,
    {
        let gas_limit_fut = tx.gas_limit().map_or_else(
            || {
                provider
                    .estimate_gas(tx.clone())
                    .into_future()
                    .right_future()
            },
            |gas_limit| async move { Ok(gas_limit) }.left_future(),
        );

        let eip1559_fees_fut = if let (Some(max_fee_per_gas), Some(max_priority_fee_per_gas)) =
            (tx.max_fee_per_gas(), tx.max_priority_fee_per_gas())
        {
            async move {
                Ok(Eip1559Estimation {
                    max_fee_per_gas,
                    max_priority_fee_per_gas,
                })
            }
            .left_future()
        } else {
            provider.estimate_eip1559_fees().right_future()
        };

        let (gas_limit, estimate) = futures_util::try_join!(gas_limit_fut, eip1559_fees_fut)?;

        Ok(GasFillable::Eip1559 {
            gas_limit,
            estimate,
        })
    }
}

impl<N: Network> TxFiller<N> for GasMiddleware {
    type Fillable = GasFillable;

    fn status(&self, tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        // legacy and eip2930 tx
        if tx.gas_price().is_some() && tx.gas_limit().is_some() {
            return FillerControlFlow::Finished;
        }

        // eip1559
        if tx.max_fee_per_gas().is_some()
            && tx.max_priority_fee_per_gas().is_some()
            && tx.gas_limit().is_some()
        {
            return FillerControlFlow::Finished;
        }

        FillerControlFlow::Ready
    }

    fn fill_sync(&self, _tx: &mut SendableTx<N>) {}

    async fn prepare<P>(
        &self,
        provider: &P,
        tx: &<N as Network>::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<N>,
    {
        if tx.gas_price().is_some() {
            self.prepare_legacy(provider, tx).await
        } else {
            match self.prepare_1559(provider, tx).await {
                // fallback to legacy
                Ok(estimate) => Ok(estimate),
                Err(RpcError::UnsupportedFeature(_)) => self.prepare_legacy(provider, tx).await,
                Err(e) => Err(e),
            }
        }
    }

    async fn fill(
        &self,
        fillable: Self::Fillable,
        mut tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        if let Some(builder) = tx.as_mut_builder() {
            match fillable {
                GasFillable::Legacy {
                    gas_limit,
                    gas_price,
                } => {
                    builder.set_gas_limit(self.raise_gas_limit(gas_limit));
                    builder.set_gas_price(gas_price);
                }
                GasFillable::Eip1559 {
                    gas_limit,
                    estimate,
                } => {
                    builder.set_gas_limit(self.raise_gas_limit(gas_limit));
                    builder.set_max_fee_per_gas(estimate.max_fee_per_gas);
                    builder.set_max_priority_fee_per_gas(estimate.max_priority_fee_per_gas);
                }
            }
        };
        Ok(tx)
    }
}

/// Filler for the `max_fee_per_blob_gas` field in EIP-4844 transactions.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlobGasFiller;

impl<N: Network> TxFiller<N> for BlobGasFiller
where
    N::TransactionRequest: TransactionBuilder4844,
{
    type Fillable = u128;

    fn status(&self, tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        // Nothing to fill if non-eip4844 tx or `max_fee_per_blob_gas` is already set to a valid
        // value.
        if tx.blob_sidecar().is_none()
            || tx
                .max_fee_per_blob_gas()
                .is_some_and(|gas| gas >= BLOB_TX_MIN_BLOB_GASPRICE)
        {
            return FillerControlFlow::Finished;
        }

        FillerControlFlow::Ready
    }

    fn fill_sync(&self, _tx: &mut SendableTx<N>) {}

    async fn prepare<P>(
        &self,
        provider: &P,
        tx: &<N as Network>::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<N>,
    {
        if let Some(max_fee_per_blob_gas) = tx.max_fee_per_blob_gas() {
            if max_fee_per_blob_gas >= BLOB_TX_MIN_BLOB_GASPRICE {
                return Ok(max_fee_per_blob_gas);
            }
        }

        provider
            .get_fee_history(2, BlockNumberOrTag::Latest, &[])
            .await?
            .base_fee_per_blob_gas
            .last()
            .ok_or(RpcError::NullResp)
            .copied()
    }

    async fn fill(
        &self,
        fillable: Self::Fillable,
        mut tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        if let Some(builder) = tx.as_mut_builder() {
            builder.set_max_fee_per_blob_gas(fillable);
        }
        Ok(tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::consensus::{SidecarBuilder, SimpleCoder, Transaction};
    use alloy::eips::eip4844::DATA_GAS_PER_BLOB;
    use alloy::primitives::{address, U256};
    use alloy::providers::ProviderBuilder;
    use alloy::rpc::types::TransactionRequest;

    #[tokio::test]
    async fn no_gas_price_or_limit() {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        // GasEstimationLayer requires chain_id to be set to handle EIP-1559 tx
        let tx = TransactionRequest {
            value: Some(U256::from(100)),
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
            chain_id: Some(31337),
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let receipt = tx.get_receipt().await.unwrap();

        assert_eq!(receipt.effective_gas_price, 1_000_000_001);
        assert_eq!(receipt.gas_used, 21000);
    }

    #[tokio::test]
    async fn no_gas_limit() {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        let gas_price = provider.get_gas_price().await.unwrap();
        let tx = TransactionRequest {
            value: Some(U256::from(100)),
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
            gas_price: Some(gas_price),
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let receipt = tx.get_receipt().await.unwrap();

        assert_eq!(receipt.gas_used, 21000);
    }

    #[tokio::test]
    async fn no_max_fee_per_blob_gas() {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        let sidecar: SidecarBuilder<SimpleCoder> = SidecarBuilder::from_slice(b"Hello World");
        let sidecar = sidecar.build().unwrap();

        let tx = TransactionRequest {
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
            sidecar: Some(sidecar),
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let receipt = tx.get_receipt().await.unwrap();

        let tx = provider
            .get_transaction_by_hash(receipt.transaction_hash)
            .await
            .unwrap()
            .unwrap();

        assert!(tx.max_fee_per_blob_gas().unwrap() >= BLOB_TX_MIN_BLOB_GASPRICE);
        assert_eq!(receipt.gas_used, 21000);
        assert_eq!(
            receipt
                .blob_gas_used
                .expect("Expected to be EIP-4844 transaction"),
            DATA_GAS_PER_BLOB
        );
    }

    #[tokio::test]
    async fn zero_max_fee_per_blob_gas() {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        let sidecar: SidecarBuilder<SimpleCoder> = SidecarBuilder::from_slice(b"Hello World");
        let sidecar = sidecar.build().unwrap();

        let tx = TransactionRequest {
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
            max_fee_per_blob_gas: Some(0),
            sidecar: Some(sidecar),
            ..Default::default()
        };

        let tx = provider.send_transaction(tx).await.unwrap();

        let receipt = tx.get_receipt().await.unwrap();

        let tx = provider
            .get_transaction_by_hash(receipt.transaction_hash)
            .await
            .unwrap()
            .unwrap();

        assert!(tx.max_fee_per_blob_gas().unwrap() >= BLOB_TX_MIN_BLOB_GASPRICE);
        assert_eq!(receipt.gas_used, 21000);
        assert_eq!(
            receipt
                .blob_gas_used
                .expect("Expected to be EIP-4844 transaction"),
            DATA_GAS_PER_BLOB
        );
    }
}
