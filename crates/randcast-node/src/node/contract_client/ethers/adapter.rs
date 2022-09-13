use self::adapter_stub::Adapter;
use super::WalletSigner;
use crate::node::{
    contract_client::adapter::{
        AdapterClientBuilder, AdapterLogs, AdapterTransactions, AdapterViews,
    },
    dal::{
        types::{
            GeneralChainIdentity, Group, GroupRelayConfirmationTask,
            GroupRelayConfirmationTaskState, RandomnessTask,
        },
        ChainIdentity,
    },
    error::{ContractClientError, NodeResult},
    ServiceClient,
};
use async_trait::async_trait;
use ethers::prelude::*;
use ethers::utils::keccak256;
use std::{collections::HashMap, convert::TryFrom, sync::Arc, time::Duration};

#[allow(clippy::useless_conversion)]
pub mod adapter_stub {
    include!("../../../../contract_stub/adapter.rs");
}

#[allow(dead_code)]
pub struct AdapterClient {
    main_id_address: Address,
    adapter_address: Address,
    signer: Arc<WalletSigner>,
}

impl AdapterClient {
    pub fn new(
        main_id_address: Address,
        adapter_address: Address,
        identity: &GeneralChainIdentity,
    ) -> Self {
        let provider = Provider::<Http>::try_from(identity.get_provider_rpc_endpoint())
            .unwrap()
            .interval(Duration::from_millis(10u64));

        // instantiate the client with the wallet
        let signer = Arc::new(SignerMiddleware::new(
            provider,
            identity
                .wallet
                .clone()
                .with_chain_id(identity.get_chain_id() as u32),
        ));

        AdapterClient {
            main_id_address,
            adapter_address,
            signer,
        }
    }
}

impl AdapterClientBuilder for GeneralChainIdentity {
    type Service = AdapterClient;

    fn build_adapter_client(&self, main_id_address: Address) -> AdapterClient {
        AdapterClient::new(main_id_address, self.get_contract_address(), self)
    }
}

type AdapterContract = Adapter<WalletSigner>;

#[async_trait]
impl ServiceClient<AdapterContract> for AdapterClient {
    async fn prepare_service_client(&self) -> NodeResult<AdapterContract> {
        let adapter_contract = Adapter::new(self.adapter_address, self.signer.clone());

        Ok(adapter_contract)
    }
}

#[allow(unused_variables)]
#[async_trait]
impl AdapterTransactions for AdapterClient {
    async fn request_randomness(&self, message: &str) -> NodeResult<()> {
        Ok(())
    }

    async fn fulfill_randomness(
        &self,
        group_index: usize,
        signature_index: usize,
        signature: Vec<u8>,
        partial_signatures: HashMap<Address, Vec<u8>>,
    ) -> NodeResult<()> {
        Ok(())
    }

    async fn fulfill_relay(
        &self,
        relayer_group_index: usize,
        task_index: usize,
        signature: Vec<u8>,
        group_as_bytes: Vec<u8>,
    ) -> NodeResult<()> {
        Ok(())
    }

    async fn cancel_invalid_relay_confirmation_task(&self, task_index: usize) -> NodeResult<()> {
        Ok(())
    }

    async fn confirm_relay(
        &self,
        task_index: usize,
        group_relay_confirmation_as_bytes: Vec<u8>,
        signature: Vec<u8>,
    ) -> NodeResult<()> {
        Ok(())
    }

    async fn set_initial_group(&self, group: Vec<u8>) -> NodeResult<()> {
        Ok(())
    }
}

#[allow(unused_variables)]
#[async_trait]
impl AdapterViews for AdapterClient {
    async fn get_group(&self, group_index: usize) -> NodeResult<Group> {
        todo!()
    }

    async fn get_last_output(&self) -> NodeResult<u64> {
        todo!()
    }

    async fn get_signature_task_completion_state(&self, index: usize) -> NodeResult<bool> {
        todo!()
    }

    async fn get_group_relay_cache(&self, group_index: usize) -> NodeResult<Group> {
        todo!()
    }

    async fn get_group_relay_confirmation_task_state(
        &self,
        task_index: usize,
    ) -> NodeResult<GroupRelayConfirmationTaskState> {
        todo!()
    }
}

#[async_trait]
impl AdapterLogs for AdapterClient {
    async fn subscribe_randomness_task(
        &self,
        cb: Box<dyn Fn(RandomnessTask) -> NodeResult<()> + Sync + Send>,
    ) -> NodeResult<()> {
        let randomness_task_filter =
            Filter::new()
                .from_block(BlockNumber::Latest)
                .topic0(ValueOrArray::Value(H256::from(keccak256(
                    "RandomnessTask(address,address,uint256)",
                ))));
        let mut stream = self
            .signer
            .watch(&randomness_task_filter)
            .await
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;
        while let Some(log) = stream.next().await {
            cb(log.into())?;
        }
        Err(ContractClientError::FetchingRandomnessTaskError.into())
    }

    async fn subscribe_group_relay_confirmation_task(
        &self,
        cb: Box<dyn Fn(GroupRelayConfirmationTask) -> NodeResult<()> + Sync + Send>,
    ) -> NodeResult<()> {
        let group_relay_confirmation_task_filter = Filter::new()
            .from_block(BlockNumber::Latest)
            .topic0(ValueOrArray::Value(H256::from(keccak256(
                "GroupRelayConfirmationTask(address,address,uint256)",
            ))));
        let mut stream = self
            .signer
            .watch(&group_relay_confirmation_task_filter)
            .await
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;
        while let Some(log) = stream.next().await {
            cb(log.into())?;
        }
        Err(ContractClientError::FetchingGroupRelayConfirmationTaskError.into())
    }
}

impl From<Log> for RandomnessTask {
    fn from(_: Log) -> Self {
        todo!()
    }
}

impl From<Log> for GroupRelayConfirmationTask {
    fn from(_: Log) -> Self {
        todo!()
    }
}
