use crate::error::ContractClientError;
use ::ethers::abi::Detokenize;
use ::ethers::prelude::builders::ContractCall;
use ::ethers::prelude::ContractError;
use ::ethers::providers::{Middleware, ProviderError};
use ::ethers::types::{BlockNumber, TransactionReceipt, U64};
use arpa_core::{
    eip1559_gas_price_estimator, fallback_eip1559_gas_price_estimator, jitter, supports_eip1559,
    ExponentialBackoffRetryDescriptor,
};
use async_trait::async_trait;
use error::ContractClientResult;
use log::{error, info};
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::{Retry, RetryIf};

pub mod contract_stub;
pub mod error;
pub mod ethers;

#[async_trait]
pub trait ServiceClient<C> {
    async fn prepare_service_client(&self) -> ContractClientResult<C>;
}

#[async_trait]
pub trait TransactionCaller {
    async fn call_contract_transaction<
        M: Middleware,
        D: Detokenize + std::fmt::Debug + Send + Sync + 'static,
    >(
        chain_id: usize,
        info: &str,
        client: &M,
        mut call: ContractCall<M, D>,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        retry_on_transaction_fail: bool,
    ) -> ContractClientResult<TransactionReceipt>
    where
        ContractClientError: From<ContractError<M>>,
    {
        let retry_strategy =
            ExponentialBackoff::from_millis(contract_transaction_retry_descriptor.base)
                .factor(contract_transaction_retry_descriptor.factor)
                .map(|e| {
                    if contract_transaction_retry_descriptor.use_jitter {
                        jitter(e)
                    } else {
                        e
                    }
                })
                .take(contract_transaction_retry_descriptor.max_attempts);

        // transform the trx to legacy if the chain does not support EIP-1559
        if !supports_eip1559(chain_id) {
            call = call.legacy();
        }
        // set gas price for EIP-1559 trxs
        else if let Some(tx) = call.tx.as_eip1559_mut() {
            let (max_fee, max_priority_fee) = match client
                .estimate_eip1559_fees(Some(eip1559_gas_price_estimator))
                .await
            {
                // if max_priority_fee is zero, it usually means that the chain is a testnet,
                // we will use the legacy method to set a priority fee, to avoid the transaction being underpriced
                Ok((max_fee, max_priority_fee)) if !max_priority_fee.is_zero() => {
                    (max_fee, max_priority_fee)
                }
                _ => {
                    // try to estimate the gas price using the legacy method
                    let base_fee_per_gas = client
                        .get_block(BlockNumber::Latest)
                        .await
                        .map_err(ContractError::from_middleware_error)?
                        .ok_or_else(|| ProviderError::CustomError("Latest block not found".into()))?
                        .base_fee_per_gas
                        .ok_or_else(|| {
                            ProviderError::CustomError("EIP-1559 not activated".into())
                        })?;

                    let gas_price = client
                        .get_gas_price()
                        .await
                        .map_err(ContractError::from_middleware_error)?;

                    fallback_eip1559_gas_price_estimator(
                        base_fee_per_gas,
                        gas_price - base_fee_per_gas,
                    )
                }
            };
            tx.max_fee_per_gas = Some(max_fee);
            tx.max_priority_fee_per_gas = Some(max_priority_fee);
        }

        let transaction_receipt = RetryIf::spawn(
            retry_strategy,
            || async {
                let pending_tx = call.send().await.map_err(|e| {
                    let e: ContractClientError = e.into();
                    e
                })?;

                info!(
                    "Calling contract transaction {} with chain_id({}): {:?}",
                    info,
                    chain_id,
                    pending_tx.tx_hash()
                );

                let receipt = pending_tx
                    .await
                    .map_err(|e| {
                        let e: ContractClientError = e.into();
                        e
                    })?
                    .ok_or(ContractClientError::NoTransactionReceipt)?;

                if receipt.status == Some(U64::from(0)) {
                    error!(
                        "Transaction failed({}) with chain_id({}), receipt: {:?}",
                        info, chain_id, receipt
                    );
                    return Err(ContractClientError::TransactionFailed(receipt));
                } else {
                    info!(
                        "Transaction successful({}) with chain_id({}), receipt: {:?}",
                        info, chain_id, receipt
                    );
                }

                Ok(receipt)
            },
            |e: &ContractClientError| {
                retry_on_transaction_fail || !matches!(e, ContractClientError::TransactionFailed(_))
            },
        )
        .await?;

        Ok(transaction_receipt)
    }
}

