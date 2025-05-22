use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::node_activation::NodeActivation,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::{controller::ControllerViews, node_registry::NodeRegistryViews};
use arpa_core::ListenerDescriptor;
use async_trait::async_trait;
use ethers::{providers::Middleware, types::Address};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct NodeActivationListener<PC: Curve> {
    listener_descriptor: ListenerDescriptor,
    is_eigenlayer: bool,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    node_registry_address: Option<Address>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> std::fmt::Display for NodeActivationListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeActivationListener")
    }
}

impl<PC: Curve> NodeActivationListener<PC> {
    pub fn new(
        listener_descriptor: ListenerDescriptor,
        is_eigenlayer: bool,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        NodeActivationListener {
            listener_descriptor,
            is_eigenlayer,
            chain_identity,
            node_registry_address: None,
            eq,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> EventPublisher<NodeActivation> for NodeActivationListener<PC> {
    async fn publish(&self, event: NodeActivation) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for NodeActivationListener<PC> {
    async fn initialize(&mut self) -> NodeResult<()> {
        if self.node_registry_address.is_none() {
            let controller_client = self.chain_identity.read().await.build_controller_client();

            let node_registry_address =
                ControllerViews::<PC>::get_node_registry_address(&controller_client).await?;

            self.node_registry_address = Some(node_registry_address);
        }

        Ok(())
    }

    async fn listen(&self) -> NodeResult<()> {
        let self_address = self.chain_identity.read().await.get_id_address();

        let node_registry_address = self.node_registry_address.unwrap();

        let node_registry_client = self
            .chain_identity
            .read()
            .await
            .build_node_registry_client(node_registry_address);

        let node = node_registry_client.get_node(self_address).await?;

        if node.id_address == self_address && !node.state {
            self.publish(NodeActivation {
                chain_id: self.listener_descriptor.chain_id,
                is_eigenlayer: self.is_eigenlayer,
                node_registry_address,
            })
            .await;
        }

        Ok(())
    }

    async fn handle_interruption(&self) -> NodeResult<()> {
        self.chain_identity
            .read()
            .await
            .get_provider()
            .get_net_version()
            .await?;

        Ok(())
    }

    fn chain_id(&self) -> usize {
        self.listener_descriptor.chain_id
    }

    fn listener_descriptor(&self) -> ListenerDescriptor {
        self.listener_descriptor
    }
}

#[cfg(feature = "unittest")]
mod tests {
    use super::*;
    use ethers::signers::{LocalWallet, Signer};
    use ethers::middleware::SignerMiddleware;
    use threshold_bls::schemes::bn254::G2Curve;
    use crate::queue::EventSubscriber;
    use crate::event::types::Topic;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use crate::test_contracts::mockcontroller:: deploy_with_args_and_get_mock_controller;
    use crate::test_contracts::mocknoderegistry::{
        MockNodeRegistry, deploy_and_get_mock_node_registry
    };
    
    use arpa_core::{
        Config, FixedIntervalRetryDescriptor, GeneralMainChainIdentity, ListenerType
    };
    use ethers::{
        providers::{Provider, Ws, Http},
        types::{Address, Bytes},
        utils::Anvil,
    };
    use std::time::Duration;
    use tokio::time::timeout;
    use anyhow::anyhow;
    use std::sync::Arc;

    async fn setup_mock_contracts(
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
        node_address: Address,
        is_node_active: bool
    ) -> Result<(Address, Address), Box<dyn std::error::Error>> {
        println!("Deploying mock contracts...");

        let node_registry = deploy_and_get_mock_node_registry(client.clone()).await?;
        let node_registry_address = node_registry.address();
        println!("Node Registry contract deployed at: {}", node_registry_address);

        let controller = deploy_with_args_and_get_mock_controller(
            client.clone(),
            node_registry_address
        ).await?;
        let controller_address = controller.address();
        println!("Controller contract deployed at: {}", controller_address);

        let tx = node_registry.register_node(
            node_address, 
            Bytes::from(vec![1, 2, 3]), 
            false
        );
        
        let receipt = tx.send().await?
            .await?;
        println!("Node registered: {} in block {}", 
            node_address, receipt.unwrap().block_number.unwrap());

        if is_node_active {
            let tx = node_registry.set_node_state(node_address, true);
            let receipt = tx.send().await?
                .await?;
            println!("Node state set to active in block {}", 
                receipt.unwrap().block_number.unwrap());
        } else {
            println!("Node state kept as inactive");
        }

        Ok((controller_address, node_registry_address))
    }
    
    async fn mock_subscribe_to_events(
        eq: &mut EventQueue,
        subscriber_name: &str,
    ) -> tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>> {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
        
        struct TestSubscriber {
            name: String,
            sender: tokio::sync::mpsc::Sender<Box<dyn std::any::Any + Send>>,
        }
        
        impl std::fmt::Debug for TestSubscriber {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "TestSubscriber({})", self.name)
            }
        }
        
        #[async_trait]
        impl Subscriber for TestSubscriber {
            async fn notify(&self, _topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()> {
                println!("TestSubscriber received notification");
                if let Some(event) = payload.as_any().downcast_ref::<NodeActivation>() {
                    println!("Received NodeActivation event");
                    let cloned_event = NodeActivation {
                        chain_id: event.chain_id,
                        is_eigenlayer: event.is_eigenlayer,
                        node_registry_address: event.node_registry_address,
                    };
                    let boxed = Box::new(cloned_event) as Box<dyn std::any::Any + Send>;
                    self.sender.send(boxed).await.map_err(|e| {
                        println!("Failed to send event: {}", e);
                        let err: crate::error::NodeError = anyhow!("Failed to send event: {}", e).into();
                        err
                    })?;
                    println!("Event sent to receiver");
                } else {
                    println!("Payload is not a NodeActivation event");
                }
                Ok(())
            }
            
            async fn subscribe(self) {
                println!("TestSubscriber subscribed");
            }
        }
        
        impl DebuggableSubscriber for TestSubscriber {}
        
        let subscriber = TestSubscriber {
            name: subscriber_name.to_string(),
            sender,
        };

        let topic = Topic::NodeActivation;
        println!("Subscribing to topic: {:?}", topic);
        
        eq.subscribe(topic, Box::new(subscriber));
        println!("Subscribed to event queue");
        
        receiver
    }
    
    #[tokio::test]
    async fn test_node_activation_listener() -> NodeResult<()> {
        println!("Starting test_node_activation_listener");
        
        let anvil = Anvil::new().spawn();
        println!("Anvil instance started at {}", anvil.endpoint());
        
        let http_provider = Provider::<Http>::try_from(anvil.endpoint())
        .map_err(|e| anyhow!("Failed to create HTTP provider: {}", e))?;
        
        let ws_provider = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await?);
        println!("Connected to Anvil WebSocket at {}", anvil.ws_endpoint());
        
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let id_address = wallet.address();
        println!("Using wallet address: {}", id_address);
        
        let chain_id = anvil.chain_id() as usize;
        println!("Chain ID: {}", chain_id);
        
        let client = Arc::new(SignerMiddleware::new(
            http_provider,
            wallet.clone().with_chain_id(anvil.chain_id()),
        ));
        
        let (controller_address, node_registry_address) = setup_mock_contracts(
            client.clone(), 
            id_address,
            false, 
        ).await.map_err(|e| anyhow!("Failed to deploy mock contracts: {}", e))?;
        
        println!("Controller deployed at: {}", controller_address);
        println!("Node Registry deployed at: {}", node_registry_address);
        
        let config = Config::default();
        
        let chain_identity = GeneralMainChainIdentity::new(
            chain_id,
            wallet.clone(),
            ws_provider.clone(),
            anvil.ws_endpoint(),
            controller_address,
            Address::random(), 
            node_registry_address,
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );
        println!("Chain identity created");
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        println!("Event queue created");
        
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            println!("Setting up test subscriber");
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber").await
        };
        
        let is_eigenlayer = false;
        let listener_descriptor = ListenerDescriptor {
            chain_id,
            l_type: ListenerType::ScheduleNodeActivation,
            interval_millis: 1000, 
            use_jitter: true,      
            reset_descriptor: FixedIntervalRetryDescriptor {
                interval_millis: 5000, 
                max_attempts: 3,       
                use_jitter: true,      
            },
        };
        
        let mut listener = NodeActivationListener::<G2Curve>::new(
            listener_descriptor,
            is_eigenlayer,
            chain_identity_arc.clone(),
            event_queue.clone(),
        );
        println!("Listener created");
        
        println!("Testing initialize method");
        let init_result = listener.initialize().await;
        assert!(init_result.is_ok(), "Initialize method should not fail");
        assert!(listener.node_registry_address.is_some(), "Node registry address should be set after initialization");
        assert_eq!(listener.node_registry_address.unwrap(), node_registry_address, "Node registry address does not match");
        
        println!("Testing direct publishing through listener");
        listener.publish(NodeActivation {
            chain_id,
            is_eigenlayer,
            node_registry_address,
        }).await;
        
        println!("Waiting for published event...");
        let received_event = timeout(Duration::from_secs(5), event_receiver.recv()).await
            .map_err(|_| {
                println!("Timeout: No event received after 5 seconds");
                anyhow!("Timeout: No event received")
            })?
            .ok_or_else(|| {
                println!("Error: Event channel closed");
                anyhow!("Error: Event channel closed")
            })?;
        
        println!("Event received!");
        
        if let Some(event) = received_event.downcast_ref::<NodeActivation>() {
            println!("Received NodeActivation event");
            assert_eq!(event.chain_id, chain_id);
            assert_eq!(event.is_eigenlayer, is_eigenlayer);
            assert_eq!(event.node_registry_address, node_registry_address);
        } else {
            println!("Received unexpected event type");
            return Err(anyhow!("Received unexpected event type").into());
        }
        
        println!("Testing listen method - should generate event since node state is false");
        let listen_result = listener.listen().await;
        assert!(listen_result.is_ok(), "Listen method should not fail");
        
        println!("Waiting for listen-triggered event...");
        let listen_triggered_event = timeout(Duration::from_secs(5), event_receiver.recv()).await
            .map_err(|_| {
                println!("Timeout: No listen-triggered event received");
                anyhow!("Timeout: No listen-triggered event received")
            })?
            .ok_or_else(|| {
                println!("Error: Event channel closed");
                anyhow!("Error: Event channel closed")
            })?;
            
        if let Some(event) = listen_triggered_event.downcast_ref::<NodeActivation>() {
            println!("Received NodeActivation event from listen method");
            assert_eq!(event.chain_id, chain_id);
            assert_eq!(event.is_eigenlayer, is_eigenlayer);
            assert_eq!(event.node_registry_address, node_registry_address);
        } else {
            println!("Received unexpected event type from listen method");
            return Err(anyhow!("Received unexpected event type from listen method").into());
        }
        
        println!("\nSetting node to active state...");
        let node_registry = MockNodeRegistry::new(node_registry_address, client.clone());
        let tx = node_registry.set_node_state(id_address, true);
        let pending_tx = tx.send().await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        let receipt = pending_tx.await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?;
        println!("Node state set to active in block {}", receipt.unwrap().block_number.unwrap());
        
        let new_event_queue = Arc::new(RwLock::new(EventQueue::new()));
        let mut event_receiver = {
            let mut eq_write = new_event_queue.write().await;
            println!("Setting up new test subscriber");
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber_2").await
        };
        
        let mut listener_for_active_node = NodeActivationListener::<G2Curve>::new(
            listener_descriptor,
            is_eigenlayer,
            chain_identity_arc.clone(),
            new_event_queue.clone(),
        );
        listener_for_active_node.initialize().await?;
        
        println!("Testing listen method with active node - should NOT generate an event");
        listener_for_active_node.listen().await?;
        
        let timeout_result = timeout(Duration::from_millis(500), event_receiver.recv()).await;
        assert!(timeout_result.is_err(), "Unexpectedly received an event when node is active");
        
        println!("Testing handle_interruption method");
        let interruption_result = listener.handle_interruption().await;
        assert!(interruption_result.is_ok(), "Handle interruption failed");
        
        println!("Testing chain_id method");
        assert_eq!(listener.chain_id(), chain_id);
        
        let display_string = format!("{}", listener);
        assert_eq!(display_string, "NodeActivationListener");
        
        println!("Test completed successfully");
        Ok(())
    }
    
    #[tokio::test]
    async fn test_node_activation_listener_with_eigenlayer() -> NodeResult<()> {
        println!("Starting test_node_activation_listener_with_eigenlayer");
        
        let anvil = Anvil::new().spawn();
        let http_provider = Provider::<Http>::try_from(anvil.endpoint()).unwrap();
        let ws_provider = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await?);
        
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let id_address = wallet.address();
        let chain_id = anvil.chain_id() as usize;
        
        let client = Arc::new(SignerMiddleware::new(
            http_provider,
            wallet.clone().with_chain_id(anvil.chain_id()),
        ));
        
        let node_registry = deploy_and_get_mock_node_registry(client.clone()).await.map_err(|e| anyhow!("Failed to deploy mock node registry: {}", e))?;
        let node_registry_address = node_registry.address();
        
        let controller = deploy_with_args_and_get_mock_controller(
            client.clone(),
            node_registry_address
        ).await.map_err(|e| anyhow!("Failed to deploy mock controller: {}", e))?;
        let controller_address = controller.address();
        
        let tx = node_registry.register_node(
            id_address, 
            Bytes::from(vec![1, 2, 3]), 
            true 
        );
        let pending_tx = tx.send().await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        pending_tx.await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?;

        let tx = node_registry.set_node_state(id_address, false);
        let pending_tx = tx.send().await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        pending_tx.await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?;
        
        let config = Config::default();
        let chain_identity = GeneralMainChainIdentity::new(
            chain_id,
            wallet.clone(),
            ws_provider.clone(),
            anvil.ws_endpoint(),
            controller_address,
            Address::random(),
            node_registry_address,
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber").await
        };
        
        let is_eigenlayer = true;
        let listener_descriptor = ListenerDescriptor {
            chain_id,
            l_type: ListenerType::ScheduleNodeActivation,
            interval_millis: 1000, 
            use_jitter: true,      
            reset_descriptor: FixedIntervalRetryDescriptor {
                interval_millis: 5000, 
                max_attempts: 3,       
                use_jitter: true,      
            },
        };
        
        let mut listener = NodeActivationListener::<G2Curve>::new(
            listener_descriptor,
            is_eigenlayer,
            chain_identity_arc.clone(),
            event_queue.clone(),
        );
        
        listener.initialize().await?;
        listener.listen().await?;
        
        let received_event = timeout(Duration::from_secs(5), event_receiver.recv()).await.map_err(|e| anyhow!("Failed to receive event: {}", e))?
            .ok_or_else(|| anyhow!("Event channel closed"))?;
            
        if let Some(event) = received_event.downcast_ref::<NodeActivation>() {
            assert_eq!(event.chain_id, chain_id);
            assert_eq!(event.is_eigenlayer, true, "is_eigenlayer flag should be true");
            assert_eq!(event.node_registry_address, node_registry_address);
            println!("Successfully received NodeActivation event with is_eigenlayer=true");
        } else {
            return Err(anyhow!("Received unexpected event type").into());
        }
        
        Ok(())
    }
}