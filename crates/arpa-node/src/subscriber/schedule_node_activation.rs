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
use ethers::{providers::Middleware, types::U256};
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
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
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
                .node_activate_as_eigenlayer_operator(
                    self.chain_identity
                        .read()
                        .await
                        .get_client()
                        .inner()
                        .signer(),
                )
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
                        receipt.gas_used.unwrap_or(U256::zero()),
                        receipt.effective_gas_price.unwrap_or(U256::zero()),
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
                            receipt.gas_used.unwrap_or(U256::zero()),
                            receipt.effective_gas_price.unwrap_or(U256::zero()),
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

#[cfg(feature = "unittest")]
mod tests {
    use super::*;
    use crate::{
        event::{node_activation::NodeActivation, types::Topic},
        queue::event_queue::EventQueue,
        test_contracts::{
            mockavsdirectory::{deploy_mock_a_v_s_directory, get_mock_a_v_s_directory_at}, 
            mocknoderegistry::{deploy_mock_node_registry_with_args, get_mock_node_registry_at, MockNodeRegistry},
            mockservicemanager::{deploy_mock_service_manager_with_args, get_mock_service_manager_at},
        },
    };
    use arpa_contract_client::contract_stub::node_registry::NodeActivatedFilter;
    use arpa_core::{Config, GeneralMainChainIdentity};
    use ethers::prelude::*;
    use std::sync::Arc;
    use threshold_bls::schemes::bn254::G2Curve;
    use tokio::sync::RwLock;

    const TEST_DKG_KEY: &[u8] = b"test_dkg_key";
    const TEST_SALT: [u8; 32] = [1u8; 32];
    const TEST_EXPIRY: u64 = 1000;
    const FIRST_GROUP_INDEX: u64 = 1;