#[async_trait]
pub trait ViewCaller {
    async fn call_contract_view<
        M: Middleware,
        D: Detokenize + std::fmt::Debug + Send + Sync + 'static,
    >(
        chain_id: usize,
        info: &str,
        call: ContractCall<M, D>,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    ) -> ContractClientResult<D>
    where
        ContractClientError: From<ContractError<M>>,
    {
        let retry_strategy = ExponentialBackoff::from_millis(contract_view_retry_descriptor.base)
            .factor(contract_view_retry_descriptor.factor)
            .map(|e| {
                if contract_view_retry_descriptor.use_jitter {
                    jitter(e)
                } else {
                    e
                }
            })
            .take(contract_view_retry_descriptor.max_attempts);

        let res = Retry::spawn(retry_strategy, || async {
            let result = call.call().await.map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;

            info!(
                "Calling contract view {} with chain_id({}), calldata: {:?}, result: {:?}",
                info,
                chain_id,
                call.calldata(),
                result
            );

            Result::<D, ContractClientError>::Ok(result)
        })
        .await?;

        Ok(res)
    }

    async fn call_contract_view_without_log<M: Middleware, D: Detokenize + Send + Sync + 'static>(
        call: ContractCall<M, D>,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    ) -> ContractClientResult<D>
    where
        ContractClientError: From<ContractError<M>>,
    {
        let retry_strategy = ExponentialBackoff::from_millis(contract_view_retry_descriptor.base)
            .factor(contract_view_retry_descriptor.factor)
            .map(|e| {
                if contract_view_retry_descriptor.use_jitter {
                    jitter(e)
                } else {
                    e
                }
            })
            .take(contract_view_retry_descriptor.max_attempts);

        let res = Retry::spawn(retry_strategy, || async {
            let result = call.call().await.map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;

            Result::<D, ContractClientError>::Ok(result)
        })
        .await?;

        Ok(res)
    }
}

pub mod node_registry {
    use crate::error::ContractClientResult;
    use arpa_core::Node;
    use async_trait::async_trait;
    use ethers::core::types::Address;
    use ethers::signers::LocalWallet;
    use ethers::types::TransactionReceipt;

    #[async_trait]
    pub trait NodeRegistryTransactions {
        async fn node_register_as_eigenlayer_operator(
            &self,
            id_public_key: Vec<u8>,
            asset_account_signer: &LocalWallet,
        ) -> ContractClientResult<TransactionReceipt>;

        async fn node_register_by_consistent_native_staking(
            &self,
            id_public_key: Vec<u8>,
        ) -> ContractClientResult<TransactionReceipt>;

        async fn node_activate_as_eigenlayer_operator(
            &self,
            asset_account_signer: &LocalWallet,
        ) -> ContractClientResult<TransactionReceipt>;

        async fn node_activate_by_consistent_native_staking(
            &self,
        ) -> ContractClientResult<TransactionReceipt>;
    }

    #[async_trait]
    pub trait NodeRegistryViews {
        async fn get_node(&self, id_address: Address) -> ContractClientResult<Node>;
    }

    pub trait NodeRegistryClientBuilder {
        type NodeRegistryService: NodeRegistryTransactions + NodeRegistryViews + Send + Sync;

        fn build_node_registry_client(
            &self,
            node_registry_address: Address,
        ) -> Self::NodeRegistryService;
    }
}

pub mod controller {
    use crate::error::ContractClientResult;
    use arpa_core::{DKGTask, Group};
    use async_trait::async_trait;
    use ethers::core::types::Address;
    use ethers::types::TransactionReceipt;
    use std::future::Future;
    use threshold_bls::group::Curve;

    #[async_trait]
    pub trait ControllerTransactions {
        async fn commit_dkg(
            &self,
            group_index: usize,
            group_epoch: usize,
            public_key: Vec<u8>,
            partial_public_key: Vec<u8>,
            disqualified_nodes: Vec<Address>,
        ) -> ContractClientResult<TransactionReceipt>;

        async fn post_process_dkg(
            &self,
            group_index: usize,
            group_epoch: usize,
        ) -> ContractClientResult<TransactionReceipt>;
    }

    #[async_trait]
    pub trait ControllerViews<C: Curve> {
        async fn get_group(&self, group_index: usize) -> ContractClientResult<Group<C>>;

        async fn get_coordinator(&self, group_index: usize) -> ContractClientResult<Address>;

        async fn get_node_registry_address(&self) -> ContractClientResult<Address>;
    }

    #[async_trait]
    pub trait ControllerLogs {
        async fn subscribe_dkg_task<
            C: FnMut(DKGTask) -> F + Send,
            F: Future<Output = ContractClientResult<()>> + Send,
        >(
            &self,
            cb: C,
        ) -> ContractClientResult<()>;
    }

    pub trait ControllerClientBuilder<C: Curve> {
        type ControllerService: ControllerTransactions
            + ControllerViews<C>
            + ControllerLogs
            + Send
            + Sync;

        fn build_controller_client(&self) -> Self::ControllerService;
    }
}

pub mod controller_oracle {
    use crate::error::ContractClientResult;
    use arpa_core::Group;
    use async_trait::async_trait;
    use ethers::types::{Address, TransactionReceipt};
    use threshold_bls::group::Curve;

    #[async_trait]
    pub trait ControllerOracleTransactions {
        async fn node_withdraw(
            &self,
            recipient: Address,
        ) -> ContractClientResult<TransactionReceipt>;
    }

