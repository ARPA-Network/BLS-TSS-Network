use crate::{
    contract_stub::{i_controller::SignatureWithSaltAndExpiry, node_registry::NodeRegistry},
    error::ContractClientResult,
    node_registry::{NodeRegistryClientBuilder, NodeRegistryTransactions, NodeRegistryViews},
    ServiceClient,
};
use crate::{TransactionCaller, ViewCaller};
use arpa_core::{
    ChainIdentity, ExponentialBackoffRetryDescriptor, GeneralMainChainIdentity,
    GeneralRelayedChainIdentity, Node, WsWalletSigner,
};
use async_trait::async_trait;
use ethers::prelude::*;
use std::sync::Arc;

pub struct NodeRegistryClient {
    chain_id: usize,
    node_registry_address: Address,
    signer: Arc<WsWalletSigner>,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl NodeRegistryClient {
    pub fn new(
        chain_id: usize,
        node_registry_address: Address,
        identity: &GeneralMainChainIdentity,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    ) -> Self {
        NodeRegistryClient {
            chain_id,
            node_registry_address,
            signer: identity.get_signer(),
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
        }
    }
}

impl NodeRegistryClientBuilder for GeneralMainChainIdentity {
    type NodeRegistryService = NodeRegistryClient;

    fn build_node_registry_client(&self, node_registry_address: Address) -> NodeRegistryClient {
        NodeRegistryClient::new(
            self.get_chain_id(),
            node_registry_address,
            self,
            self.get_contract_transaction_retry_descriptor(),
            self.get_contract_view_retry_descriptor(),
        )
    }
}

impl NodeRegistryClientBuilder for GeneralRelayedChainIdentity {
    type NodeRegistryService = NodeRegistryClient;

    fn build_node_registry_client(&self, _node_registry_address: Address) -> NodeRegistryClient {
        panic!("not implemented")
    }
}

type NodeRegistryContract = NodeRegistry<WsWalletSigner>;

#[async_trait]
impl ServiceClient<NodeRegistryContract> for NodeRegistryClient {
    async fn prepare_service_client(&self) -> ContractClientResult<NodeRegistryContract> {
        let node_registry_contract =
            NodeRegistry::new(self.node_registry_address, self.signer.clone());

        Ok(node_registry_contract)
    }
}

#[async_trait]
impl TransactionCaller for NodeRegistryClient {}

#[async_trait]
impl ViewCaller for NodeRegistryClient {}

#[async_trait]
impl NodeRegistryTransactions for NodeRegistryClient {
    async fn node_register(&self, id_public_key: Vec<u8>) -> ContractClientResult<H256> {
        let node_registry_contract =
            ServiceClient::<NodeRegistryContract>::prepare_service_client(self).await?;

        let empty_signature = SignatureWithSaltAndExpiry {
            signature: vec![0u8; 65].into(),
            salt: [0u8; 32],
            expiry: 0u64.into(),
        };

        let call = node_registry_contract.node_register(id_public_key.into(), empty_signature);

        NodeRegistryClient::call_contract_transaction(
            self.chain_id,
            "node_register",
            node_registry_contract.client_ref(),
            call,
            self.contract_transaction_retry_descriptor,
            true,
        )
        .await
    }
}

#[async_trait]
impl NodeRegistryViews for NodeRegistryClient {
    async fn get_node(&self, id_address: Address) -> ContractClientResult<Node> {
        let node_registry_contract =
            ServiceClient::<NodeRegistryContract>::prepare_service_client(self).await?;

        NodeRegistryClient::call_contract_view(
            self.chain_id,
            "get_node",
            node_registry_contract.get_node(id_address),
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
}