    async fn setup_anvil() -> (ethers::utils::AnvilInstance, Arc<Provider<Ws>>, LocalWallet) {
        let anvil = ethers::utils::Anvil::new().spawn();
        let ws_provider = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await.unwrap());
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let wallet = wallet.with_chain_id(anvil.chain_id());
        (anvil, ws_provider, wallet)
    }

    async fn setup_mock_contracts_with_dependencies(
        ws_provider: Arc<Provider<Ws>>,
        wallet: LocalWallet,
    ) -> (Address, Address, Address) { 
        let client = Arc::new(SignerMiddleware::new((*ws_provider).clone(), wallet.clone()));
        let avs_directory_address = deploy_mock_a_v_s_directory(client.clone()).await.unwrap();
        let service_manager_address = deploy_mock_service_manager_with_args(client.clone(), avs_directory_address).await.unwrap();
        let node_registry_address = deploy_mock_node_registry_with_args(
            client.clone(),
            (Address::random(), Address::random(), service_manager_address)
        ).await.unwrap();
        (node_registry_address, service_manager_address, avs_directory_address)
    }

    async fn setup_node_registry_with_node_and_dependencies(
        ws_provider: Arc<Provider<Ws>>,
        wallet: LocalWallet,
        node_address: Address,
        is_eigenlayer: bool,
    ) -> (Address, Address, Address, MockNodeRegistry<SignerMiddleware<Provider<Ws>, LocalWallet>>) {
        let client = Arc::new(SignerMiddleware::new((*ws_provider).clone(), wallet.clone()));
        let (registry_address, service_manager_address, avs_directory_address) = 
            setup_mock_contracts_with_dependencies(ws_provider, wallet).await;
        let registry_contract = get_mock_node_registry_at(registry_address, client);
        registry_contract
            .register_node(node_address, TEST_DKG_KEY.to_vec().into(), is_eigenlayer)
            .send().await.unwrap().await.unwrap();
        (registry_address, service_manager_address, avs_directory_address, registry_contract)
    }

    async fn deploy_node_registry(ws_provider: Arc<Provider<Ws>>, wallet: LocalWallet) -> Address {
        let client = Arc::new(SignerMiddleware::new((*ws_provider).clone(), wallet.clone()));
        deploy_mock_node_registry_with_args(
            client,
            (Address::random(), Address::random(), Address::random())
        ).await.unwrap()
    }

    async fn create_chain_identity(
        chain_id: u64,
        wallet: LocalWallet,
        ws_provider: Arc<Provider<Ws>>,
        ws_endpoint: String,
        node_registry_address: Address,
    ) -> Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> {
        let config = Config::default();
        let general_chain_identity = GeneralMainChainIdentity::new(
            chain_id.try_into().unwrap(),
            wallet.clone(),
            ws_provider.clone(),
            ws_endpoint,
            Address::random(), 
            Address::random(),            
            node_registry_address,  
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );
        Arc::new(RwLock::new(Box::new(general_chain_identity)))
    }

    async fn setup_node_registry_with_node(
        ws_provider: Arc<Provider<Ws>>,
        wallet: LocalWallet,
        node_address: Address,
        is_eigenlayer: bool,
    ) -> (Address, MockNodeRegistry<SignerMiddleware<Provider<Ws>, LocalWallet>>) {
        let client = Arc::new(SignerMiddleware::new((*ws_provider).clone(), wallet.clone()));
        let registry_address = deploy_mock_node_registry_with_args(
            client.clone(),
            (Address::random(), Address::random(), Address::random())
        ).await.unwrap();
        let registry_contract = get_mock_node_registry_at(registry_address, client);
        registry_contract
            .register_node(node_address, TEST_DKG_KEY.to_vec().into(), is_eigenlayer)
            .send().await.unwrap().await.unwrap();
        (registry_address, registry_contract)
    }

    #[tokio::test]
    async fn test_node_activation_subscriber_creation() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let node_registry_address = deploy_node_registry(ws_provider.clone(), wallet.clone()).await;
        let chain_identity = create_chain_identity(
            1,
            wallet,
            ws_provider,
            "ws://localhost:8545".to_string(),
            node_registry_address,
        ).await;
        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);
        assert!(format!("{:?}", subscriber).contains("NodeActivationSubscriber"));
    }

    #[tokio::test]
    async fn test_successful_eigenlayer_node_activation() {
        let (anvil, ws_provider, wallet) = setup_anvil().await;
        let node_address = wallet.address();
        let (node_registry_address, service_manager_address, avs_directory_address, registry_contract) = 
            setup_node_registry_with_node_and_dependencies(
                ws_provider.clone(), wallet.clone(), node_address, true
            ).await;

        let chain_identity = create_chain_identity(
            anvil.chain_id(), wallet.clone(), ws_provider.clone(),
            anvil.ws_endpoint(), node_registry_address
        ).await;

        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let activation_event = NodeActivation {
            chain_id: anvil.chain_id() as usize,
            is_eigenlayer: true,
            node_registry_address,
        };

        let node_before = registry_contract.get_node(node_address).call().await.unwrap();
        assert!(!node_before.state);
        
        let client = Arc::new(SignerMiddleware::new((*ws_provider).clone(), wallet.clone()));
        let config = registry_contract.get_node_registry_config().call().await.unwrap();
        assert_eq!(config.2, service_manager_address); 
        
        let service_manager_contract = get_mock_service_manager_at(service_manager_address, client.clone());
        let returned_avs_directory = service_manager_contract.avs_directory().call().await.unwrap();
        assert_eq!(returned_avs_directory, avs_directory_address);
        
        let avs_directory_contract = get_mock_a_v_s_directory_at(avs_directory_address, client);
        let test_hash = avs_directory_contract
            .calculate_operator_avs_registration_digest_hash(
                node_address, service_manager_address, TEST_SALT, TEST_EXPIRY.into()
            )
            .call().await.unwrap();
        assert_ne!(test_hash, [0u8; 32]);

        let result = subscriber.notify(Topic::NodeActivation, &activation_event).await;
        assert!(result.is_ok());

        let node_after = registry_contract.get_node(node_address).call().await.unwrap();
        assert!(node_after.state);
        assert!(node_after.is_eigenlayer_node);
    }

    #[tokio::test]
    async fn test_successful_native_staking_node_activation() {
        let (anvil, ws_provider, wallet) = setup_anvil().await;
        let node_address = wallet.address();

        let (node_registry_address, registry_contract) = setup_node_registry_with_node(
            ws_provider.clone(), wallet.clone(), node_address, false
        ).await;

        let chain_identity = create_chain_identity(
            anvil.chain_id(), wallet, ws_provider, anvil.ws_endpoint(), node_registry_address
        ).await;

        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let activation_event = NodeActivation {
            chain_id: anvil.chain_id() as usize,
            is_eigenlayer: false,
            node_registry_address,
        };

        let node_before = registry_contract.get_node(node_address).call().await.unwrap();
        assert!(!node_before.state);

        let result = subscriber.notify(Topic::NodeActivation, &activation_event).await;
        assert!(result.is_ok());

        let node_after = registry_contract.get_node(node_address).call().await.unwrap();
        assert!(node_after.state);
        assert!(!node_after.is_eigenlayer_node);
    }

    #[tokio::test]
    async fn test_node_activation_already_active_error() {
        let (anvil, ws_provider, wallet) = setup_anvil().await;
        let node_address = wallet.address();

        let (node_registry_address, registry_contract) = setup_node_registry_with_node(
            ws_provider.clone(), wallet.clone(), node_address, false
        ).await;

        registry_contract.set_node_state(node_address, true).send().await.unwrap().await.unwrap();

        let chain_identity = create_chain_identity(
            anvil.chain_id(), wallet, ws_provider, anvil.ws_endpoint(), node_registry_address
        ).await;

        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let activation_event = NodeActivation {
            chain_id: anvil.chain_id() as usize,
            is_eigenlayer: false,
            node_registry_address,
        };

        let result = subscriber.notify(Topic::NodeActivation, &activation_event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_node_activation_not_registered_error() {
        let (anvil, ws_provider, wallet) = setup_anvil().await;
        let node_registry_address = deploy_node_registry(ws_provider.clone(), wallet.clone()).await;

        let chain_identity = create_chain_identity(
            anvil.chain_id(), wallet, ws_provider, anvil.ws_endpoint(), node_registry_address
        ).await;

        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let activation_event = NodeActivation {
            chain_id: anvil.chain_id() as usize,
            is_eigenlayer: false,
            node_registry_address,
        };

        let result = subscriber.notify(Topic::NodeActivation, &activation_event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_node_activation_event_emission() {
        let (anvil, ws_provider, wallet) = setup_anvil().await;
        let node_address = wallet.address();

        let (node_registry_address, _registry_contract) = setup_node_registry_with_node(
            ws_provider.clone(), wallet.clone(), node_address, false
        ).await;

        let chain_identity = create_chain_identity(
            anvil.chain_id(), wallet, ws_provider.clone(), anvil.ws_endpoint(), node_registry_address
        ).await;

        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let activation_event = NodeActivation {
            chain_id: anvil.chain_id() as usize,
            is_eigenlayer: false,
            node_registry_address,
        };

        let event_filter = MockNodeRegistry::new(node_registry_address, ws_provider.clone())
            .event::<NodeActivatedFilter>();

        let result = subscriber.notify(Topic::NodeActivation, &activation_event).await;
        assert!(result.is_ok());

        let events = event_filter.from_block(0).query().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].node_address, node_address);
        assert_eq!(events[0].group_index, U256::from(FIRST_GROUP_INDEX)); 
    }

    #[tokio::test]
    async fn test_subscriber_subscribe() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let node_registry_address = deploy_node_registry(ws_provider.clone(), wallet.clone()).await;
        let chain_identity = create_chain_identity(
            1, wallet, ws_provider, "ws://localhost:8545".to_string(), node_registry_address
        ).await;

        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq.clone());
        subscriber.subscribe().await;
    }

    #[tokio::test]
    async fn test_different_chain_ids() {
        let (anvil, ws_provider, wallet) = setup_anvil().await;
        let node_address = wallet.address();

        let (node_registry_address, _registry_contract) = setup_node_registry_with_node(
            ws_provider.clone(), wallet.clone(), node_address, false
        ).await;

        let chain_identity = create_chain_identity(
            anvil.chain_id(), wallet, ws_provider, anvil.ws_endpoint(), node_registry_address
        ).await;

        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let test_chain_ids = vec![1, 137, 42161, 10];

        for chain_id in test_chain_ids {
            let activation_event = NodeActivation {
                chain_id,
                is_eigenlayer: false,
                node_registry_address,
            };

            let result = subscriber.notify(Topic::NodeActivation, &activation_event).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_node_activation_with_pending_status() {
        let (anvil, ws_provider, wallet) = setup_anvil().await;
        let node_address = wallet.address();

        let (node_registry_address, registry_contract) = setup_node_registry_with_node(
            ws_provider.clone(), wallet.clone(), node_address, false
        ).await;

        let chain_identity = create_chain_identity(
            anvil.chain_id(), wallet, ws_provider, anvil.ws_endpoint(), node_registry_address
        ).await;

        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = NodeActivationSubscriber::new(chain_identity, eq);

        let activation_event = NodeActivation {
            chain_id: anvil.chain_id() as usize,
            is_eigenlayer: false,
            node_registry_address,
        };

        let result = subscriber.notify(Topic::NodeActivation, &activation_event).await;
        assert!(result.is_ok());

        let node_after = registry_contract.get_node(node_address).call().await.unwrap();
        assert!(node_after.state);
    }
}