    #[async_trait]
    pub trait ControllerOracleViews<C: Curve> {
        async fn get_group(&self, group_index: usize) -> ContractClientResult<Group<C>>;
    }

    pub trait ControllerOracleClientBuilder<C: Curve> {
        type ControllerOracleService: ControllerOracleViews<C> + Send + Sync;

        fn build_controller_oracle_client(&self) -> Self::ControllerOracleService;
    }
}

pub mod controller_relayer {
    use crate::error::ContractClientResult;
    use async_trait::async_trait;
    use ethers::types::TransactionReceipt;

    #[async_trait]
    pub trait ControllerRelayerTransactions {
        async fn relay_group(
            &self,
            chain_id: usize,
            group_index: usize,
        ) -> ContractClientResult<TransactionReceipt>;
    }

    pub trait ControllerRelayerClientBuilder {
        type ControllerRelayerService: ControllerRelayerTransactions + Send + Sync;

        fn build_controller_relayer_client(&self) -> Self::ControllerRelayerService;
    }
}

pub mod coordinator {
    use async_trait::async_trait;
    use dkg_core::BoardPublisher;
    use ethers::{core::types::Address, types::TransactionReceipt};
    use thiserror::Error;
    use threshold_bls::group::Curve;

    use crate::error::{ContractClientError, ContractClientResult};

    #[derive(Debug, Error)]
    pub enum DKGContractError {
        #[error(transparent)]
        SerializationError(#[from] bincode::Error),
        #[error(transparent)]
        PublishingError(#[from] ContractClientError),
    }

    #[async_trait]
    pub trait CoordinatorTransactions {
        /// Participant publishes their data and depending on the phase the data gets inserted
        /// in the shares, responses or justifications mapping. Reverts if the participant
        /// has already published their data for a phase or if the DKG has ended.
        async fn publish(&self, value: Vec<u8>) -> ContractClientResult<TransactionReceipt>;
    }

    #[async_trait]
    pub trait CoordinatorViews {
        // Helpers to fetch data in the mappings. If a participant has registered but not
        // published their data for a phase, the array element at their index is expected to be 0

        /// Gets the participants' shares
        async fn get_shares(&self) -> ContractClientResult<Vec<Vec<u8>>>;

        /// Gets the participants' responses
        async fn get_responses(&self) -> ContractClientResult<Vec<Vec<u8>>>;

        /// Gets the participants' justifications
        async fn get_justifications(&self) -> ContractClientResult<Vec<Vec<u8>>>;

        /// Gets the participants' ethereum addresses
        async fn get_participants(&self) -> ContractClientResult<Vec<Address>>;

        /// Gets the participants' BLS keys along with the thershold of the DKG
        async fn get_dkg_keys(&self) -> ContractClientResult<(usize, Vec<Vec<u8>>)>;

        /// Returns the current phase of the DKG.
        async fn in_phase(&self) -> ContractClientResult<i8>;
    }

    pub trait CoordinatorClientBuilder<C: Curve> {
        type CoordinatorService: CoordinatorTransactions
            + CoordinatorViews
            + BoardPublisher<C>
            + Sync
            + Send;

        fn build_coordinator_client(&self, contract_address: Address) -> Self::CoordinatorService;
    }
}

pub mod adapter {
    use arpa_core::{PartialSignature, RandomnessTask};
    use async_trait::async_trait;
    use ethers::core::types::Address;
    use ethers::types::{TransactionReceipt, U256};
    use std::collections::BTreeMap;
    use std::future::Future;

    use crate::error::ContractClientResult;

    #[async_trait]
    pub trait AdapterTransactions {
        async fn fulfill_randomness(
            &self,
            group_index: usize,
            task: RandomnessTask,
            signature: Vec<u8>,
            partial_signatures: BTreeMap<Address, PartialSignature>,
        ) -> ContractClientResult<TransactionReceipt>;
    }

    #[async_trait]
    pub trait AdapterViews {
        async fn get_last_randomness(&self) -> ContractClientResult<U256>;

        async fn is_task_pending(&self, request_id: &[u8]) -> ContractClientResult<bool>;
    }

    #[async_trait]
    pub trait AdapterLogs {
        async fn subscribe_randomness_task<
            C: FnMut(RandomnessTask) -> F + Send,
            F: Future<Output = ContractClientResult<()>> + Send,
        >(
            &self,
            cb: C,
        ) -> ContractClientResult<()>;
    }

    pub trait AdapterClientBuilder {
        type AdapterService: AdapterTransactions + AdapterViews + AdapterLogs + Send + Sync;

        fn build_adapter_client(&self, main_id_address: Address) -> Self::AdapterService;
    }
}

pub mod provider {

    use std::future::Future;

    use async_trait::async_trait;

    use crate::error::ContractClientResult;

    #[async_trait]
    pub trait BlockFetcher {
        async fn subscribe_new_block_height<
            C: FnMut(usize) -> F + Send,
            F: Future<Output = ContractClientResult<()>> + Send,
        >(
            &self,
            cb: C,
        ) -> ContractClientResult<()>;
    }
}
