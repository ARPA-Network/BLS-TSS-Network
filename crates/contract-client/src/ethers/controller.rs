use crate::{
    controller::{
        ControllerClientBuilder, ControllerLogs, ControllerTransactions, ControllerViews,
    },
    error::{ContractClientError, ContractClientResult},
    ethers::{
        controller::{
            Controller::{ControllerInstance, DkgTask as ContractDkgTask},
            IController::CommitDkgParams,
        },
        parse_controller_contract_group,
    },
    ServiceClient,
};
use crate::{TransactionCaller, ViewCaller};
use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, U256},
    rpc::types::TransactionReceipt,
    sol,
};
use arpa_core::{
    ChainIdentity, DKGTask, ExponentialBackoffRetryDescriptor, GeneralMainChainIdentity,
    GeneralRelayedChainIdentity, Group, MainChainIdentity, ProviderClientWithSigner,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use log::info;
use std::future::Future;
use threshold_bls::group::Curve;

sol! {
    #[sol(ignore_unlinked)]
    #[sol(rpc)]
    Controller,
    "abi/Controller.json"
}

pub struct ControllerClient {
    chain_id: u64,
    controller_address: Address,
    client: ProviderClientWithSigner,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    max_priority_fee_per_gas: Option<u128>,
}

impl ControllerClient {
    pub fn new(
        chain_id: u64,
        controller_address: Address,
        identity: &GeneralMainChainIdentity,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
        max_priority_fee_per_gas: Option<u128>,
    ) -> Self {
        ControllerClient {
            chain_id,
            controller_address,
            client: identity.get_client(),
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
            max_priority_fee_per_gas,
        }
    }
}

impl<C: Curve> ControllerClientBuilder<C> for GeneralMainChainIdentity {
    type ControllerService = ControllerClient;

    fn build_controller_client(&self) -> ControllerClient {
        ControllerClient::new(
            self.get_chain_id(),
            self.get_controller_address(),
            self,
            self.get_contract_transaction_retry_descriptor(),
            self.get_contract_view_retry_descriptor(),
            self.get_max_priority_fee_per_gas(),
        )
    }
}

impl<C: Curve> ControllerClientBuilder<C> for GeneralRelayedChainIdentity {
    type ControllerService = ControllerClient;

    fn build_controller_client(&self) -> ControllerClient {
        panic!("not implemented")
    }
}

type ControllerContract = ControllerInstance<ProviderClientWithSigner>;

#[async_trait]
impl ServiceClient<ControllerContract> for ControllerClient {
    async fn prepare_service_client(&self) -> ContractClientResult<ControllerContract> {
        let controller_contract = Controller::new(self.controller_address, self.client.clone());

        Ok(controller_contract)
    }
}

#[async_trait]
impl TransactionCaller for ControllerClient {}

#[async_trait]
impl ViewCaller for ControllerClient {}

#[async_trait]
impl ControllerTransactions for ControllerClient {
    async fn commit_dkg(
        &self,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<Address>,
    ) -> ContractClientResult<TransactionReceipt> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        let call = controller_contract.commitDkg(CommitDkgParams {
            groupIndex: U256::from(group_index),
            groupEpoch: U256::from(group_epoch),
            publicKey: public_key.into(),
            partialPublicKey: partial_public_key.into(),
            disqualifiedNodes: disqualified_nodes,
        });

        ControllerClient::call_contract_transaction(
            self.chain_id,
            "commit_dkg",
            controller_contract.provider(),
            call,
            self.contract_transaction_retry_descriptor,
            true,
            self.max_priority_fee_per_gas,
        )
        .await
    }

    async fn post_process_dkg(
        &self,
        group_index: usize,
        group_epoch: usize,
    ) -> ContractClientResult<TransactionReceipt> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        let call =
            controller_contract.postProcessDkg(U256::from(group_index), U256::from(group_epoch));

        ControllerClient::call_contract_transaction(
            self.chain_id,
            "post_process_dkg",
            controller_contract.provider(),
            call,
            self.contract_transaction_retry_descriptor,
            false,
            self.max_priority_fee_per_gas,
        )
        .await
    }
}

#[async_trait]
impl<C: Curve> ControllerViews<C> for ControllerClient {
    async fn get_group(&self, group_index: usize) -> ContractClientResult<Group<C>> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        ControllerClient::call_contract_view(
            self.chain_id,
            "get_group",
            controller_contract.getGroup(U256::from(group_index)),
            self.contract_view_retry_descriptor,
        )
        .await
        .map(parse_controller_contract_group)
    }

    async fn get_coordinator(&self, group_index: usize) -> ContractClientResult<Address> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        ControllerClient::call_contract_view(
            self.chain_id,
            "get_coordinator",
            controller_contract.getCoordinator(U256::from(group_index)),
            self.contract_view_retry_descriptor,
        )
        .await
    }

    async fn get_node_registry_address(&self) -> ContractClientResult<Address> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        let config = ControllerClient::call_contract_view(
            self.chain_id,
            "get_controller_config",
            controller_contract.getControllerConfig(),
            self.contract_view_retry_descriptor,
        )
        .await?;

        Ok(config.nodeRegistryContractAddress)
    }
}

#[async_trait]
impl ControllerLogs for ControllerClient {
    async fn subscribe_dkg_task<
        C: FnMut(DKGTask) -> F + Send,
        F: Future<Output = ContractClientResult<()>> + Send,
    >(
        &self,
        mut cb: C,
    ) -> ContractClientResult<()> {
        let contract = Controller::new(self.controller_address, self.client.clone());

        let mut stream = contract
            .DkgTask_filter()
            .from_block(BlockNumberOrTag::Latest)
            .subscribe()
            .await?
            .into_stream();

        while let Some(Ok(evt)) = stream.next().await {
            let (
                ContractDkgTask {
                    globalEpoch: _,
                    groupIndex,
                    groupEpoch,
                    size,
                    threshold,
                    members,
                    assignmentBlockHeight: _,
                    coordinatorAddress,
                },
                meta,
            ) = evt;

            info!(
                "Received DKG task: group_index: {}, epoch: {}, size: {}, threshold: {}, members: {:?}, coordinator_address: {}, block_number: {}",
                groupIndex, groupEpoch, size, threshold, members, coordinatorAddress, meta.block_number.unwrap_or(0)
            );

            let task = DKGTask {
                group_index: groupIndex.to::<usize>(),
                epoch: groupEpoch.to::<usize>(),
                size: size.to::<usize>(),
                threshold: threshold.to::<usize>(),
                members,
                assignment_block_height: meta.block_number.unwrap_or(0) as usize,
                coordinator_address: coordinatorAddress,
            };
            cb(task).await?;
        }
        Err(ContractClientError::FetchingDkgTaskError)
    }
}
