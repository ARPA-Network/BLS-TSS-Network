use crate::error::ContractClientError;
use ::ethers::abi::Detokenize;
use ::ethers::prelude::ContractError;
use ::ethers::providers::Middleware;
use ::ethers::types::U64;
use ::ethers::{prelude::builders::ContractCall, types::H256};
use arpa_core::{eip1559_gas_price_estimator, jitter, ExponentialBackoffRetryDescriptor};
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
    ) -> ContractClientResult<H256>
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

        // set gas price for EIP-1559 trxs
        if let Some(tx) = call.tx.as_eip1559_mut() {
            let (max_fee, max_priority_fee) = client
                .estimate_eip1559_fees(Some(eip1559_gas_price_estimator))
                .await
                .map_err(ContractError::from_middleware_error)?;
            tx.max_fee_per_gas = Some(max_fee);
            tx.max_priority_fee_per_gas = Some(max_priority_fee);
        }

        let transaction_hash = RetryIf::spawn(
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
                    return Err(ContractClientError::TransactionFailed);
                } else {
                    info!(
                        "Transaction successful({}) with chain_id({}), receipt: {:?}",
                        info, chain_id, receipt
                    );
                }

                Ok(receipt.transaction_hash)
            },
            |e: &ContractClientError| {
                retry_on_transaction_fail || !matches!(e, ContractClientError::TransactionFailed)
            },
        )
        .await?;

        Ok(transaction_hash)
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

pub mod controller {
    use crate::error::ContractClientResult;
    use arpa_core::{DKGTask, Group, Node};
    use async_trait::async_trait;
    use ethers::core::types::Address;
    use ethers::types::H256;
    use std::future::Future;
    use threshold_bls::group::Curve;

    #[async_trait]
    pub trait ControllerTransactions {
        async fn node_register(&self, id_public_key: Vec<u8>) -> ContractClientResult<H256>;

        async fn commit_dkg(
            &self,
            group_index: usize,
            group_epoch: usize,
            public_key: Vec<u8>,
            partial_public_key: Vec<u8>,
            disqualified_nodes: Vec<Address>,
        ) -> ContractClientResult<H256>;

        async fn post_process_dkg(
            &self,
            group_index: usize,
            group_epoch: usize,
        ) -> ContractClientResult<H256>;
    }

    #[async_trait]
    pub trait ControllerViews<C: Curve> {
        async fn get_node(&self, id_address: Address) -> ContractClientResult<Node>;

        async fn get_group(&self, group_index: usize) -> ContractClientResult<Group<C>>;

        async fn get_coordinator(&self, group_index: usize) -> ContractClientResult<Address>;
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
    use ethers::types::{Address, H256};
    use threshold_bls::group::Curve;

    #[async_trait]
    pub trait ControllerOracleTransactions {
        async fn node_withdraw(&self, recipient: Address) -> ContractClientResult<H256>;
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
    use ethers::types::H256;

    #[async_trait]
    pub trait ControllerRelayerTransactions {
        async fn relay_group(
            &self,
            chain_id: usize,
            group_index: usize,
        ) -> ContractClientResult<H256>;
    }

    pub trait ControllerRelayerClientBuilder {
        type ControllerRelayerService: ControllerRelayerTransactions + Send + Sync;

        fn build_controller_relayer_client(&self) -> Self::ControllerRelayerService;
    }
}

pub mod coordinator {
    use async_trait::async_trait;
    use dkg_core::BoardPublisher;
    use ethers::core::types::Address;
    use ethers::types::H256;
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
        async fn publish(&self, value: Vec<u8>) -> ContractClientResult<H256>;
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
    use ethers::types::{H256, U256};
    use std::{collections::HashMap, future::Future};

    use crate::error::ContractClientResult;

    #[async_trait]
    pub trait AdapterTransactions {
        async fn fulfill_randomness(
            &self,
            group_index: usize,
            task: RandomnessTask,
            signature: Vec<u8>,
            partial_signatures: HashMap<Address, PartialSignature>,
        ) -> ContractClientResult<H256>;
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
