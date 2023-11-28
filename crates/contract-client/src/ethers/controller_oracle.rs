use crate::controller_oracle::ControllerOracleTransactions;
use crate::{
    contract_stub::controller_oracle::{ControllerOracle, Group as ContractGroup},
    controller_oracle::{ControllerOracleClientBuilder, ControllerOracleViews},
    error::ContractClientResult,
    ServiceClient,
};
use crate::{TransactionCaller, ViewCaller};
use arpa_core::{
    u256_to_vec, ChainIdentity, ExponentialBackoffRetryDescriptor, GeneralMainChainIdentity,
    GeneralRelayedChainIdentity, Group, Member, RelayedChainIdentity, WsWalletSigner,
};
use async_trait::async_trait;
use ethers::prelude::*;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::Arc;
use threshold_bls::group::Curve;

pub struct ControllerOracleClient {
    chain_id: usize,
    controller_oracle_address: Address,
    signer: Arc<WsWalletSigner>,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl ControllerOracleClient {
    pub fn new(
        chain_id: usize,
        controller_oracle_address: Address,
        identity: &GeneralRelayedChainIdentity,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    ) -> Self {
        ControllerOracleClient {
            chain_id,
            controller_oracle_address,
            signer: identity.get_signer(),
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
        }
    }
}

impl<C: Curve> ControllerOracleClientBuilder<C> for GeneralMainChainIdentity {
    type ControllerOracleService = ControllerOracleClient;

    fn build_controller_oracle_client(&self) -> ControllerOracleClient {
        panic!("not implemented")
    }
}

impl<C: Curve> ControllerOracleClientBuilder<C> for GeneralRelayedChainIdentity {
    type ControllerOracleService = ControllerOracleClient;

    fn build_controller_oracle_client(&self) -> ControllerOracleClient {
        ControllerOracleClient::new(
            self.get_chain_id(),
            self.get_controller_oracle_address(),
            self,
            self.get_contract_transaction_retry_descriptor(),
            self.get_contract_view_retry_descriptor(),
        )
    }
}

type ControllerOracleContract = ControllerOracle<WsWalletSigner>;

#[async_trait]
impl ServiceClient<ControllerOracleContract> for ControllerOracleClient {
    async fn prepare_service_client(&self) -> ContractClientResult<ControllerOracleContract> {
        let controller_oracle_contract =
            ControllerOracle::new(self.controller_oracle_address, self.signer.clone());

        Ok(controller_oracle_contract)
    }
}

#[async_trait]
impl TransactionCaller for ControllerOracleClient {}

#[async_trait]
impl ViewCaller for ControllerOracleClient {}

#[async_trait]
impl ControllerOracleTransactions for ControllerOracleClient {
    async fn node_withdraw(&self, recipient: Address) -> ContractClientResult<H256> {
        let controller_oracle_contract =
            ServiceClient::<ControllerOracleContract>::prepare_service_client(self).await?;

        let call = controller_oracle_contract.node_withdraw(recipient);

        ControllerOracleClient::call_contract_transaction(
            self.chain_id,
            "node_withdraw",
            controller_oracle_contract.client_ref(),
            call,
            self.contract_transaction_retry_descriptor,
            true,
        )
        .await
    }
}

#[async_trait]
impl<C: Curve> ControllerOracleViews<C> for ControllerOracleClient {
    async fn get_group(&self, group_index: usize) -> ContractClientResult<Group<C>> {
        let controller_oracle_contract =
            ServiceClient::<ControllerOracleContract>::prepare_service_client(self).await?;

        ControllerOracleClient::call_contract_view(
            self.chain_id,
            "get_group",
            controller_oracle_contract.get_group(group_index.into()),
            self.contract_view_retry_descriptor,
        )
        .await
        .map(parse_contract_group)
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
