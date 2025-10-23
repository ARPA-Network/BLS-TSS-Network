use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::{node_activation::NodeActivation, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
};
use arpa_contract_client::{error::ContractClientError, node_registry::NodeRegistryTransactions};
use arpa_core::log::{build_general_payload, build_transaction_receipt_payload, LogType};
use async_trait::async_trait;
use log::{debug, error, info};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct NodeActivationSubscriber<PC: Curve> {
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    eq: Arc<RwLock<EventQueue>>,
    c: PhantomData<PC>,
}

impl<PC: Curve> NodeActivationSubscriber<PC> {
    pub fn new(
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        NodeActivationSubscriber {
            chain_identity,
            eq,
            c: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> Subscriber
    for NodeActivationSubscriber<PC>
{
    async fn notify(&self, topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()> {
        debug!("{:?}", topic);

        let &NodeActivation {
            chain_id,
            is_eigenlayer,
            node_registry_address,
        } = payload.as_any().downcast_ref::<NodeActivation>().unwrap();

        let node_registry_client = self
            .chain_identity
            .read()
            .await
            .build_node_registry_client(node_registry_address);

        let receipt_result = if is_eigenlayer {
            node_registry_client
                .node_activate_as_eigenlayer_operator(self.chain_identity.read().await.get_signer())
                .await
        } else {
            node_registry_client
                .node_activate_by_consistent_native_staking()
                .await
        };

        match receipt_result {
            Ok(receipt) => {
                info!(
                    "{}",
                    build_transaction_receipt_payload(
                        LogType::NodeActivated,
                        "Node activated",
                        chain_id,
                        receipt.transaction_hash,
                        receipt.gas_used,
                        receipt.effective_gas_price,
                    )
                );
            }
            Err(e) => match e {
                ContractClientError::TransactionFailed(receipt) => {
                    error!(
                        "{}",
                        build_transaction_receipt_payload(
                            LogType::NodeActivationFailed,
                            "Node activate failed",
                            chain_id,
                            receipt.transaction_hash,
                            receipt.gas_used,
                            receipt.effective_gas_price,
                        )
                    );
                }
                _ => {
                    error!(
                        "{}",
                        build_general_payload(
                            LogType::NodeActivationFailed,
                            &format!("Node activate failed with error: {:?}", e),
                            Some(chain_id)
                        )
                    );
                }
            },
        }

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write()
            .await
            .subscribe(Topic::NodeActivation, subscriber);
    }
}

impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> DebuggableSubscriber
    for NodeActivationSubscriber<PC>
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::{node_activation::NodeActivation, types::Topic},
        queue::event_queue::EventQueue,
    };
    use alloy::node_bindings::{Anvil, AnvilInstance};
    use alloy::primitives::{Address, U256};
    use alloy::providers::{Provider, WsConnect};
    use alloy::signers::local::PrivateKeySigner;
    use alloy::sol;
    use arpa_core::{build_client, Config, GeneralMainChainIdentity, ProviderClientWithSigner};
    use std::sync::Arc;
    use threshold_bls::schemes::bn254::G2Curve;
    use tokio::sync::RwLock;

    const TEST_DKG_KEY: &[u8] = b"test_dkg_key";
    const TEST_SALT: [u8; 32] = [1u8; 32];
    const TEST_EXPIRY: u64 = 1000;
    const FIRST_GROUP_INDEX: u64 = 1;

    sol! {
        #[sol(rpc)]
        MockAVSDirectory,
        "test-contract/MockAVSDirectory.json"
    }

    sol! {
        #[sol(rpc)]
        MockServiceManager,
        "test-contract/MockServiceManager.json" 
    }

    sol! {
        #[sol(rpc)]
        MockNodeRegistry,
        "test-contract/MockNodeRegistry.json"
    }

    struct TestEnvironment {
        _anvil: AnvilInstance,
        client: ProviderClientWithSigner,
        wallet: PrivateKeySigner,
        chain_id: u64
    }

    impl TestEnvironment {
        async fn new() -> Self {
            let anvil = Anvil::new().spawn();
            let ws_connect = WsConnect::new(anvil.ws_endpoint());
            let wallet: PrivateKeySigner = anvil.keys()[0].clone().into();
            let chain_id = anvil.chain_id();
            let client = build_client(wallet.clone(), chain_id, ws_connect)
                .await
                .unwrap();

            TestEnvironment {
                _anvil: anvil,
                client,
                wallet,
                chain_id
            }
        }

        async fn deploy_mock_avs_directory(client: ProviderClientWithSigner,) -> Address {
            let contract = MockAVSDirectory::deploy(client.clone()).await;
            *contract.address()
        }

        async fn deploy_mock_service_manager(&self, avs_directory: Address) -> Address {
            let contract = MockServiceManager::deploy(client.clone()).await;
            *contract.address()
        }

        async fn deploy_mock_node_registry(
            &self,
            service_manager: Address,
        ) -> (Address, MockNodeRegistry::MockNodeRegistryInstance<ProviderClientWithSigner>) {
            let contract = MockNodeRegistry::deploy(
                &self.client.clone()
            )
            .await
            .unwrap();
            (*contract.address(), contract)
        }

        async fn setup_full_contracts(&self) -> (Address, Address, Address, MockNodeRegistry::MockNodeRegistryInstance<ProviderClientWithSigner>) {
            let avs_directory = self.deploy_mock_avs_directory().await;
            let service_manager = self.deploy_mock_service_manager(avs_directory).await;
            let (node_registry, registry_instance) =
                self.deploy_mock_node_registry(service_manager).await;
            (node_registry, service_manager, avs_directory, registry_instance)
        }

        async fn register_node(
            &self,
            registry: &MockNodeRegistry::MockNodeRegistryInstance<ProviderClientWithSigner>,
            node_address: Address,
            is_eigenlayer: bool,
        ) {
            registry
                .registerNode(node_address, TEST_DKG_KEY.to_vec().into(), is_eigenlayer)
                .send()
                .await
                .unwrap()
                .get_receipt()
                .await
                .unwrap();
        }

        async fn create_chain_identity(
            &self,
            node_registry_address: Address,
        ) -> Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> {
            let config = Config::default();
            let ws_connect = WsConnect::new(self.ws_endpoint.clone());
            let general_chain_identity = GeneralMainChainIdentity::new(
                self.chain_id.try_into().unwrap(),
                self.wallet.clone(),
                ws_connect,
                self.client.clone(),
                self.ws_endpoint.clone(),
                Address::ZERO, // controller
                Address::ZERO, // controller_relayer
                node_registry_address,
                config
                    .get_time_limits()
                    .contract_transaction_retry_descriptor,
                config.get_time_limits().contract_view_retry_descriptor,
                None,
            );
            Arc::new(RwLock::new(Box::new(general_chain_identity)))
        }
    }

    #[tokio::test]
    async fn test_node_activation_subscriber_creation() {
        let env = TestEnvironment::new().await;
        let (node_registry_address, _, _, _) = env.setup_full_contracts().await;
        let chain_identity = env.create_chain_identity(node_registry_address).await;
        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);
        assert!(format!("{:?}", subscriber).contains("NodeActivationSubscriber"));
    }

    #[tokio::test]
    async fn test_successful_eigenlayer_node_activation() {
        let env = TestEnvironment::new().await;
        let node_address = env.wallet.address();
        let (node_registry_address, service_manager_address, avs_directory_address, registry_contract) =
            env.setup_full_contracts().await;

        env.register_node(&registry_contract, node_address, true)
            .await;

        let chain_identity = env.create_chain_identity(node_registry_address).await;
        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let activation_event = NodeActivation {
            chain_id: env.chain_id as usize,
            is_eigenlayer: true,
            node_registry_address,
        };

        // Check node state before activation
        let node_before = registry_contract.getNode(node_address).call().await.unwrap();
        assert!(!node_before.state);

        // Verify service manager and avs directory setup
        let service_manager = MockServiceManager::new(service_manager_address, &env.client);
        let returned_avs_directory = service_manager.avsDirectory().call().await.unwrap()._0;
        assert_eq!(returned_avs_directory, avs_directory_address);

        let avs_directory = MockAVSDirectory::new(avs_directory_address, &env.client);
        let test_hash = avs_directory
            .calculateOperatorAVSRegistrationDigestHash(
                node_address,
                service_manager_address,
                TEST_SALT.into(),
                U256::from(TEST_EXPIRY),
            )
            .call()
            .await
            .unwrap()
            ._0;
        assert_ne!(test_hash, [0u8; 32].into());

        let result = subscriber
            .notify(Topic::NodeActivation, &activation_event)
            .await;
        assert!(result.is_ok());

        let node_after = registry_contract.getNode(node_address).call().await.unwrap();
        assert!(node_after.state);
        assert!(node_after.isEigenlayerNode);
    }

    #[tokio::test]
    async fn test_successful_native_staking_node_activation() {
        let env = TestEnvironment::new().await;
        let node_address = env.wallet.address();
        let (node_registry_address, _, _, registry_contract) = env.setup_full_contracts().await;

        env.register_node(&registry_contract, node_address, false)
            .await;

        let chain_identity = env.create_chain_identity(node_registry_address).await;
        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let activation_event = NodeActivation {
            chain_id: env.chain_id as usize,
            is_eigenlayer: false,
            node_registry_address,
        };

        let node_before = registry_contract.getNode(node_address).call().await.unwrap();
        assert!(!node_before.state);

        let result = subscriber
            .notify(Topic::NodeActivation, &activation_event)
            .await;
        assert!(result.is_ok());

        let node_after = registry_contract.getNode(node_address).call().await.unwrap();
        assert!(node_after.state);
        assert!(!node_after.isEigenlayerNode);
    }

    #[tokio::test]
    async fn test_node_activation_already_active_error() {
        let env = TestEnvironment::new().await;
        let node_address = env.wallet.address();
        let (node_registry_address, _, _, registry_contract) = env.setup_full_contracts().await;

        env.register_node(&registry_contract, node_address, false)
            .await;

        // Set node as active
        registry_contract
            .setNodeState(node_address, true)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        let chain_identity = env.create_chain_identity(node_registry_address).await;
        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let activation_event = NodeActivation {
            chain_id: env.chain_id as usize,
            is_eigenlayer: false,
            node_registry_address,
        };

        let result = subscriber
            .notify(Topic::NodeActivation, &activation_event)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_node_activation_not_registered_error() {
        let env = TestEnvironment::new().await;
        let (node_registry_address, _, _, _) = env.setup_full_contracts().await;

        let chain_identity = env.create_chain_identity(node_registry_address).await;
        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let activation_event = NodeActivation {
            chain_id: env.chain_id as usize,
            is_eigenlayer: false,
            node_registry_address,
        };

        let result = subscriber
            .notify(Topic::NodeActivation, &activation_event)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_node_activation_event_emission() {
        let env = TestEnvironment::new().await;
        let node_address = env.wallet.address();
        let (node_registry_address, _, _, registry_contract) = env.setup_full_contracts().await;

        env.register_node(&registry_contract, node_address, false)
            .await;

        let chain_identity = env.create_chain_identity(node_registry_address).await;
        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let activation_event = NodeActivation {
            chain_id: env.chain_id as usize,
            is_eigenlayer: false,
            node_registry_address,
        };

        let result = subscriber
            .notify(Topic::NodeActivation, &activation_event)
            .await;
        assert!(result.is_ok());

        // Query for NodeActivated events
        let event_filter = registry_contract.NodeActivated_filter().from_block(0);
        let events = event_filter.query().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].nodeAddress, node_address);
        assert_eq!(events[0].groupIndex, U256::from(FIRST_GROUP_INDEX));
    }

    #[tokio::test]
    async fn test_subscriber_subscribe() {
        let env = TestEnvironment::new().await;
        let (node_registry_address, _, _, _) = env.setup_full_contracts().await;
        let chain_identity = env.create_chain_identity(node_registry_address).await;

        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq.clone());
        subscriber.subscribe().await;
    }

