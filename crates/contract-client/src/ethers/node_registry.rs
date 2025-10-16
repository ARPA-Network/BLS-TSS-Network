use crate::{
    // contract_stub::{
    //     i_controller::SignatureWithSaltAndExpiry, iavs_directory, node_registry::NodeRegistry,
    //     service_manager,
    // },
    error::ContractClientResult,
    ethers::{
        avs_directory,
        node_registry::{
            INodeRegistry::INodeRegistryInstance, ISignatureUtils::SignatureWithSaltAndExpiry,
        },
        service_manager,
    },
    node_registry::{NodeRegistryClientBuilder, NodeRegistryTransactions, NodeRegistryViews},
    ServiceClient,
};
use crate::{TransactionCaller, ViewCaller};
use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, U256},
    rpc::types::TransactionReceipt,
    signers::local::PrivateKeySigner,
    sol,
};
use alloy::{providers::Provider, signers::SignerSync};
use arpa_core::{
    ChainIdentity, ExponentialBackoffRetryDescriptor, GeneralMainChainIdentity,
    GeneralRelayedChainIdentity, Node, ProviderClientWithSigner,
};
use async_trait::async_trait;
use rand::Rng;

sol! {
    #[sol(ignore_unlinked)]
    #[sol(rpc)]
    INodeRegistry,
    "abi/INodeRegistry.json"
}

pub struct NodeRegistryClient {
    chain_id: u64,
    id_address: Address,
    node_registry_address: Address,
    client: ProviderClientWithSigner,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    max_priority_fee_per_gas: Option<u128>,
}

impl NodeRegistryClient {
    pub fn new(
        chain_id: u64,
        node_registry_address: Address,
        identity: &GeneralMainChainIdentity,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
        max_priority_fee_per_gas: Option<u128>,
    ) -> Self {
        NodeRegistryClient {
            chain_id,
            id_address: identity.get_id_address(),
            node_registry_address,
            client: identity.get_client(),
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
            max_priority_fee_per_gas,
        }
    }

    async fn build_signature_with_salt_and_expiry(
        &self,
        node_registry_contract: &NodeRegistryContract,
        signer: &PrivateKeySigner,
    ) -> ContractClientResult<SignatureWithSaltAndExpiry> {
        let service_manager_address = node_registry_contract
            .getNodeRegistryConfig()
            .call()
            .await?
            .serviceManagerContractAddress;
        let service_manager_contract =
            service_manager::ServiceManager::new(service_manager_address, self.client.clone());
        let avs_directory_address = service_manager_contract.avsDirectory().call().await?;
        let avs_directory_contract =
            avs_directory::IAVSDirectory::new(avs_directory_address, self.client.clone());
        // generate random salt
        let salt = rand::thread_rng().gen::<[u8; 32]>().into();

        let expiry = self
            .client
            .get_block(alloy::eips::BlockId::Number(BlockNumberOrTag::Latest))
            .await
            .map(|o| o.map(|b| b.header.timestamp))?
            .unwrap()
            + 1000;

        let digest_hash = avs_directory_contract
            .calculateOperatorAVSRegistrationDigestHash(
                signer.address(),
                service_manager_address,
                salt,
                U256::from(expiry),
            )
            .call()
            .await?;
        let signature = signer
            .sign_hash_sync(&digest_hash)?
            .as_bytes()
            .to_vec()
            .into();

        Ok(SignatureWithSaltAndExpiry {
            signature,
            salt,
            expiry: U256::from(expiry),
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
            self.get_max_priority_fee_per_gas(),
        )
    }
}

impl NodeRegistryClientBuilder for GeneralRelayedChainIdentity {
    type NodeRegistryService = NodeRegistryClient;

    fn build_node_registry_client(&self, _node_registry_address: Address) -> NodeRegistryClient {
        panic!("not implemented")
    }
}

type NodeRegistryContract = INodeRegistryInstance<ProviderClientWithSigner>;

#[async_trait]
impl ServiceClient<NodeRegistryContract> for NodeRegistryClient {
    async fn prepare_service_client(&self) -> ContractClientResult<NodeRegistryContract> {
        let node_registry_contract =
            INodeRegistry::new(self.node_registry_address, self.client.clone());

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
        asset_account_signer: &PrivateKeySigner,
    ) -> ContractClientResult<TransactionReceipt> {
        let node_registry_contract =
            ServiceClient::<NodeRegistryContract>::prepare_service_client(self).await?;

        let signature = self
            .build_signature_with_salt_and_expiry(&node_registry_contract, asset_account_signer)
            .await?;

        let call = node_registry_contract.nodeRegister(
            id_public_key.into(),
            true,
            asset_account_signer.address(),
            signature,
        );

        NodeRegistryClient::call_contract_transaction(
            self.chain_id,
            "node_register",
            node_registry_contract.provider(),
            call,
            self.contract_transaction_retry_descriptor,
            true,
            self.max_priority_fee_per_gas,
        )
        .await
    }

    async fn node_register_by_consistent_native_staking(
        &self,
        id_public_key: Vec<u8>,
    ) -> ContractClientResult<TransactionReceipt> {
        let node_registry_contract =
            ServiceClient::<NodeRegistryContract>::prepare_service_client(self).await?;

        let call = node_registry_contract.nodeRegister(
            id_public_key.into(),
            false,
            self.id_address,
            SignatureWithSaltAndExpiry {
                signature: vec![0u8; 65].into(),
                salt: [0u8; 32].into(),
                expiry: U256::ZERO,
            },
        );

        NodeRegistryClient::call_contract_transaction(
            self.chain_id,
            "node_register",
            node_registry_contract.provider(),
            call,
            self.contract_transaction_retry_descriptor,
            true,
            self.max_priority_fee_per_gas,
        )
        .await
    }

    async fn node_activate_as_eigenlayer_operator(
        &self,
        asset_account_signer: &PrivateKeySigner,
    ) -> ContractClientResult<TransactionReceipt> {
        let node_registry_contract =
            ServiceClient::<NodeRegistryContract>::prepare_service_client(self).await?;

        let signature = self
            .build_signature_with_salt_and_expiry(&node_registry_contract, asset_account_signer)
            .await?;

        let call = node_registry_contract.nodeActivate(signature);

        NodeRegistryClient::call_contract_transaction(
            self.chain_id,
            "node_activate",
            node_registry_contract.provider(),
            call,
            self.contract_transaction_retry_descriptor,
            true,
            self.max_priority_fee_per_gas,
        )
        .await
    }

    async fn node_activate_by_consistent_native_staking(
        &self,
    ) -> ContractClientResult<TransactionReceipt> {
        let node_registry_contract =
            ServiceClient::<NodeRegistryContract>::prepare_service_client(self).await?;

        let call = node_registry_contract.nodeActivate(SignatureWithSaltAndExpiry {
            signature: vec![0u8; 65].into(),
            salt: [0u8; 32].into(),
            expiry: U256::ZERO,
        });

        NodeRegistryClient::call_contract_transaction(
            self.chain_id,
            "node_activate",
            node_registry_contract.provider(),
            call,
            self.contract_transaction_retry_descriptor,
            true,
            self.max_priority_fee_per_gas,
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
            node_registry_contract.getNode(id_address),
            self.contract_view_retry_descriptor,
        )
        .await
        .map(|n| Node {
            id_address: n.idAddress,
            id_public_key: n.dkgPublicKey.to_vec(),
            is_eigenlayer_node: n.isEigenlayerNode,
            state: n.state,
            pending_until_block: n.pendingUntilBlock.to::<usize>(),
        })
    }
}
