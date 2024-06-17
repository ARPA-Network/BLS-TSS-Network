use crate::{
    contract_stub::{
        i_controller::SignatureWithSaltAndExpiry, iavs_directory, node_registry::NodeRegistry,
        service_manager,
    },
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
use ethers::{core::rand::Rng, prelude::*};
use std::sync::Arc;

pub struct NodeRegistryClient {
    chain_id: usize,
    id_address: Address,
    node_registry_address: Address,
    client: Arc<WsWalletSigner>,
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
            id_address: identity.get_id_address(),
            node_registry_address,
            client: identity.get_client(),
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
        }
    }

    async fn build_signature_with_salt_and_expiry(
        &self,
        node_registry_contract: &NodeRegistryContract,
        signer: &LocalWallet,
    ) -> ContractClientResult<SignatureWithSaltAndExpiry> {
        let service_manager_address = node_registry_contract.get_node_registry_config().await?.2;
        let service_manager_contract =
            service_manager::ServiceManager::new(service_manager_address, self.client.clone());
        let avs_directory_address = service_manager_contract.avs_directory().await?;
        let avs_directory_contract =
            iavs_directory::IAVSDirectory::new(avs_directory_address, self.client.clone());
        // generate random salt
        let salt = rand::thread_rng().gen::<[u8; 32]>();

        let expiry = self
            .client
            .provider()
            .get_block(BlockNumber::Latest)
            .await
            .map(|o| o.map(|b| b.timestamp))?
            .unwrap()
            + 1000;

        let digest_hash = avs_directory_contract
            .calculate_operator_avs_registration_digest_hash(
                self.client.inner().address(),
                service_manager_address,
                salt,
                expiry,
            )
            .await?;
        let signature = signer.sign_hash(digest_hash.into())?.to_vec().into();

        Ok(SignatureWithSaltAndExpiry {
            signature,
            salt,
            expiry,
        })
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
            NodeRegistry::new(self.node_registry_address, self.client.clone());

        Ok(node_registry_contract)
    }
}

#[async_trait]
impl TransactionCaller for NodeRegistryClient {}

#[async_trait]
impl ViewCaller for NodeRegistryClient {}

#[async_trait]
impl NodeRegistryTransactions for NodeRegistryClient {
    async fn node_register_as_eigenlayer_operator(
        &self,
        id_public_key: Vec<u8>,
        asset_account_signer: &LocalWallet,
    ) -> ContractClientResult<TransactionReceipt> {
        let node_registry_contract =
            ServiceClient::<NodeRegistryContract>::prepare_service_client(self).await?;

        let signature = self
            .build_signature_with_salt_and_expiry(&node_registry_contract, asset_account_signer)
            .await?;

        let call = node_registry_contract.node_register(
            id_public_key.into(),
            true,
            asset_account_signer.address(),
            signature,
        );

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

    async fn node_register_by_consistent_native_staking(
        &self,
        id_public_key: Vec<u8>,
    ) -> ContractClientResult<TransactionReceipt> {
        let node_registry_contract =
            ServiceClient::<NodeRegistryContract>::prepare_service_client(self).await?;

        let call = node_registry_contract.node_register(
            id_public_key.into(),
            false,
            self.id_address,
            SignatureWithSaltAndExpiry {
                signature: vec![0u8; 65].into(),
                salt: [0u8; 32],
                expiry: 0u64.into(),
            },
        );

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

    async fn node_activate_as_eigenlayer_operator(
        &self,
        asset_account_signer: &LocalWallet,
    ) -> ContractClientResult<TransactionReceipt> {
        let node_registry_contract =
            ServiceClient::<NodeRegistryContract>::prepare_service_client(self).await?;

        let signature = self
            .build_signature_with_salt_and_expiry(&node_registry_contract, asset_account_signer)
            .await?;

        let call = node_registry_contract.node_activate(signature);

        NodeRegistryClient::call_contract_transaction(
            self.chain_id,
            "node_activate",
            node_registry_contract.client_ref(),
            call,
            self.contract_transaction_retry_descriptor,
            true,
        )
        .await
    }

    async fn node_activate_by_consistent_native_staking(
        &self,
    ) -> ContractClientResult<TransactionReceipt> {
        let node_registry_contract =
            ServiceClient::<NodeRegistryContract>::prepare_service_client(self).await?;

        let call = node_registry_contract.node_activate(SignatureWithSaltAndExpiry {
            signature: vec![0u8; 65].into(),
            salt: [0u8; 32],
            expiry: 0u64.into(),
        });

        NodeRegistryClient::call_contract_transaction(
            self.chain_id,
            "node_activate",
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
            is_eigenlayer_node: n.is_eigenlayer_node,
            state: n.state,
            pending_until_block: n.pending_until_block.as_usize(),
        })
    }
}
