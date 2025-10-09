#![allow(clippy::large_enum_variant)]
#![allow(clippy::too_many_arguments)]
use crate::error::ContractClientError;
use alloy::contract::{CallBuilder, CallDecoder};
use alloy::eips::eip1559::Eip1559Estimation;
use alloy::eips::BlockNumberOrTag;
use alloy::providers::utils::Eip1559Estimator;
use alloy::providers::Provider;
use alloy::rpc::types::TransactionReceipt;
use arpa_core::{
    eip1559_gas_price_estimator, fallback_eip1559_gas_price_estimator, jitter, supports_eip1559,
    ExponentialBackoffRetryDescriptor, ProviderClientWithSigner,
};
use async_trait::async_trait;
use error::ContractClientResult;
use log::{error, info};
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::{Retry, RetryIf};

pub mod error;
pub mod ethers;

#[async_trait]
pub trait ServiceClient<C> {
    async fn prepare_service_client(&self) -> ContractClientResult<C>;
}

#[async_trait]
pub trait TransactionCaller {
    async fn call_contract_transaction<
        P: Provider,
        D: CallDecoder + std::fmt::Debug + Send + Sync + 'static,
    >(
        chain_id: u64,
        info: &str,
        client: &ProviderClientWithSigner,
        call: CallBuilder<P, D>,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        retry_on_transaction_fail: bool,
        max_priority_fee_per_gas: Option<u128>,
    ) -> ContractClientResult<TransactionReceipt>
// where
    //     ContractClientError: From<ContractError<M>>,
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

        let mut tx = call.into_transaction_request();

        // transform the trx to legacy if the chain does not support EIP-1559
        if !supports_eip1559(chain_id) {
            // call = call.legacy();
            if let Some(max_priority_fee_per_gas) = max_priority_fee_per_gas {
                tx.gas_price = Some(max_priority_fee_per_gas);
            }
        }
        // set gas price for EIP-1559 trxs
        else if tx.has_eip1559_fields() {
            let (max_fee, max_priority_fee) = match client
                .estimate_eip1559_fees_with(Eip1559Estimator::Custom(Box::new(
                    eip1559_gas_price_estimator,
                )))
                .await
            {
                // if max_priority_fee is zero, it usually means that the chain is a testnet,
                // we will use the legacy method to set a priority fee, to avoid the transaction being underpriced
                Ok(Eip1559Estimation {
                    max_fee_per_gas: max_fee,
                    max_priority_fee_per_gas,
                }) if max_priority_fee_per_gas != 0 => (max_fee, max_priority_fee_per_gas),
                _ => {
                    // try to estimate the gas price using the legacy method
                    let base_fee_per_gas = client
                        .get_block(alloy::eips::BlockId::Number(BlockNumberOrTag::Latest))
                        .await?
                        .ok_or_else(|| {
                            ContractClientError::CustomError("Latest block not found".into())
                        })?
                        .header
                        .base_fee_per_gas
                        .ok_or_else(|| {
                            ContractClientError::CustomError("EIP-1559 not activated".into())
                        })?;

                    let gas_price = client.get_gas_price().await?;
                    // .map_err(ContractError::from_middleware_error)?;

                    let Eip1559Estimation {
                        max_fee_per_gas,
                        max_priority_fee_per_gas,
                    } = fallback_eip1559_gas_price_estimator(
                        base_fee_per_gas as u128,
                        gas_price - base_fee_per_gas as u128,
                    );
                    (max_fee_per_gas, max_priority_fee_per_gas)
                }
            };

            if let Some(max_priority_fee_per_gas) = max_priority_fee_per_gas {
                tx.max_priority_fee_per_gas = Some(max_priority_fee_per_gas);
                if max_priority_fee_per_gas > max_priority_fee {
                    tx.max_fee_per_gas =
                        Some(max_fee - max_priority_fee + max_priority_fee_per_gas);
                } else {
                    tx.max_fee_per_gas = Some(max_fee);
                }
            } else {
                tx.max_fee_per_gas = Some(max_fee);
                tx.max_priority_fee_per_gas = Some(max_priority_fee);
            }
        }

        let transaction_receipt = RetryIf::spawn(
            retry_strategy,
            || async {
                let pending_tx = client.send_transaction(tx.clone()).await?;

                info!(
                    "Calling contract transaction {} with chain_id({}): {:?}",
                    info,
                    chain_id,
                    pending_tx.tx_hash()
                );

                let receipt = pending_tx.get_receipt().await?;

                if !receipt.status() {
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
        P: Provider,
        D: CallDecoder + Unpin + std::fmt::Debug + Send + Sync + 'static,
    >(
        chain_id: u64,
        info: &str,
        call: CallBuilder<P, D>,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    ) -> ContractClientResult<D::CallOutput> {
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
            let result = call.call().await?;

            info!(
                "Calling contract view {} with chain_id({}), calldata: {:?}",
                info,
                chain_id,
                call.calldata(),
            );

            Result::<D::CallOutput, ContractClientError>::Ok(result)
        })
        .await?;

        Ok(res)
    }

    async fn call_contract_view_without_log<
        P: Provider,
        D: CallDecoder + Unpin + std::fmt::Debug + Send + Sync + 'static,
    >(
        call: CallBuilder<P, D>,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    ) -> ContractClientResult<D::CallOutput> {
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
            let result = call.call().await?;

            Result::<D::CallOutput, ContractClientError>::Ok(result)
        })
        .await?;

        Ok(res)
    }
}

pub mod node_registry {
    use crate::error::ContractClientResult;
    use alloy::primitives::Address;
    use alloy::rpc::types::TransactionReceipt;
    use alloy::signers::local::PrivateKeySigner;
    use arpa_core::Node;
    use async_trait::async_trait;

    #[async_trait]
    pub trait NodeRegistryTransactions {
        async fn node_register_as_eigenlayer_operator(
            &self,
            id_public_key: Vec<u8>,
            asset_account_signer: &PrivateKeySigner,
        ) -> ContractClientResult<TransactionReceipt>;

        async fn node_register_by_consistent_native_staking(
            &self,
            id_public_key: Vec<u8>,
        ) -> ContractClientResult<TransactionReceipt>;

        async fn node_activate_as_eigenlayer_operator(
            &self,
            asset_account_signer: &PrivateKeySigner,
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
    use alloy::primitives::Address;
    use alloy::rpc::types::TransactionReceipt;
    use arpa_core::{DKGTask, Group};
    use async_trait::async_trait;
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
    use alloy::primitives::Address;
    use alloy::rpc::types::TransactionReceipt;
    use arpa_core::Group;
    use async_trait::async_trait;
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
    use alloy::rpc::types::TransactionReceipt;
    use async_trait::async_trait;

    #[async_trait]
    pub trait ControllerRelayerTransactions {
        async fn relay_group(
            &self,
            chain_id: u64,
            group_index: usize,
        ) -> ContractClientResult<TransactionReceipt>;
    }

    pub trait ControllerRelayerClientBuilder {
        type ControllerRelayerService: ControllerRelayerTransactions + Send + Sync;

        fn build_controller_relayer_client(&self) -> Self::ControllerRelayerService;
    }
}

pub mod coordinator {
    use alloy::primitives::Address;
    use alloy::rpc::types::TransactionReceipt;
    use async_trait::async_trait;
    use dkg_core::BoardPublisher;
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
    use alloy::primitives::{Address, U256};
    use alloy::rpc::types::TransactionReceipt;
    use arpa_core::{PartialSignature, RandomnessTask};
    use async_trait::async_trait;
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
