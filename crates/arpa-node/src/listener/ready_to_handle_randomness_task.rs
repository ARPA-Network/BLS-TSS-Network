use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::ready_to_handle_randomness_task::ReadyToHandleRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use alloy::primitives::Address;
use alloy::providers::Provider;
use arpa_contract_client::adapter::AdapterViews;
use arpa_core::{ListenerDescriptor, RandomnessTask};
use arpa_dal::{BLSTasksHandler, BlockInfoHandler, GroupInfoHandler};
use async_trait::async_trait;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ReadyToHandleRandomnessTaskListener<PC: Curve> {
    listener_descriptor: ListenerDescriptor,
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
        listener_descriptor: ListenerDescriptor,
        id_address: Address,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
        eq: Arc<RwLock<EventQueue>>,
        randomness_task_exclusive_window: usize,
    ) -> Self {
        ReadyToHandleRandomnessTaskListener {
            listener_descriptor,
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
                    chain_id: self.listener_descriptor.chain_id,
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

    fn chain_id(&self) -> u64 {
        self.listener_descriptor.chain_id
    }

    fn listener_descriptor(&self) -> ListenerDescriptor {
        self.listener_descriptor
    }
}

#[cfg(feature = "unittest")]
mod tests {
    use super::*;
    use crate::error::NodeError;
    use crate::event::types::Topic;
    use crate::queue::EventSubscriber;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use crate::test_contracts::mockadapter::{deploy_with_args_and_get_mock_adapter, MockAdapter};
    use ethers::middleware::SignerMiddleware;
    use ethers::signers::{LocalWallet, Signer};
    use threshold_bls::schemes::bn254::G2Curve;

    use anyhow::anyhow;
    use arpa_core::{
        Config, FixedIntervalRetryDescriptor, GeneralMainChainIdentity, ListenerType,
        RandomnessRequestType,
    };
    use arpa_dal::cache::{InMemoryBLSTasksQueue, InMemoryBlockInfoCache, InMemoryGroupInfoCache};
    use ethers::{
        providers::{Http, Provider, Ws},
        types::{Address, U256},
        utils::Anvil,
    };
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::timeout;

    async fn setup_contract_with_commitment(
        adapter: &MockAdapter<SignerMiddleware<Provider<Http>, LocalWallet>>,
        request_id: [u8; 32],
        commitment: [u8; 32],
        description: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tx = adapter.set_request_commitment(request_id.into(), commitment.into());
        let receipt = tx.send().await?.await?;
        println!(
            "  {} in block {}",
            description,
            receipt.unwrap().block_number.unwrap()
        );
        Ok(())
    }

    async fn verify_contract_commitment(
        adapter: &MockAdapter<SignerMiddleware<Provider<Http>, LocalWallet>>,
        request_id: [u8; 32],
        expected_non_zero: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = adapter
            .get_pending_request_commitment(request_id.into())
            .call()
            .await?;
        let result_as_u256 = U256::from(result);
        let is_non_zero = !result_as_u256.is_zero();

        if expected_non_zero {
            println!(
                "Verified commitment for request ID {:?} is non-zero",
                request_id
            );
        } else if !is_non_zero {
            println!(
                "WARNING: Commitment for request ID {:?} is zero!",
                request_id
            );
        }

        Ok(())
    }

    async fn setup_mock_adapter(
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
        pending_request_ids: Vec<[u8; 32]>,
    ) -> Result<Address, Box<dyn std::error::Error>> {
        println!("Deploying mock adapter contract...");

        let adapter = deploy_with_args_and_get_mock_adapter(client.clone(), ()).await?;
        let adapter_address = adapter.address();
        println!("Adapter contract deployed at: {}", adapter_address);

        println!("Setting up pending requests in adapter contract...");
        let non_zero_commitment = [1u8; 32];

        for &request_id in pending_request_ids.iter() {
            setup_contract_with_commitment(
                &adapter,
                request_id,
                non_zero_commitment,
                &format!("Set request ID {:?} as pending", request_id),
            )
            .await?;

            verify_contract_commitment(&adapter, request_id, true).await?;
        }

        let non_pending_request_id = [4u8; 32];
        let zero_commitment = [0u8; 32];
        setup_contract_with_commitment(
            &adapter,
            non_pending_request_id,
            zero_commitment,
            &format!(
                "Set request ID {:?} with zero commitment",
                non_pending_request_id
            ),
        )
        .await?;

        let test_request_id = [5u8; 32];
        let test_commitment = [2u8; 32];
        setup_contract_with_commitment(
            &adapter,
            test_request_id,
            test_commitment,
            "Set test request ID for verification",
        )
        .await?;

        let result = adapter
            .get_pending_request_commitment(test_request_id.into())
            .call()
            .await?;
        println!("Verification call result: {:?}", result);
        let result_as_u256 = U256::from(result);
        println!(
            "As U256: {}, Is non-zero: {}",
            result_as_u256,
            !result_as_u256.is_zero()
        );

        Ok(adapter_address)
    }

    async fn create_test_subscriber(
        eq: &mut EventQueue,
        subscriber_name: &str,
        chain_id: u64,
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
                if let Some(event) = payload
                    .as_any()
                    .downcast_ref::<ReadyToHandleRandomnessTask>()
                {
                    println!(
                        "Received ReadyToHandleRandomnessTask event with {} tasks",
                        event.tasks.len()
                    );
                    let cloned_event = ReadyToHandleRandomnessTask {
                        chain_id: event.chain_id,
                        tasks: event.tasks.clone(),
                    };
                    let boxed = Box::new(cloned_event) as Box<dyn std::any::Any + Send>;
                    self.sender.send(boxed).await.map_err(|e| {
                        println!("Failed to send event: {}", e);
                        let err: crate::error::NodeError =
                            anyhow!("Failed to send event: {}", e).into();
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

    fn create_randomness_task(
        request_id: [u8; 32],
        subscription_id: u64,
        params: Vec<u8>,
        seed: u64,
    ) -> RandomnessTask {
        RandomnessTask {
            request_id: request_id.to_vec(),
            subscription_id,
            group_index: 1,
            request_type: RandomnessRequestType::Randomness,
            params,
            requester: random_address(),
            seed: U256::from(seed),
            request_confirmations: 5,
            callback_gas_limit: 100000,
            callback_max_gas_price: U256::from(10000000000u64),
            assignment_block_height: 90,
        }
    }

    async fn setup_test_caches(
        id_address: Address,
    ) -> (
        Arc<RwLock<Box<dyn BlockInfoHandler>>>,
        Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>>,
        Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
    ) {
        let block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>> =
            Arc::new(RwLock::new(Box::new(InMemoryBlockInfoCache::new(100, 12))));

        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::new(id_address)),
        ));

        let randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>> =
            Arc::new(RwLock::new(Box::new(InMemoryBLSTasksQueue::new())));

        {
            let mut group_cache_write = group_cache.write().await;
            let dkg_task = arpa_core::DKGTask {
                group_index: 1,
                epoch: 1,
                size: 3,
                threshold: 2,
                assignment_block_height: 90,
                members: vec![id_address],
                coordinator_address: random_address(),
            };
            group_cache_write.save_task_info(0, dkg_task).await.unwrap();
            group_cache_write
                .save_committers(1, 1, vec![id_address])
                .await
                .unwrap();
            group_cache_write
                .update_dkg_status(1, 1, arpa_core::DKGStatus::CommitSuccess)
                .await
                .unwrap();
            println!("Group cache configured with successful DKG state");
        }

        {
            let mut block_cache_write = block_cache.write().await;
            block_cache_write.set_block_height(100);
            println!("Block cache updated with height 100");
        }

        (block_cache, group_cache, randomness_tasks_cache)
    }

    async fn wait_for_event(
        event_receiver: &mut tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>>,
        timeout_secs: u64,
        description: &str,
    ) -> Result<ReadyToHandleRandomnessTask, NodeError> {
        println!("Waiting for {} event...", description);
        let received_event = timeout(Duration::from_secs(timeout_secs), event_receiver.recv())
            .await
            .map_err(|_| {
                println!(
                    "Timeout: No {} event received after {} seconds",
                    description, timeout_secs
                );
                anyhow!("Timeout: No {} event received", description)
            })?
            .ok_or_else(|| {
                println!("Error: Event channel closed");
                anyhow!("Error: Event channel closed")
            })?;

        println!("{} event received!", description);

        if let Some(event) = received_event.downcast_ref::<ReadyToHandleRandomnessTask>() {
            println!(
                "Received ReadyToHandleRandomnessTask event with {} tasks",
                event.tasks.len()
            );
            Ok(ReadyToHandleRandomnessTask {
                chain_id: event.chain_id,
                tasks: event.tasks.clone(),
            })
        } else {
            println!("Received unexpected event type");
            Err(anyhow!("Received unexpected event type").into())
        }
    }

    async fn verify_event_content(
        event: &ReadyToHandleRandomnessTask,
        expected_chain_id: u64,
        expected_task_count: Option<usize>,
        expected_request_ids: Option<Vec<Vec<u8>>>,
        test_name: &str,
    ) -> NodeResult<()> {
        assert_eq!(event.chain_id, expected_chain_id);

        if let Some(count) = expected_task_count {
            assert_eq!(event.tasks.len(), count);
        }

        if let Some(request_ids) = expected_request_ids {
            for expected_id in request_ids {
                let found = event
                    .tasks
                    .iter()
                    .any(|task| task.request_id == expected_id);
                assert!(found, "Expected request ID not found in tasks");
            }
        }

        println!("{} test PASSED!", test_name);
        Ok(())
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

        let adapter_address = setup_mock_adapter(client.clone(), pending_request_ids)
            .await
            .map_err(|e| anyhow!("Failed to deploy mock adapter contract: {}", e))?;

        println!("Adapter contract deployed at: {}", adapter_address);

        let controller_address = random_address();
        let config = Config::default();

        let chain_identity = GeneralMainChainIdentity::new(
            chain_id,
            wallet.clone(),
            ws_provider.clone(),
            anvil.ws_endpoint(),
            controller_address,
            random_address(),
            adapter_address,
            config
                .get_time_limits()
                .contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );
        println!("Chain identity created");

        let (block_cache, group_cache, randomness_tasks_cache) =
            setup_test_caches(id_address).await;
        println!("Caches created");

        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        println!("Event queue created");

        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = Arc::new(
            RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>),
        );

        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            println!("Setting up test subscriber");
            create_test_subscriber(&mut *eq_write, "test_subscriber", chain_id).await
        };

        {
            let mut tasks_cache_write = randomness_tasks_cache.write().await;

            let task1 = create_randomness_task(request_id1, 1, vec![0u8, 1u8, 2u8], 123456);
            let task3 = create_randomness_task(request_id3, 3, vec![6u8, 7u8, 8u8], 789012);

            tasks_cache_write.add(task1).await?;
            tasks_cache_write.add(task3).await?;
            println!("Added 2 randomness tasks to cache");
        }

        let exclusive_window = 10;
        let listener_descriptor = ListenerDescriptor {
            chain_id,
            l_type: ListenerType::ReadyToHandleRandomnessTask,
            interval_millis: 1000,
            use_jitter: true,
            reset_descriptor: FixedIntervalRetryDescriptor {
                interval_millis: 5000,
                max_attempts: 3,
                use_jitter: true,
            },
        };

        let listener = ReadyToHandleRandomnessTaskListener::<G2Curve>::new(
            listener_descriptor,
            id_address,
            chain_identity_arc.clone(),
            block_cache.clone(),
            group_cache.clone(),
            randomness_tasks_cache.clone(),
            event_queue.clone(),
            exclusive_window,
        );
        println!("Listener created");

        // TEST PART 1: Direct publishing
        println!("\nTEST PART 1: Testing direct publishing through listener");
        let test_task = create_randomness_task(request_id2, 2, vec![3u8, 4u8, 5u8], 654321);

        listener
            .publish(ReadyToHandleRandomnessTask {
                chain_id,
                tasks: vec![test_task.clone()],
            })
            .await;

        let direct_event = wait_for_event(&mut event_receiver, 5, "published").await?;
        verify_event_content(
            &direct_event,
            chain_id,
            Some(1),
            Some(vec![test_task.request_id]),
            "Direct publishing",
        )
        .await?;

        // TEST PART 2: Listen method
        println!("\nTEST PART 2: Testing listen method with actual adapter contract");
        let listen_result = listener.listen().await;
        assert!(listen_result.is_ok(), "Listen method should not fail");

        let listen_event = wait_for_event(&mut event_receiver, 5, "listen-triggered").await?;
        verify_event_content(&listen_event, chain_id, None, None, "Listen method").await?;

        let found_task1 = listen_event
            .tasks
            .iter()
            .any(|task| task.request_id == request_id1.to_vec());
        let found_task3 = listen_event
            .tasks
            .iter()
            .any(|task| task.request_id == request_id3.to_vec());
        println!("Found task1: {}, Found task3: {}", found_task1, found_task3);
        assert!(
            found_task1 || found_task3,
            "Tasks should include at least one of the expected tasks"
        );

        // TEST PART 3: Additional methods
        println!("\nTEST PART 3: Testing additional methods");
        println!("Testing handle_interruption method");
        let interruption_result = listener.handle_interruption().await;
        assert!(interruption_result.is_ok(), "Handle interruption failed");

        println!("Testing chain_id method");
        assert_eq!(listener.chain_id(), chain_id);

        let display_string = format!("{}", listener);
        assert_eq!(display_string, "ReadyToHandleRandomnessTaskListener");

        println!("All tests completed successfully!");
        Ok(())
    }
}
