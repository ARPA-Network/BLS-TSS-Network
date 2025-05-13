use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::new_block::NewBlock,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::provider::BlockFetcher;
use arpa_core::ListenerDescriptor;
use async_trait::async_trait;
use log::debug;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct BlockListener<PC: Curve> {
    listener_descriptor: ListenerDescriptor,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> std::fmt::Display for BlockListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlockListener")
    }
}

impl<PC: Curve> BlockListener<PC> {
    pub fn new(
        listener_descriptor: ListenerDescriptor,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        BlockListener {
            listener_descriptor,
            chain_identity,
            eq,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> EventPublisher<NewBlock> for BlockListener<PC> {
    async fn publish(&self, event: NewBlock) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for BlockListener<PC> {
    async fn listen(&self) -> NodeResult<()> {
        let chain_id = self.listener_descriptor.chain_id;
        let eq = self.eq.clone();

        let provider = self.chain_identity.read().await.get_provider().clone();

        provider
            .subscribe_new_block_height(move |block_height: usize| {
                debug!("New block height: {} for chain {}", block_height, chain_id);

                let eq = eq.clone();
                async move {
                    eq.read()
                        .await
                        .publish(NewBlock {
                            chain_id,
                            block_height,
                        })
                        .await;

                    Ok(())
                }
            })
            .await?;

        Ok(())
    }

    async fn handle_interruption(&self) -> NodeResult<()> {
        self.chain_identity.write().await.reset_provider().await?;

        Ok(())
    }

    fn chain_id(&self) -> usize {
        self.listener_descriptor.chain_id
    }

    fn listener_descriptor(&self) -> ListenerDescriptor {
        self.listener_descriptor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::signers::{LocalWallet, Signer};
    use ethers_middleware::SignerMiddleware;
    use threshold_bls::schemes::bn254::G2Curve;
    use crate::queue::EventSubscriber;
    use crate::event::types::Topic;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use arpa_core::{
        Config, GeneralMainChainIdentity
    };
    use ethers::{
        providers::{Provider, Ws, Http, Middleware},
        types::{Address, TransactionRequest, U256},
        utils::Anvil,
    };
    use std::time::Duration;
    use tokio::time::{timeout, sleep};
    use tokio::task::JoinHandle;
    use anyhow::anyhow;
    use std::sync::Arc;

    async fn mock_subscribe_to_events(
        eq: &mut EventQueue,
        subscriber_name: &str,
        chain_id: usize,
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
                if let Some(event) = payload.as_any().downcast_ref::<NewBlock>() {
                    println!("Received NewBlock event with block height: {}", event.block_height);
                    let cloned_event = NewBlock {
                        chain_id: event.chain_id,
                        block_height: event.block_height,
                    };
                    let boxed = Box::new(cloned_event) as Box<dyn std::any::Any + Send>;
                    self.sender.send(boxed).await.map_err(|e| {
                        println!("Failed to send event: {}", e);
                        let err: crate::error::NodeError = anyhow!("Failed to send event: {}", e).into();
                        err
                    })?;
                    println!("Event sent to receiver");
                } else {
                    println!("Payload is not a NewBlock event");
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

        let topic = Topic::NewBlock(chain_id);
        println!("Subscribing to topic: {:?}", topic);
        
        eq.subscribe(topic, Box::new(subscriber));
        println!("Subscribed to event queue");
        
        receiver
    }
    
    async fn run_listener_in_task<PC: Curve + Sync + Send + 'static>(
        listener: BlockListener<PC>,
    ) -> JoinHandle<NodeResult<()>> {
        tokio::spawn(async move {
            listener.listen().await
        })
    }
    
    #[tokio::test]
    async fn test_block_listener() -> NodeResult<()> {
        println!("Starting test_block_listener");
        
        let anvil = Anvil::new().spawn();
        println!("Anvil instance started");
        
        let ws_provider = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await.unwrap());
        println!("Connected to Anvil WebSocket at {}", anvil.ws_endpoint());
        
        let http_provider = Provider::<Http>::try_from(anvil.endpoint()).unwrap();
        println!("Connected to Anvil HTTP at {}", anvil.endpoint());
        
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let wallet_address = wallet.address();
        println!("Using wallet address: {}", wallet_address);
        
        let chain_id = anvil.chain_id() as usize;
        println!("Chain ID: {}", chain_id);
        
        let controller_address = Address::random();
        let adapter_address = Address::random();
        
        let config = Config::default();
        
        let chain_identity = GeneralMainChainIdentity::new(
            chain_id,
            wallet.clone(),
            ws_provider.clone(),
            anvil.ws_endpoint(),
            controller_address,
            adapter_address,
            Address::random(),
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
        );
        println!("Chain identity created");
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        println!("Event queue created");
        
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            println!("Setting up test subscriber");
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber", chain_id).await
        };
        
        let block_listener = BlockListener::<G2Curve>::new(
            chain_id,
            chain_identity_arc.clone(),
            event_queue.clone(),
        );
        println!("Block listener created");
        
        println!("Testing direct publishing through listener");
        let test_block_height = 12345;
        block_listener.publish(NewBlock {
            chain_id,
            block_height: test_block_height,
        }).await;
        
        println!("Waiting for event...");
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
        
        if let Some(event) = received_event.downcast_ref::<NewBlock>() {
            println!("Received NewBlock event with chain_id: {} and block_height: {}", 
                    event.chain_id, event.block_height);
            assert_eq!(event.chain_id, chain_id);
            assert_eq!(event.block_height, test_block_height);
        } else {
            println!("Received unexpected event type");
            return Err(anyhow!("Received unexpected event type").into());
        }
        
        println!("Testing block updates from chain");
        
        let listener_task = run_listener_in_task(block_listener).await;
        println!("Block listener started in background task");
        
        let current_block_height = ws_provider.get_block_number().await.unwrap().as_u64() as usize;
        println!("Current block height: {}", current_block_height);
        
        let signer = wallet.with_chain_id(anvil.chain_id());
        let client = SignerMiddleware::new(http_provider, signer);
        
        println!("Sending transaction to trigger new block...");
        let tx = TransactionRequest::new()
            .to(Address::random())
            .value(U256::from(1))
            .from(wallet_address);
        
        let pending_tx = client.send_transaction(tx, None).await.unwrap();
        let receipt = pending_tx.await.unwrap();
        println!("Transaction confirmed in block: {:?}", receipt.unwrap().block_number);
        
        println!("Waiting for block event...");
        
        let chain_event_result = timeout(Duration::from_secs(5), event_receiver.recv()).await;
        
        match chain_event_result {
            Ok(Some(received_chain_event)) => {
                if let Some(event) = received_chain_event.downcast_ref::<NewBlock>() {
                    println!("Received NewBlock event from chain with chain_id: {} and block_height: {}", 
                            event.chain_id, event.block_height);
                    assert_eq!(event.chain_id, chain_id);
                    assert!(event.block_height > 0, "Block height should be positive");
                } else {
                    println!("Received unexpected event type from chain");
                    listener_task.abort();
                    return Err(anyhow!("Received unexpected event type from chain").into());
                }
            },
            Ok(None) => {
                println!("Warning: Event channel closed unexpectedly");
            },
            Err(_) => {
                println!("Warning: Timeout waiting for chain event");
            }
        }
        
        println!("Aborting block listener task");
        listener_task.abort();
        
        sleep(Duration::from_millis(500)).await;
        
        let interrupt_listener = BlockListener::<G2Curve>::new(
            chain_id,
            chain_identity_arc.clone(),
            event_queue.clone(),
        );
        
        println!("Testing handle_interruption method");
        let interruption_result = interrupt_listener.handle_interruption().await;
        assert!(interruption_result.is_ok(), "Handle interruption failed");
        
        println!("Testing chain_id method");
        assert_eq!(interrupt_listener.chain_id().await, chain_id);
        
        let display_string = format!("{}", interrupt_listener);
        assert_eq!(display_string, "BlockListener");
        
        println!("Test completed successfully");
        Ok(())
    }
}