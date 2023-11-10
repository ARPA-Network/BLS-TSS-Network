use crate::{
    contract_stub::controller::{
        CommitDkgParams, Controller, DkgTaskFilter, Group as ContractGroup,
    },
    controller::{
        ControllerClientBuilder, ControllerLogs, ControllerTransactions, ControllerViews,
    },
    error::{ContractClientError, ContractClientResult},
    ServiceClient,
};
use crate::{TransactionCaller, ViewCaller};
use arpa_core::{
    u256_to_vec, ChainIdentity, DKGTask, ExponentialBackoffRetryDescriptor,
    GeneralMainChainIdentity, GeneralRelayedChainIdentity, Group, MainChainIdentity, Member, Node,
    WsWalletSigner,
};
use async_trait::async_trait;
use ethers::prelude::*;
use log::info;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::{future::Future, sync::Arc};
use threshold_bls::group::Curve;

pub struct ControllerClient {
    chain_id: usize,
    controller_address: Address,
    signer: Arc<WsWalletSigner>,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl ControllerClient {
    pub fn new(
        chain_id: usize,
        controller_address: Address,
        identity: &GeneralMainChainIdentity,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    ) -> Self {
        ControllerClient {
            chain_id,
            controller_address,
            signer: identity.get_signer(),
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
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
        )
    }
}

impl<C: Curve> ControllerClientBuilder<C> for GeneralRelayedChainIdentity {
    type ControllerService = ControllerClient;

    fn build_controller_client(&self) -> ControllerClient {
        panic!("not implemented")
    }
}

type ControllerContract = Controller<WsWalletSigner>;

#[async_trait]
impl ServiceClient<ControllerContract> for ControllerClient {
    async fn prepare_service_client(&self) -> ContractClientResult<ControllerContract> {
        let controller_contract = Controller::new(self.controller_address, self.signer.clone());

        Ok(controller_contract)
    }
}

#[async_trait]
impl TransactionCaller for ControllerClient {}

#[async_trait]
impl ViewCaller for ControllerClient {}

#[async_trait]
impl ControllerTransactions for ControllerClient {
    async fn node_register(&self, id_public_key: Vec<u8>) -> ContractClientResult<H256> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        let call = controller_contract.node_register(id_public_key.into());

        ControllerClient::call_contract_transaction(
            self.chain_id,
            "node_register",
            controller_contract.client_ref(),
            call,
            self.contract_transaction_retry_descriptor,
            true,
        )
        .await
    }

    async fn commit_dkg(
        &self,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<Address>,
    ) -> ContractClientResult<H256> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        let call = controller_contract.commit_dkg(CommitDkgParams {
            group_index: group_index.into(),
            group_epoch: group_epoch.into(),
            public_key: public_key.into(),
            partial_public_key: partial_public_key.into(),
            disqualified_nodes,
        });

        ControllerClient::call_contract_transaction(
            self.chain_id,
            "commit_dkg",
            controller_contract.client_ref(),
            call,
            self.contract_transaction_retry_descriptor,
            true,
        )
        .await
    }

    async fn post_process_dkg(
        &self,
        group_index: usize,
        group_epoch: usize,
    ) -> ContractClientResult<H256> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        let call = controller_contract.post_process_dkg(group_index.into(), group_epoch.into());

        ControllerClient::call_contract_transaction(
            self.chain_id,
            "post_process_dkg",
            controller_contract.client_ref(),
            call,
            self.contract_transaction_retry_descriptor,
            false,
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
            controller_contract.get_group(group_index.into()),
            self.contract_view_retry_descriptor,
        )
        .await
        .map(parse_contract_group)
    }

    async fn get_node(&self, id_address: Address) -> ContractClientResult<Node> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        ControllerClient::call_contract_view(
            self.chain_id,
            "get_node",
            controller_contract.get_node(id_address),
            self.contract_view_retry_descriptor,
        )
        .await
        .map(|n| Node {
            id_address: n.id_address,
            id_public_key: n.dkg_public_key.to_vec(),
            state: n.state,
            pending_until_block: n.pending_until_block.as_usize(),
        })
    }

    async fn get_coordinator(&self, group_index: usize) -> ContractClientResult<Address> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        ControllerClient::call_contract_view(
            self.chain_id,
            "get_coordinator",
            controller_contract.get_coordinator(group_index.into()),
            self.contract_view_retry_descriptor,
        )
        .await
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
        let contract = Controller::new(self.controller_address, self.signer.clone());

        let events = contract
            .event::<DkgTaskFilter>()
            .from_block(BlockNumber::Latest);

        let mut stream = events.subscribe().await?.with_meta();

        while let Some(Ok(evt)) = stream.next().await {
            let (
                DkgTaskFilter {
                    global_epoch: _,
                    group_index,
                    group_epoch,
                    size,
                    threshold,
                    members,
                    assignment_block_height: _,
                    coordinator_address,
                },
                meta,
            ) = evt;

            info!(
                "Received DKG task: group_index: {}, epoch: {}, size: {}, threshold: {}, members: {:?}, coordinator_address: {}, block_number: {}",
                group_index, group_epoch, size, threshold, members, coordinator_address, meta.block_number
            );

            let task = DKGTask {
                group_index: group_index.as_usize(),
                epoch: group_epoch.as_usize(),
                size: size.as_usize(),
                threshold: threshold.as_usize(),
                members,
                assignment_block_height: meta.block_number.as_usize(),
                coordinator_address,
            };
            cb(task).await?;
        }
        Err(ContractClientError::FetchingDkgTaskError)
    }
}

fn parse_contract_group<C: Curve>(cg: ContractGroup) -> Group<C> {
    let ContractGroup {
        index,
        epoch,
        size,
        threshold,
        public_key,
        members,
        committers,
        commit_cache_list: _,
        is_strictly_majority_consensus_reached,
    } = cg;

    let members: BTreeMap<Address, Member<C>> = members
        .into_iter()
        .enumerate()
        .map(|(index, cm)| {
            let partial_public_key =
                if cm.partial_public_key.is_empty() || cm.partial_public_key[0] == U256::zero() {
                    None
                } else {
                    let bytes = cm
                        .partial_public_key
                        .iter()
                        .map(u256_to_vec)
                        .reduce(|mut acc, mut e| {
                            acc.append(&mut e);
                            acc
                        })
                        .unwrap();
                    Some(bincode::deserialize(&bytes).unwrap())
                };

            let m = Member {
                index,
                id_address: cm.node_id_address,
                rpc_endpoint: None,
                partial_public_key,
            };
            (m.id_address, m)
        })
        .collect();

    let public_key = if public_key.is_empty() || public_key[0] == U256::zero() {
        None
    } else {
        let bytes = public_key
            .iter()
            .map(u256_to_vec)
            .reduce(|mut acc, mut e| {
                acc.append(&mut e);
                acc
            })
            .unwrap();
        Some(bincode::deserialize(&bytes).unwrap())
    };

    Group {
        index: index.as_usize(),
        epoch: epoch.as_usize(),
        size: size.as_usize(),
        threshold: threshold.as_usize(),
        state: is_strictly_majority_consensus_reached,
        public_key,
        members,
        committers,
        c: PhantomData,
    }
}
