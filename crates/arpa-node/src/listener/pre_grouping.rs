use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::new_dkg_task::NewDKGTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::controller::ControllerLogs;
use arpa_core::{
    log::{build_task_related_payload, LogType},
    ListenerDescriptor, TaskType,
};
use arpa_dal::GroupInfoHandler;
use async_trait::async_trait;
use ethers::providers::Middleware;
use log::info;
use serde_json::json;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PreGroupingListener<PC: Curve> {
    listener_descriptor: ListenerDescriptor,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> std::fmt::Display for PreGroupingListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PreGroupingListener")
    }
}

impl<PC: Curve> PreGroupingListener<PC> {
    pub fn new(
        listener_descriptor: ListenerDescriptor,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        PreGroupingListener {
            listener_descriptor,
            chain_identity,
            group_cache,
            eq,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> EventPublisher<NewDKGTask> for PreGroupingListener<PC> {
    async fn publish(&self, event: NewDKGTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for PreGroupingListener<PC> {
    async fn listen(&self) -> NodeResult<()> {
        let client = self.chain_identity.read().await.build_controller_client();
        let self_id_address = self.chain_identity.read().await.get_id_address();

        client
            .subscribe_dkg_task(move |dkg_task| {
                let group_cache = self.group_cache.clone();
                let eq = self.eq.clone();

                async move {
                    let chain_id = self.listener_descriptor.chain_id;

                    if let Some((node_index, _)) = dkg_task
                        .members
                        .iter()
                        .enumerate()
                        .find(|(_, id_address)| **id_address == self_id_address)
                    {
                        let cache_index = group_cache.read().await.get_index().unwrap_or(0);

                        let cache_epoch = group_cache.read().await.get_epoch().unwrap_or(0);

                        if cache_index != dkg_task.group_index || cache_epoch != dkg_task.epoch {
                            info!(
                                "{}",
                                build_task_related_payload(
                                    LogType::TaskReceived,
                                    "DKG grouping task received.",
                                    chain_id,
                                    &[],
                                    TaskType::DKG,
                                    json!(dkg_task),
                                    None
                                )
                            );

                            let self_index = node_index;

                            eq.read()
                                .await
                                .publish(NewDKGTask {
                                    chain_id,
                                    dkg_task,
                                    self_index,
                                })
                                .await;
                        }
                    }
                    Ok(())
                }
            })
            .await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_contracts::mockcontroller:: deploy_with_args_and_get_mock_controller;
    use ethers::signers::{LocalWallet, Signer};
    use ethers::middleware::SignerMiddleware;
    use ethers::types::{U256, Address};
    use threshold_bls::schemes::bn254::G2Curve;
    use crate::queue::EventSubscriber;
    use crate::event::types::Topic;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use arpa_core::{
        Config, DKGTask, FixedIntervalRetryDescriptor, GeneralMainChainIdentity, ListenerType
    };
    use arpa_dal::{
        cache::InMemoryGroupInfoCache,
        GroupInfoHandler
    };
    use ethers::{
        providers::{Provider, Ws, Http},
        utils::Anvil,
    };
    use std::time::Duration;
    use tokio::time::timeout;
    use anyhow::anyhow;
    use std::sync::Arc;

    fn create_listener_descriptor(chain_id: usize) -> ListenerDescriptor {
        ListenerDescriptor {
            chain_id,
            l_type: ListenerType::PreGrouping,
            interval_millis: 1000,
            use_jitter: true,
            reset_descriptor: FixedIntervalRetryDescriptor {
                interval_millis: 5000,
                max_attempts: 3,
                use_jitter: true,
            },
        }
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
                if let Some(dkg_task_event) = payload.as_any().downcast_ref::<NewDKGTask>() {
                    println!("Received NewDKGTask event");
                    let cloned_event = NewDKGTask {
                        chain_id: dkg_task_event.chain_id,
                        dkg_task: dkg_task_event.dkg_task.clone(),
                        self_index: dkg_task_event.self_index,
                    };
                    let boxed = Box::new(cloned_event) as Box<dyn std::any::Any + Send>;
                    self.sender.send(boxed).await.map_err(|e| {
                        println!("Failed to send event: {}", e);
                        let err: crate::error::NodeError = anyhow!("Failed to send event: {}", e).into();
                        err
                    })?;
                    println!("Event sent to receiver");
                } else {
                    println!("Payload is not a NewDKGTask event");
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

        let topic = Topic::NewDKGTask;
        println!("Subscribing to topic: {:?}", topic);
        
        eq.subscribe(topic, Box::new(subscriber));
        println!("Subscribed to event queue");
        
        receiver
    }
    
    #[tokio::test]
    async fn test_pre_grouping_listener() -> NodeResult<()> {
        let anvil = Anvil::new().spawn();
        let provider = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await.unwrap());
        let config = Config::default();
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let id_address = wallet.address();
        let chain_id = anvil.chain_id() as usize;
        let controller_address = Address::random();
        let adapter_address = Address::random();
        let node_registry_address = Address::random();
        
        let chain_identity = GeneralMainChainIdentity::new(
            chain_id,
            wallet,
            provider.clone(),
            anvil.ws_endpoint(),
            controller_address,
            adapter_address,
            node_registry_address,
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );
        
        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::new(id_address)),
        ));
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        let listener_descriptor = create_listener_descriptor(chain_id);
        let listener = PreGroupingListener::<G2Curve>::new(
            listener_descriptor.clone(),
            chain_identity_arc.clone(),
            group_cache.clone(),
            event_queue.clone(),
        );
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber").await
        };
        
        let dkg_task = DKGTask {
            group_index: 1,
            epoch: 1,
            size: 3,
            threshold: 2,
            assignment_block_height: 100,
            members: vec![id_address, Address::random(), Address::random()],
            coordinator_address: Address::random()
        };
        
        listener.publish(NewDKGTask {
            chain_id,
            dkg_task: dkg_task.clone(),
            self_index: 0,
        }).await;
        
        let received_event = timeout(Duration::from_secs(1), event_receiver.recv()).await
            .map_err(|_| anyhow!("Timeout: No event received"))?
            .ok_or_else(|| anyhow!("Error: Event channel closed"))?;
        
        if let Some(task_event) = received_event.downcast_ref::<NewDKGTask>() {
            assert_eq!(task_event.chain_id, chain_id);
            assert_eq!(task_event.dkg_task.group_index, 1);
            assert_eq!(task_event.dkg_task.epoch, 1);
            assert_eq!(task_event.dkg_task.members[0], id_address);
            assert_eq!(task_event.self_index, 0);
        } else {
            return Err(anyhow!("Received unexpected event type").into());
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_pre_grouping_listener_listen() -> NodeResult<()> {
        println!("Starting test_pre_grouping_listener_listen");
        
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
        let node_registry_address = Address::random();
        let controller = deploy_with_args_and_get_mock_controller(client.clone(), node_registry_address)
            .await
            .map_err(|e| anyhow!("Failed to deploy mock controller contract: {}", e))?;
        
        let controller_address = controller.address();
        println!("Controller contract deployed at: {}", controller_address);
        
        let adapter_address = Address::random();
        let config = Config::default();
        
        let chain_identity = GeneralMainChainIdentity::new(
            chain_id,
            wallet.clone(),
            ws_provider.clone(),
            anvil.ws_endpoint(),
            controller_address,
            adapter_address,
            node_registry_address,
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );
        println!("Chain identity created");
        
        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::new(id_address)),
        ));
        println!("Group cache created");
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        println!("Event queue created");
        
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            println!("Setting up test subscriber");
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber").await
        };
        let listener_descriptor = create_listener_descriptor(chain_id);
        let listener = PreGroupingListener::<G2Curve>::new(
            listener_descriptor.clone(),
            chain_identity_arc.clone(),
            group_cache.clone(),
            event_queue.clone(),
        );
        println!("Listener created");
        
        println!("Starting listener...");
        tokio::spawn(async move {
            if let Err(e) = listener.listen().await {
                println!("Listener error: {:?}", e);
            }
        });
        
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        println!("Emitting DkgTask event from contract...");
        
        let members = vec![id_address, Address::random(), Address::random()];
        let global_epoch = U256::from(1);
        let group_index = U256::from(2);
        let group_epoch = U256::from(1);
        let size = U256::from(3);
        let threshold = U256::from(2);
        let assignment_block_height = U256::from(100);
        let coordinator_address = Address::random();
        

        {
            let tx = controller.emit_dkg_task_event(
                global_epoch,
                group_index,
                group_epoch,
                size,
                threshold,
                members.clone(),
                assignment_block_height,
                coordinator_address
            );
            
            let pending_tx = tx.send().await
                .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;
                
            pending_tx.await
                .map_err(|e| anyhow!("Transaction failed: {}", e))?;
        }
        
        println!("DkgTask event emitted, waiting for listener to process...");
        
        println!("Waiting for event to be received...");
        let received_event = timeout(Duration::from_secs(5), event_receiver.recv()).await
            .map_err(|_| anyhow!("Timeout: No event received after emitting DkgTask"))?
            .ok_or_else(|| anyhow!("Error: Event channel closed"))?;
        
        println!("Event received!");
        
        if let Some(task_event) = received_event.downcast_ref::<NewDKGTask>() {
            println!("Received NewDKGTask: {:?}", task_event);
            assert_eq!(task_event.chain_id, chain_id);
            assert_eq!(task_event.dkg_task.group_index, group_index.as_usize());
            assert_eq!(task_event.dkg_task.epoch, group_epoch.as_usize());
            assert_eq!(task_event.dkg_task.members, members);
            assert_eq!(task_event.dkg_task.coordinator_address, coordinator_address);
            assert_eq!(task_event.self_index, 0);
            println!("Event validation passed!");
        } else {
            println!("Received unexpected event type");
            return Err(anyhow!("Received unexpected event type").into());
        }
        
        println!("Test completed successfully");
        Ok(())
    }
    
