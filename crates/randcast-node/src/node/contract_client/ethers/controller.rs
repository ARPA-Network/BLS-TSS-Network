use crate::node::{
    contract_client::{
        controller::{
            ControllerClientBuilder, ControllerLogs, ControllerTransactions, ControllerViews,
        },
        types::Node,
    },
    dal::{
        types::{DKGTask, GeneralChainIdentity, GroupRelayTask},
        ChainIdentity,
    },
    error::{ContractClientError, NodeResult},
    ServiceClient,
};
use async_trait::async_trait;
use ethers::{prelude::*, utils::keccak256};
use std::{convert::TryFrom, sync::Arc, time::Duration};

use self::controller_stub::Controller;

use super::WalletSigner;

#[allow(clippy::useless_conversion)]
pub mod controller_stub {
    include!("../../../../contract_stub/controller.rs");
}

pub struct ControllerClient {
    controller_address: Address,
    signer: Arc<WalletSigner>,
}

impl ControllerClient {
    pub fn new(controller_address: Address, identity: &GeneralChainIdentity) -> Self {
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

        ControllerClient {
            controller_address,
            signer,
        }
    }
}

impl ControllerClientBuilder for GeneralChainIdentity {
    type Service = ControllerClient;

    fn build_controller_client(&self) -> ControllerClient {
        ControllerClient::new(self.get_contract_address(), self)
    }
}

type ControllerContract = Controller<WalletSigner>;

#[async_trait]
impl ServiceClient<ControllerContract> for ControllerClient {
    async fn prepare_service_client(&self) -> NodeResult<ControllerContract> {
        let controller_contract = Controller::new(self.controller_address, self.signer.clone());

        Ok(controller_contract)
    }
}

#[allow(unused_variables)]
#[async_trait]
impl ControllerTransactions for ControllerClient {
    async fn node_register(&self, id_public_key: Vec<u8>) -> NodeResult<()> {
        Ok(())
    }

    async fn commit_dkg(
        &self,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<Address>,
    ) -> NodeResult<()> {
        Ok(())
    }

    async fn post_process_dkg(&self, group_index: usize, group_epoch: usize) -> NodeResult<()> {
        Ok(())
    }
}

#[allow(unused_variables)]
#[async_trait]
impl ControllerViews for ControllerClient {
    async fn get_node(&self, id_address: Address) -> NodeResult<Node> {
        todo!()
    }
}

#[async_trait]
impl ControllerLogs for ControllerClient {
    async fn subscribe_dkg_task(
        &self,
        cb: Box<dyn Fn(DKGTask) -> NodeResult<()> + Sync + Send>,
    ) -> NodeResult<()> {
        let dkg_task_filter =
            Filter::new()
                .from_block(BlockNumber::Latest)
                .topic0(ValueOrArray::Value(H256::from(keccak256(
                    "DkgTask(address,address,uint256)",
                ))));
        let mut stream = self.signer.watch(&dkg_task_filter).await.map_err(|e| {
            let e: ContractClientError = e.into();
            e
        })?;
        while let Some(log) = stream.next().await {
            cb(log.into())?;
        }
        Err(ContractClientError::FetchingDkgTaskError.into())
    }

    async fn subscribe_group_relay_task(
        &self,
        cb: Box<dyn Fn(GroupRelayTask) -> NodeResult<()> + Sync + Send>,
    ) -> NodeResult<()> {
        let group_relay_task_filter =
            Filter::new()
                .from_block(BlockNumber::Latest)
                .topic0(ValueOrArray::Value(H256::from(keccak256(
                    "GroupRelayTask(address,address,uint256)",
                ))));
        let mut stream = self
            .signer
            .watch(&group_relay_task_filter)
            .await
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;
        while let Some(log) = stream.next().await {
            cb(log.into())?;
        }
        Err(ContractClientError::FetchingGroupRelayTaskError.into())
    }
}

impl From<Log> for DKGTask {
    fn from(_: Log) -> Self {
        todo!()
    }
}

impl From<Log> for GroupRelayTask {
    fn from(_: Log) -> Self {
        todo!()
    }
}
