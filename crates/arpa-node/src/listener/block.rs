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
    use ethers::middleware::SignerMiddleware;
    use threshold_bls::schemes::bn254::G2Curve;
    use crate::queue::EventSubscriber;
    use crate::event::types::Topic;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use arpa_core::{
        Config, FixedIntervalRetryDescriptor, GeneralMainChainIdentity, ListenerType
    };
    use ethers::{
        providers::{Provider, Ws, Http, Middleware},
        types::{Address, TransactionRequest, U256},
        utils::{Anvil, AnvilInstance},
    };
    use std::time::Duration;
    use tokio::time::{timeout, sleep};
    use tokio::task::JoinHandle;
    use anyhow::anyhow;
    use std::sync::Arc;

    async fn mock_subscriber(
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
                if let Some(event) = payload.as_any().downcast_ref::<NewBlock>() {
                    let cloned_event = NewBlock {
                        chain_id: event.chain_id,
                        block_height: event.block_height,
                    };
                    let boxed = Box::new(cloned_event) as Box<dyn std::any::Any + Send>;
                    self.sender.send(boxed).await.map_err(|e| {
                        let err: crate::error::NodeError = anyhow!("Failed to send event: {}", e).into();
                        err
                    })?;
                }
                Ok(())
            }
            
            async fn subscribe(self) {}
        }
        
        impl DebuggableSubscriber for TestSubscriber {}
        
        let subscriber = TestSubscriber {
            name: subscriber_name.to_string(),
            sender,
        };

        let topic = Topic::NewBlock(chain_id);
        eq.subscribe(topic, Box::new(subscriber));
        
        receiver
    }
    
    async fn run_listener_in_task<PC: Curve + Sync + Send + 'static>(
        listener: BlockListener<PC>,
    ) -> JoinHandle<NodeResult<()>> {
        tokio::spawn(async move {
            listener.listen().await
        })
    }
    
    async fn setup_test_environment() -> (
        usize,                                            
        Arc<RwLock<ChainIdentityHandlerType<G2Curve>>>,   
        Arc<RwLock<EventQueue>>,                          
        ListenerDescriptor,                               
        Arc<Provider<Ws>>,                                
        Arc<SignerMiddleware<Provider<Http>, LocalWallet>>, 
        AnvilInstance,                                      
    ) {
        let anvil = Anvil::new().spawn();
        
        let ws_provider = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await.unwrap());
        let http_provider = Provider::<Http>::try_from(anvil.endpoint()).unwrap();
        
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        
        let chain_id = anvil.chain_id() as usize;
        
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
            None,
        );
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        
        let listener_descriptor = ListenerDescriptor {
            chain_id,
            l_type: ListenerType::Block,
            interval_millis: 1000, 
            use_jitter: true,      
            reset_descriptor: FixedIntervalRetryDescriptor {
                interval_millis: 5000, 
                max_attempts: 3,       
                use_jitter: true,      
            },
        };

        let signer = wallet.with_chain_id(anvil.chain_id());
        let client = Arc::new(SignerMiddleware::new(http_provider, signer));
        
        (
            chain_id,
            chain_identity_arc,
            event_queue, 
            listener_descriptor,
            ws_provider,
            client,
            anvil, 
        )
    }

    #[tokio::test]
    async fn test_event_publishing() -> NodeResult<()> {
        let (
            chain_id,
            chain_identity_arc,
            event_queue,
            listener_descriptor,
            _ws_provider,
            _client,
            _anvil,
        ) = setup_test_environment().await;
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            mock_subscriber(&mut *eq_write, "test_subscriber", chain_id).await
        };
        
        let block_listener = BlockListener::<G2Curve>::new(
            listener_descriptor,
            chain_identity_arc,
            event_queue,
        );
        
        let test_block_height = 12345;
        block_listener.publish(NewBlock {
            chain_id,
            block_height: test_block_height,
        }).await;
        
        let received_event = timeout(Duration::from_secs(5), event_receiver.recv()).await
            .map_err(|_| anyhow!("Timeout: No event received"))?
            .ok_or_else(|| anyhow!("Error: Event channel closed"))?;
        
        if let Some(event) = received_event.downcast_ref::<NewBlock>() {
            assert_eq!(event.chain_id, chain_id);
            assert_eq!(event.block_height, test_block_height);
            Ok(())
        } else {
            Err(anyhow!("Received unexpected event type").into())
        }
    }

    #[tokio::test]
    async fn test_block_listener_with_chain_events() -> NodeResult<()> {
        let (
            chain_id,
            chain_identity_arc,
            event_queue,
            listener_descriptor,
            _ws_provider,
            client,
            _anvil,
        ) = setup_test_environment().await;
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            mock_subscriber(&mut *eq_write, "test_subscriber", chain_id).await
        };
        
        let block_listener = BlockListener::<G2Curve>::new(
            listener_descriptor,
            chain_identity_arc,
            event_queue,
        );
        
        let listener_task = run_listener_in_task(block_listener).await;
                
        sleep(Duration::from_millis(500)).await;
        
        let tx = TransactionRequest::new()
            .to(Address::random())
            .value(U256::from(1));
        
        client.send_transaction(tx, None).await.unwrap().await.unwrap();
        
        let chain_event_result = timeout(Duration::from_secs(5), event_receiver.recv()).await;
        
        listener_task.abort();
        
        match chain_event_result {
            Ok(Some(received_chain_event)) => {
                if let Some(event) = received_chain_event.downcast_ref::<NewBlock>() {
                    assert_eq!(event.chain_id, chain_id);
                    assert!(event.block_height > 0, "Block height should be positive");
                    Ok(())
                } else {
                    Err(anyhow!("Received unexpected event type from chain").into())
                }
            },
            Ok(None) => Err(anyhow!("Event channel closed unexpectedly").into()),
            Err(_) => Err(anyhow!("Timeout waiting for chain event").into()),
        }
    }

    #[tokio::test]
    async fn test_handle_interruption() -> NodeResult<()> {
        let (
            _chain_id,
            chain_identity_arc,
            event_queue,
            listener_descriptor,
            _ws_provider,
            _client,
            _anvil,
        ) = setup_test_environment().await;
        
        let block_listener = BlockListener::<G2Curve>::new(
            listener_descriptor,
            chain_identity_arc,
            event_queue,
        );
        
        let result = block_listener.handle_interruption().await;
        assert!(result.is_ok(), "Handle interruption failed");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_accessor_methods() -> NodeResult<()> {
        let (
            chain_id,
            chain_identity_arc,
            event_queue,
            listener_descriptor,
            _ws_provider,
            _client,
            _anvil,
        ) = setup_test_environment().await;
        
        let block_listener = BlockListener::<G2Curve>::new(
            listener_descriptor.clone(),
            chain_identity_arc,
            event_queue,
        );
        
        assert_eq!(block_listener.chain_id(), chain_id);
        
        let returned_descriptor = block_listener.listener_descriptor();
        assert_eq!(returned_descriptor.chain_id, listener_descriptor.chain_id);
        assert_eq!(returned_descriptor.l_type, listener_descriptor.l_type);
        assert_eq!(returned_descriptor.interval_millis, listener_descriptor.interval_millis);
        assert_eq!(returned_descriptor.use_jitter, listener_descriptor.use_jitter);
        
        let display_string = format!("{}", block_listener);
        assert_eq!(display_string, "BlockListener");
        
        Ok(())
    }
}