use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::ready_to_handle_randomness_task::ReadyToHandleRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::adapter::AdapterViews;
use arpa_core::RandomnessTask;
use arpa_dal::{BLSTasksHandler, BlockInfoHandler, GroupInfoHandler};
use async_trait::async_trait;
use ethers::{providers::Middleware, types::Address};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ReadyToHandleRandomnessTaskListener<PC: Curve> {
    chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
    randomness_task_exclusive_window: usize,
}

impl<PC: Curve> std::fmt::Display for ReadyToHandleRandomnessTaskListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReadyToHandleRandomnessTaskListener")
    }
}

impl<PC: Curve> ReadyToHandleRandomnessTaskListener<PC> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
        eq: Arc<RwLock<EventQueue>>,
        randomness_task_exclusive_window: usize,
    ) -> Self {
        ReadyToHandleRandomnessTaskListener {
            chain_id,
            id_address,
            chain_identity,
            block_cache,
            group_cache,
            randomness_tasks_cache,
            eq,
            pc: PhantomData,
            randomness_task_exclusive_window,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> EventPublisher<ReadyToHandleRandomnessTask>
    for ReadyToHandleRandomnessTaskListener<PC>
{
    async fn publish(&self, event: ReadyToHandleRandomnessTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for ReadyToHandleRandomnessTaskListener<PC> {
    async fn listen(&self) -> NodeResult<()> {
        let is_bls_ready = self.group_cache.read().await.get_state();

        if let Ok(true) = is_bls_ready {
            let current_group_index = self.group_cache.read().await.get_index()?;

            let current_block_height = self.block_cache.read().await.get_block_height();

            let available_tasks = self
                .randomness_tasks_cache
                .write()
                .await
                .check_and_get_available_tasks(
                    current_block_height,
                    current_group_index,
                    self.randomness_task_exclusive_window,
                )
                .await?;

            if available_tasks.is_empty() {
                return Ok(());
            }

            let mut tasks_to_process: Vec<RandomnessTask> = vec![];

            let client = self
                .chain_identity
                .read()
                .await
                .build_adapter_client(self.id_address);

            for task in available_tasks {
                if let Ok(true) = client.is_task_pending(&task.request_id).await {
                    tasks_to_process.push(task);
                }
            }

            if !tasks_to_process.is_empty() {
                self.publish(ReadyToHandleRandomnessTask {
                    chain_id: self.chain_id,
                    tasks: tasks_to_process,
                })
                .await;
            }
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

    async fn chain_id(&self) -> usize {
        self.chain_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::signers::{LocalWallet, Signer};
    use ethers_middleware::SignerMiddleware;
    use ethers::contract::{ContractFactory, abigen};
    use threshold_bls::schemes::bn254::G2Curve;
    use crate::queue::EventSubscriber;
    use crate::event::types::Topic;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use arpa_core::{
        Config, GeneralMainChainIdentity, RandomnessRequestType
    };
    use arpa_dal::cache::{InMemoryBLSTasksQueue, InMemoryBlockInfoCache, InMemoryGroupInfoCache};
    use ethers::{
        providers::{Provider, Ws, Http},
        types::{Address, U256, Bytes},
        utils::Anvil,
    };
    use std::time::Duration;
    use tokio::time::timeout;
    use anyhow::anyhow;
    use std::sync::Arc;

    abigen!(
        MockAdapter,
        r#"[
            function getPendingRequestCommitment(bytes32 requestId) external view returns (bytes32)
            function setRequestCommitment(bytes32 requestId, bytes32 commitment) external
        ]"#,
    );

    async fn deploy_mock_adapter(
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
        pending_request_ids: Vec<[u8; 32]>,
    ) -> Result<Address, Box<dyn std::error::Error>> {
        println!("Deploying mock adapter contract...");
    
        const ADAPTER_ABI: &str = r#"[{"inputs":[{"internalType":"bytes32","name":"","type":"bytes32"}],"name":"_requestCommitments","outputs":[{"internalType":"bytes32","name":"","type":"bytes32"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"bytes32","name":"requestId","type":"bytes32"}],"name":"getPendingRequestCommitment","outputs":[{"internalType":"bytes32","name":"","type":"bytes32"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"bytes32","name":"requestId","type":"bytes32"},{"internalType":"bytes32","name":"commitment","type":"bytes32"}],"name":"setRequestCommitment","outputs":[],"stateMutability":"nonpayable","type":"function"}]"#;
        const ADAPTER_BYTECODE: &str = "6080604052348015600e575f5ffd5b506101118061001c5f395ff3fe6080604052348015600e575f5ffd5b5060043610603a575f3560e01c80631565034c14603e5780635619490314606c57806368c50d52146088575b5f5ffd5b605a604936600460a6565b5f9081526020819052604090205490565b60405190815260200160405180910390f35b605a607736600460a6565b5f6020819052908152604090205481565b60a4609336600460bc565b5f9182526020829052604090912055565b005b5f6020828403121560b5575f5ffd5b5035919050565b5f5f6040838503121560cc575f5ffd5b5050803592602090910135915056fea2646970667358221220209b5a4c78813894f8864999fc9ec2b21ee36dc388a76beb182591847f1c6fd664736f6c634300081b0033";
    
        let adapter_factory = ContractFactory::new(
            serde_json::from_str(ADAPTER_ABI).expect("Invalid ADAPTER_ABI"),
            ADAPTER_BYTECODE.parse::<Bytes>().expect("Invalid ADAPTER_BYTECODE"),
            client.clone(),
        );
        
        let adapter_contract_deployed = adapter_factory.deploy(())?.send().await?;
        let adapter_address = adapter_contract_deployed.address();
        println!("Adapter contract deployed at: {}", adapter_address);
        
        abigen!(
            MockAdapter,
            r#"[
                function getPendingRequestCommitment(bytes32 requestId) external view returns (bytes32)
                function setRequestCommitment(bytes32 requestId, bytes32 commitment) external
            ]"#,
        );
        
        let adapter = MockAdapter::new(adapter_address, client.clone());
        
        println!("Setting up pending requests in adapter contract...");
        for &request_id in pending_request_ids.iter() {
            let request_commitment = [1u8; 32]; 
            adapter.set_request_commitment(request_id.into(), request_commitment.into())
                .send().await?;
            println!("  Set request ID {:?} as pending", request_id);
        }
        let test_request_id = [5u8; 32];
        let test_commitment = [2u8; 32];
        adapter.set_request_commitment(test_request_id.into(), test_commitment.into())
            .send().await?;
        println!("Set test request ID for verification");

        let result = adapter.get_pending_request_commitment(test_request_id.into()).call().await?;
        println!("Verification call result: {:?}", result);

        Ok(adapter_address)
    }
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
                if let Some(event) = payload.as_any().downcast_ref::<ReadyToHandleRandomnessTask>() {
                    println!("Received ReadyToHandleRandomnessTask event with {} tasks", event.tasks.len());
                    let cloned_event = ReadyToHandleRandomnessTask {
                        chain_id: event.chain_id,
                        tasks: event.tasks.clone(),
                    };
                    let boxed = Box::new(cloned_event) as Box<dyn std::any::Any + Send>;
                    self.sender.send(boxed).await.map_err(|e| {
                        println!("Failed to send event: {}", e);
                        let err: crate::error::NodeError = anyhow!("Failed to send event: {}", e).into();
                        err
                    })?;
                    println!("Event sent to receiver");
                } else {
                    println!("Payload is not a ReadyToHandleRandomnessTask event");
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

        let topic = Topic::ReadyToHandleRandomnessTask(chain_id);
        println!("Subscribing to topic: {:?}", topic);
        
        eq.subscribe(topic, Box::new(subscriber));
        println!("Subscribed to event queue");
        
        receiver
    }
    
    #[tokio::test]
    async fn test_ready_to_handle_randomness_task_listener() -> NodeResult<()> {
        println!("Starting test_ready_to_handle_randomness_task_listener");
        
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
        
        let request_id1 = [1u8; 32]; 
        let request_id2 = [2u8; 32]; 
        let request_id3 = [3u8; 32]; 

        let pending_request_ids = vec![request_id1, request_id3];
        
        let adapter_address = deploy_mock_adapter(
            client.clone(), 
            pending_request_ids,
        ).await.map_err(|e| anyhow!("Failed to deploy mock adapter contract: {}", e))?;
        
        println!("Adapter contract deployed at: {}", adapter_address);
        
        let controller_address = Address::random(); 
        
        let config = Config::default();
        
        let chain_identity = GeneralMainChainIdentity::new(
            chain_id,
            wallet.clone(),
            ws_provider.clone(),
            anvil.ws_endpoint(),
            controller_address,
            Address::random(),  
            adapter_address,    
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
        );
        println!("Chain identity created");
        
        let block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>> =
            Arc::new(RwLock::new(Box::new(InMemoryBlockInfoCache::new(100, 12))));
        
        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> =
            Arc::new(RwLock::new(Box::new(InMemoryGroupInfoCache::<G2Curve>::new(id_address))));
        
        let randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>> =
            Arc::new(RwLock::new(Box::new(InMemoryBLSTasksQueue::new())));
        
        println!("Caches created");
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        println!("Event queue created");
        
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            println!("Setting up test subscriber");
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber", chain_id).await
        };
        
        {
            let mut group_cache_write = group_cache.write().await;
            let dkg_task = arpa_core::DKGTask {
                group_index: 1,
                epoch: 1,
                size: 3,
                threshold: 2,
                assignment_block_height: 90,
                members: vec![id_address],
                coordinator_address: Address::random()
            };
            group_cache_write.save_task_info(0, dkg_task).await?;
    
            group_cache_write.save_committers(1, 1, vec![id_address]).await?;
            
            group_cache_write.update_dkg_status(1, 1, arpa_core::DKGStatus::CommitSuccess).await?;
            
            println!("Group cache configured with successful DKG state");
        }
        
        {
            let mut block_cache_write = block_cache.write().await;
            block_cache_write.set_block_height(100);
            println!("Block cache updated with height 100");
        }
        
        {
            let mut tasks_cache_write = randomness_tasks_cache.write().await;
            
            let task1 = RandomnessTask {
                request_id: request_id1.to_vec(),
                subscription_id: 1,
                group_index: 1,
                request_type: RandomnessRequestType::Randomness,
                params: vec![0u8, 1u8, 2u8],
                requester: Address::random(),
                seed: U256::from(123456),
                request_confirmations: 5,
                callback_gas_limit: 100000,
                callback_max_gas_price: U256::from(10000000000u64),
                assignment_block_height: 90,
            };
            
            let task3 = RandomnessTask {
                request_id: request_id3.to_vec(),
                subscription_id: 3,
                group_index: 1,
                request_type: RandomnessRequestType::Randomness,
                params: vec![6u8, 7u8, 8u8],
                requester: Address::random(),
                seed: U256::from(789012),
                request_confirmations: 5,
                callback_gas_limit: 100000,
                callback_max_gas_price: U256::from(10000000000u64),
                assignment_block_height: 90,
            };
            
            tasks_cache_write.add(task1).await?;
            tasks_cache_write.add(task3).await?;
            println!("Added 2 randomness tasks to cache");
        }
        
        let exclusive_window = 10;
        let listener = ReadyToHandleRandomnessTaskListener::<G2Curve>::new(
            chain_id,
            id_address,
            chain_identity_arc.clone(),
            block_cache.clone(),
            group_cache.clone(),
            randomness_tasks_cache.clone(),
            event_queue.clone(),
            exclusive_window,
        );
        println!("Listener created");
        
        println!("\nTEST PART 1: Testing direct publishing through listener");
        let test_task = RandomnessTask {
            request_id: request_id2.to_vec(),
            subscription_id: 2,
            group_index: 1,
            request_type: RandomnessRequestType::Randomness,
            params: vec![3u8, 4u8, 5u8],
            requester: Address::random(),
            seed: U256::from(654321),
            request_confirmations: 3,
            callback_gas_limit: 200000,
            callback_max_gas_price: U256::from(20000000000u64),
            assignment_block_height: 95,
        };
        
        listener.publish(ReadyToHandleRandomnessTask {
            chain_id,
            tasks: vec![test_task.clone()],
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
        
        if let Some(event) = received_event.downcast_ref::<ReadyToHandleRandomnessTask>() {
            println!("Received ReadyToHandleRandomnessTask event with {} tasks", event.tasks.len());
            assert_eq!(event.chain_id, chain_id);
            assert_eq!(event.tasks.len(), 1);
            assert_eq!(event.tasks[0].request_id, test_task.request_id);
            assert_eq!(event.tasks[0].subscription_id, test_task.subscription_id);
            println!("Direct publishing test PASSED!");
        } else {
            println!("Received unexpected event type");
            return Err(anyhow!("Received unexpected event type").into());
        }
        
        println!("\nTEST PART 2: Testing listen method with actual adapter contract");
        let listen_result = listener.listen().await;
        assert!(listen_result.is_ok(), "Listen method should not fail");
        
        println!("Waiting for listen-triggered event...");
        let listen_triggered_event = timeout(Duration::from_secs(5), event_receiver.recv()).await
            .map_err(|_| {
                println!("Timeout: No listen-triggered event received after 5 seconds");
                anyhow!("Timeout: No listen-triggered event received")
            })?
            .ok_or_else(|| {
                println!("Error: Event channel closed");
                anyhow!("Error: Event channel closed")
            })?;
        
        if let Some(event) = listen_triggered_event.downcast_ref::<ReadyToHandleRandomnessTask>() {
            println!("Received ReadyToHandleRandomnessTask event from listen method with {} tasks", event.tasks.len());
            assert_eq!(event.chain_id, chain_id);
            assert!(event.tasks.len() > 0, "Should have at least one task");
            
            let found_task1 = event.tasks.iter().any(|task| task.request_id == request_id1.to_vec());
            let found_task3 = event.tasks.iter().any(|task| task.request_id == request_id3.to_vec());
            
            println!("Found task1: {}, Found task3: {}", found_task1, found_task3);
            assert!(found_task1 || found_task3, "Tasks should include at least one of the expected tasks");
            
            println!("Listen method test PASSED!");
        } else {
            println!("Received unexpected event type from listen method");
            return Err(anyhow!("Received unexpected event type from listen method").into());
        }
        
        println!("\nTEST PART 3: Testing additional methods");
        println!("Testing handle_interruption method");
        let interruption_result = listener.handle_interruption().await;
        assert!(interruption_result.is_ok(), "Handle interruption failed");
        
        println!("Testing chain_id method");
        assert_eq!(listener.chain_id().await, chain_id);
        
        let display_string = format!("{}", listener);
        assert_eq!(display_string, "ReadyToHandleRandomnessTaskListener");
        
        println!("All tests completed successfully!");
        Ok(())
    }
}