    #[tokio::test]
    async fn test_pre_grouping_listener_listen_not_member() -> NodeResult<()> {
        println!("Starting test_pre_grouping_listener_listen_not_member");
        
        let anvil = Anvil::new().spawn();
        let http_provider = Provider::<Http>::try_from(anvil.endpoint())
            .map_err(|e| anyhow!("Failed to create HTTP provider: {}", e))?;
        
        let ws_provider = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await?);
        
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let id_address = wallet.address();
        
        let chain_id = anvil.chain_id() as usize;
        
        let client = Arc::new(SignerMiddleware::new(
            http_provider,
            wallet.clone().with_chain_id(anvil.chain_id()),
        ));
        let node_registry_address = Address::random();
        let controller = deploy_with_args_and_get_mock_controller(client.clone(), node_registry_address)
            .await
            .map_err(|e| anyhow!("Failed to deploy mock controller contract: {}", e))?;
            
        let controller_address = controller.address();        
        let adapter_address = Address::random();       
        
        let config = Config::default();
        
        let chain_identity = GeneralMainChainIdentity::new(
            chain_id,
            wallet.clone(),
            ws_provider.clone(),
            anvil.ws_endpoint(),
            controller_address,
            adapter_address,
            node_registry_address,
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );
        
        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::new(id_address)),
        ));
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber").await
        };
        let listener_descriptor = create_listener_descriptor(chain_id);
        let listener = PreGroupingListener::<G2Curve>::new(
            listener_descriptor.clone(),
            chain_identity_arc.clone(),
            group_cache.clone(),
            event_queue.clone(),
        );
        
        tokio::spawn(async move {
            if let Err(e) = listener.listen().await {
                println!("Listener error: {:?}", e);
            }
        });
        
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        let members = vec![Address::random(), Address::random(), Address::random()];
        let global_epoch = U256::from(1);
        let group_index = U256::from(2);
        let group_epoch = U256::from(1);
        let size = U256::from(3);
        let threshold = U256::from(2);
        let assignment_block_height = U256::from(100);
        let coordinator_address = Address::random();
        
        {
            let tx = controller.emit_dkg_task_event(
                global_epoch,
                group_index,
                group_epoch,
                size,
                threshold,
                members.clone(),
                assignment_block_height,
                coordinator_address
            );
            
            let pending_tx = tx.send().await
                .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;
                
            pending_tx.await
                .map_err(|e| anyhow!("Transaction failed: {}", e))?;
        }
        
        let timeout_result = timeout(Duration::from_millis(500), event_receiver.recv()).await;
        assert!(timeout_result.is_err(), "Unexpectedly received an event when not a member");
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_pre_grouping_listener_listen_same_task() -> NodeResult<()> {
        println!("Starting test_pre_grouping_listener_listen_same_task");
        
        let anvil = Anvil::new().spawn();
        let http_provider = Provider::<Http>::try_from(anvil.endpoint())
            .map_err(|e| anyhow!("Failed to create HTTP provider: {}", e))?;
        
        let ws_provider = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await?);
        
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let id_address = wallet.address();
        
        let chain_id = anvil.chain_id() as usize;
        
        let client = Arc::new(SignerMiddleware::new(
            http_provider,
            wallet.clone().with_chain_id(anvil.chain_id()),
        ));
        let node_registry_address = Address::random();
        let controller = deploy_with_args_and_get_mock_controller(client.clone(), node_registry_address)
            .await
            .map_err(|e| anyhow!("Failed to deploy mock controller contract: {}", e))?;
            
        let controller_address = controller.address();
        
        let adapter_address = Address::random();
        let config = Config::default();
        
        let chain_identity = GeneralMainChainIdentity::new(
            chain_id,
            wallet.clone(),
            ws_provider.clone(),
            anvil.ws_endpoint(),
            controller_address,
            adapter_address,
            node_registry_address,
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );
        
        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::new(id_address)),
        ));
        
        {
            let mut group_cache_write = group_cache.write().await;
            let dkg_task = DKGTask {
                group_index: 2,
                epoch: 1,
                size: 3,
                threshold: 2,
                assignment_block_height: 50,
                members: vec![id_address, Address::random(), Address::random()],
                coordinator_address: Address::random()
            };
            
            group_cache_write.save_task_info(0, dkg_task).await?;
            
            assert_eq!(group_cache_write.get_index().unwrap(), 2);
            assert_eq!(group_cache_write.get_epoch().unwrap(), 1);
        }
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber").await
        };
        let listener_descriptor = create_listener_descriptor(chain_id);
        let listener = PreGroupingListener::<G2Curve>::new(
            listener_descriptor.clone(),
            chain_identity_arc.clone(),
            group_cache.clone(),
            event_queue.clone(),
        );
        
        tokio::spawn(async move {
            if let Err(e) = listener.listen().await {
                println!("Listener error: {:?}", e);
            }
        });
        
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        let members = vec![id_address, Address::random(), Address::random()];
        let global_epoch = U256::from(1);
        let group_index = U256::from(2);
        let group_epoch = U256::from(1);
        let size = U256::from(3);
        let threshold = U256::from(2);
        let assignment_block_height = U256::from(100);
        let coordinator_address = Address::random();
        
        {
            let tx = controller.emit_dkg_task_event(
                global_epoch,
                group_index,
                group_epoch,
                size,
                threshold,
                members.clone(),
                assignment_block_height,
                coordinator_address
            );
            
            let pending_tx = tx.send().await
                .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;
                
            pending_tx.await
                .map_err(|e| anyhow!("Transaction failed: {}", e))?;
        }
        
        let timeout_result = timeout(Duration::from_millis(500), event_receiver.recv()).await;
        assert!(timeout_result.is_err(), "Unexpectedly received an event for same task");
        
        Ok(())
    }
}