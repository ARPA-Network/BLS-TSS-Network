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
        utils::{Anvil,AnvilInstance}
    };
    use std::time::Duration;
    use tokio::time::timeout;
    use anyhow::anyhow;
    use std::sync::Arc;

    struct TestEnvironment {
        _anvil: AnvilInstance,
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
        ws_provider: Arc<Provider<Ws>>,
        wallet: LocalWallet,
        id_address: Address,
        chain_id: usize,
        controller_address: Address,
        node_registry_address: Address,
    }

    impl TestEnvironment {
        async fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let anvil = Anvil::new().spawn();
            println!("Anvil instance started at {}", anvil.endpoint());
            
            let http_provider = Provider::<Http>::try_from(anvil.endpoint())?;
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
            
            let (controller_address, node_registry_address) = 
                deploy_contracts(client.clone()).await?;
            
            Ok(TestEnvironment {
                _anvil: anvil,
                client,
                ws_provider,
                wallet,
                id_address,
                chain_id,
                controller_address,
                node_registry_address,
            })
        }

        fn create_chain_identity(&self) -> Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> {
            let config = Config::default();
            let chain_identity = GeneralMainChainIdentity::new(
                self.chain_id,
                self.wallet.clone(),
                self.ws_provider.clone(),
                format!("ws://127.0.0.1:8545"), 
                self.controller_address,
                Address::random(),
                self.node_registry_address,
                config.get_time_limits().contract_transaction_retry_descriptor,
                config.get_time_limits().contract_view_retry_descriptor,
                None,
            );
            
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>))
        }

        fn create_listener_descriptor(&self) -> ListenerDescriptor {
            ListenerDescriptor {
                chain_id: self.chain_id,
                l_type: ListenerType::ScheduleNodeActivation,
                interval_millis: 1000,
                use_jitter: true,
                reset_descriptor: FixedIntervalRetryDescriptor {
                    interval_millis: 5000,
                    max_attempts: 3,
                    use_jitter: true,
                },
            }
        }

        async fn register_node(&self, is_eigenlayer: bool) -> Result<(), Box<dyn std::error::Error>> {
            let node_registry = MockNodeRegistry::new(self.node_registry_address, self.client.clone());
            
            let tx = node_registry.register_node(
                self.id_address,
                Bytes::from(vec![1, 2, 3]),
                is_eigenlayer,
            );
            
            let receipt = tx.send().await?.await?;
            println!("Node registered: {} in block {}", 
                self.id_address, receipt.unwrap().block_number.unwrap());
            
            Ok(())
        }

        async fn set_node_state(&self, is_active: bool) -> Result<(), Box<dyn std::error::Error>> {
            let node_registry = MockNodeRegistry::new(self.node_registry_address, self.client.clone());
            
            let tx = node_registry.set_node_state(self.id_address, is_active);
            let receipt = tx.send().await?.await?;
            
            if is_active {
                println!("Node state set to active in block {}", 
                    receipt.unwrap().block_number.unwrap());
            } else {
                println!("Node state set to inactive in block {}", 
                    receipt.unwrap().block_number.unwrap());
            }
            
            Ok(())
        }
    }

    async fn deploy_contracts(
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
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

        Ok((controller_address, node_registry_address))
    }
    
    async fn setup_event_subscriber(
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

    async fn wait_for_event(
        receiver: &mut tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>>,
        timeout_duration: Duration,
    ) -> NodeResult<NodeActivation> {
        let received_event = timeout(timeout_duration, receiver.recv()).await
            .map_err(|_| anyhow!("Timeout: No event received"))?
            .ok_or_else(|| anyhow!("Error: Event channel closed"))?;
        
        println!("Event received!");
        
        if let Some(event) = received_event.downcast_ref::<NodeActivation>() {
            println!("Received NodeActivation event");
            Ok(NodeActivation {
                chain_id: event.chain_id,
                is_eigenlayer: event.is_eigenlayer,
                node_registry_address: event.node_registry_address,
            })
        } else {
            Err(anyhow!("Received unexpected event type").into())
        }
    }

    async fn assert_no_event_received(
        receiver: &mut tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>>,
        timeout_duration: Duration,
    ) {
        let timeout_result = timeout(timeout_duration, receiver.recv()).await;
        assert!(timeout_result.is_err(), "Unexpectedly received an event when none was expected");
    }

    fn assert_node_activation_event(
        event: &NodeActivation,
        expected_chain_id: usize,
        expected_is_eigenlayer: bool,
        expected_node_registry_address: Address,
    ) {
        assert_eq!(event.chain_id, expected_chain_id);
        assert_eq!(event.is_eigenlayer, expected_is_eigenlayer);
        assert_eq!(event.node_registry_address, expected_node_registry_address);
    }
    
    #[tokio::test]
    async fn test_node_activation_listener() -> NodeResult<()> {
        println!("Starting test_node_activation_listener");
        
        let env = TestEnvironment::new().await
            .map_err(|e| anyhow!("Failed to setup test environment: {}", e))?;
        
        env.register_node(false).await
            .map_err(|e| anyhow!("Failed to register node: {}", e))?;
        env.set_node_state(false).await
            .map_err(|e| anyhow!("Failed to set node state: {}", e))?;
        
        let chain_identity_arc = env.create_chain_identity();
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            println!("Setting up test subscriber");
            setup_event_subscriber(&mut *eq_write, "test_subscriber").await
        };
        
        let is_eigenlayer = false;
        let listener_descriptor = env.create_listener_descriptor();
        
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
        assert_eq!(listener.node_registry_address.unwrap(), env.node_registry_address, "Node registry address does not match");
        
        println!("Testing direct publishing through listener");
        listener.publish(NodeActivation {
            chain_id: env.chain_id,
            is_eigenlayer,
            node_registry_address: env.node_registry_address,
        }).await;
        
        let event = wait_for_event(&mut event_receiver, Duration::from_secs(5)).await?;
        assert_node_activation_event(&event, env.chain_id, is_eigenlayer, env.node_registry_address);
        
        println!("Testing listen method - should generate event since node state is false");
        let listen_result = listener.listen().await;
        assert!(listen_result.is_ok(), "Listen method should not fail");
        
        let event = wait_for_event(&mut event_receiver, Duration::from_secs(5)).await?;
        assert_node_activation_event(&event, env.chain_id, is_eigenlayer, env.node_registry_address);
        
        println!("\nSetting node to active state...");
        env.set_node_state(true).await
            .map_err(|e| anyhow!("Failed to set node state: {}", e))?;
        
        let new_event_queue = Arc::new(RwLock::new(EventQueue::new()));
        let mut event_receiver = {
            let mut eq_write = new_event_queue.write().await;
            println!("Setting up new test subscriber");
            setup_event_subscriber(&mut *eq_write, "test_subscriber_2").await
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
        
        assert_no_event_received(&mut event_receiver, Duration::from_millis(500)).await;
        
        println!("Testing handle_interruption method");
        let interruption_result = listener.handle_interruption().await;
        assert!(interruption_result.is_ok(), "Handle interruption failed");
        
        println!("Testing chain_id method");
        assert_eq!(listener.chain_id(), env.chain_id);
        
        let display_string = format!("{}", listener);
        assert_eq!(display_string, "NodeActivationListener");
        
        println!("Test completed successfully");
        Ok(())
    }
    
    #[tokio::test]
    async fn test_node_activation_listener_with_eigenlayer() -> NodeResult<()> {
        println!("Starting test_node_activation_listener_with_eigenlayer");
        
        let env = TestEnvironment::new().await
            .map_err(|e| anyhow!("Failed to setup test environment: {}", e))?;
        
        env.register_node(true).await
            .map_err(|e| anyhow!("Failed to register node: {}", e))?;
        env.set_node_state(false).await
            .map_err(|e| anyhow!("Failed to set node state: {}", e))?;
        
        let chain_identity_arc = env.create_chain_identity();
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            setup_event_subscriber(&mut *eq_write, "test_subscriber").await
        };
        
        let is_eigenlayer = true;
        let listener_descriptor = env.create_listener_descriptor();
        
        let mut listener = NodeActivationListener::<G2Curve>::new(
            listener_descriptor,
            is_eigenlayer,
            chain_identity_arc.clone(),
            event_queue.clone(),
        );
        
        listener.initialize().await?;
        listener.listen().await?;
        
        let event = wait_for_event(&mut event_receiver, Duration::from_secs(5)).await?;
        assert_node_activation_event(&event, env.chain_id, true, env.node_registry_address);
        println!("Successfully received NodeActivation event with is_eigenlayer=true");
        
        Ok(())
    }
}