    #[tokio::test]
    async fn test_different_chain_ids() {
        let env = TestEnvironment::new().await;
        let node_address = env.wallet.address();
        let (node_registry_address, _, _, registry_contract) = env.setup_full_contracts().await;

        env.register_node(&registry_contract, node_address, false)
            .await;

        let chain_identity = env.create_chain_identity(node_registry_address).await;
        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let test_chain_ids = vec![1, 137, 42161, 10];

        for chain_id in test_chain_ids {
            let activation_event = NodeActivation {
                chain_id,
                is_eigenlayer: false,
                node_registry_address,
            };

            let result = subscriber
                .notify(Topic::NodeActivation, &activation_event)
                .await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_node_activation_with_pending_status() {
        let env = TestEnvironment::new().await;
        let node_address = env.wallet.address();
        let (node_registry_address, _, _, registry_contract) = env.setup_full_contracts().await;

        env.register_node(&registry_contract, node_address, false)
            .await;

        let chain_identity = env.create_chain_identity(node_registry_address).await;
        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let activation_event = NodeActivation {
            chain_id: env.chain_id as usize,
            is_eigenlayer: false,
            node_registry_address,
        };

        let result = subscriber
            .notify(Topic::NodeActivation, &activation_event)
            .await;
        assert!(result.is_ok());

        let node_after = registry_contract.getNode(node_address).call().await.unwrap();
        assert!(node_after.state);
    }
}