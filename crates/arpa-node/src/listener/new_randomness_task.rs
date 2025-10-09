use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::new_randomness_task::NewRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use alloy::primitives::Address;
use alloy::providers::Provider;
use arpa_contract_client::adapter::AdapterLogs;
use arpa_core::{
    log::{build_task_related_payload, LogType},
    BLSTaskType, ListenerDescriptor, RandomnessTask, TaskType,
};
use arpa_dal::BLSTasksHandler;
use async_trait::async_trait;
use log::info;
use serde_json::json;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct NewRandomnessTaskListener<PC: Curve> {
    listener_descriptor: ListenerDescriptor,
    id_address: Address,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> std::fmt::Display for NewRandomnessTaskListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NewRandomnessTaskListener")
    }
}

impl<PC: Curve> NewRandomnessTaskListener<PC> {
    pub fn new(
        listener_descriptor: ListenerDescriptor,
        id_address: Address,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        NewRandomnessTaskListener {
            listener_descriptor,
            id_address,
            chain_identity,
            randomness_tasks_cache,
            eq,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> EventPublisher<NewRandomnessTask> for NewRandomnessTaskListener<PC> {
    async fn publish(&self, event: NewRandomnessTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for NewRandomnessTaskListener<PC> {
    async fn listen(&self) -> NodeResult<()> {
        let client = self
            .chain_identity
            .read()
            .await
            .build_adapter_client(self.id_address);
        let chain_id = self.listener_descriptor.chain_id;

        client
            .subscribe_randomness_task(move |randomness_task| {
                let randomness_tasks_cache = self.randomness_tasks_cache.clone();
                let eq = self.eq.clone();

                async move {
                    let contained_res = randomness_tasks_cache
                        .read()
                        .await
                        .contains(&randomness_task.request_id)
                        .await;
                    if let Ok(false) = contained_res {
                        info!(
                            "{}",
                            build_task_related_payload(
                                LogType::TaskReceived,
                                "New randomness task received.",
                                chain_id,
                                &randomness_task.request_id,
                                TaskType::BLS(BLSTaskType::Randomness),
                                json!(randomness_task),
                                None
                            )
                        );

                        randomness_tasks_cache
                            .write()
                            .await
                            .add(randomness_task.clone())
                            .await
                            .map_err(anyhow::Error::from)?;

                        eq.read()
                            .await
                            .publish(NewRandomnessTask::new(chain_id, randomness_task))
                            .await;
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
    use crate::event::types::Topic;
    use crate::queue::EventSubscriber;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use crate::test_contracts::mockadapter::deploy_and_get_mock_adapter;
    use anyhow::anyhow;
    use arpa_core::{
        Config, FixedIntervalRetryDescriptor, GeneralMainChainIdentity, ListenerType,
        RandomnessRequestType, RandomnessTask,
    };
    use arpa_dal::cache::InMemoryBLSTasksQueue;
    use arpa_dal::BLSTasksHandler;
    use ethers::middleware::SignerMiddleware;
    use ethers::signers::{LocalWallet, Signer};
    use ethers::types::{BlockNumber, U256};
    use ethers::{
        providers::{Http, Provider, Ws},
        types::Address,
        utils::{Anvil, AnvilInstance},
    };
    use std::sync::Arc;
    use std::time::Duration;
    use threshold_bls::schemes::bn254::G2Curve;
    use tokio::time::timeout;

    struct TestEnvironment {
        _anvil: AnvilInstance, // Keep anvil alive for the duration of the test
        ws_provider: Arc<Provider<Ws>>,
        wallet: LocalWallet,
        id_address: Address,
        chain_id: u64,
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
        chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>>,
        randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
        event_queue: Arc<RwLock<EventQueue>>,
    }

    struct TestEventParams {
        request_id: [u8; 32],
        group_index: u32,
        sub_id: u64,
        seed: U256,
        requester: Address,
        params: Vec<u8>,
        request_confirmations: u16,
        callback_gas_limit: u32,
        callback_max_gas_price: U256,
        estimated_payment: U256,
    }

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
            if let Some(event) = payload.as_any().downcast_ref::<NewRandomnessTask>() {
                println!("Received NewRandomnessTask event");
                let cloned_event =
                    NewRandomnessTask::new(event.chain_id, event.randomness_task.clone());
                let boxed = Box::new(cloned_event) as Box<dyn std::any::Any + Send>;
                if let Err(e) = self.sender.send(boxed).await {
                    println!("Failed to send event: {}", e);
                    return Err(anyhow!("Failed to send event: {}", e).into());
                }
                println!("Event sent to receiver");
            } else {
                println!("Payload is not a NewRandomnessTask event");
            }
            Ok(())
        }

        async fn subscribe(self) {
            println!("TestSubscriber subscribed");
        }
    }

    impl DebuggableSubscriber for TestSubscriber {}

    impl TestEnvironment {
        async fn new() -> NodeResult<Self> {
            println!("Setting up test environment...");

            let anvil = Anvil::new().spawn();
            println!("Anvil instance started at {}", anvil.endpoint());

            tokio::time::sleep(Duration::from_millis(100)).await;

            let http_provider = Provider::<Http>::try_from(anvil.endpoint())
                .map_err(|e| anyhow!("Failed to create HTTP provider: {}", e))?;

            let chain_id_test = http_provider
                .get_chainid()
                .await
                .map_err(|e| anyhow!("Failed to get chain ID from HTTP provider: {}", e))?;
            println!("HTTP provider connected, chain ID: {}", chain_id_test);

            let ws_provider = Arc::new(
                Provider::<Ws>::connect(&anvil.ws_endpoint())
                    .await
                    .map_err(|e| anyhow!("Failed to connect to WebSocket: {}", e))?,
            );
            println!("Connected to Anvil WebSocket at {}", anvil.ws_endpoint());

            let ws_chain_id = ws_provider
                .get_chainid()
                .await
                .map_err(|e| anyhow!("Failed to get chain ID from WS provider: {}", e))?;
            println!("WebSocket provider connected, chain ID: {}", ws_chain_id);

            let wallet: LocalWallet = anvil.keys()[0].clone().into();
            let id_address = wallet.address();
            println!("Using wallet address: {}", id_address);

            let chain_id = anvil.chain_id() as usize;
            println!("Chain ID: {}", chain_id);

            let client = Arc::new(SignerMiddleware::new(
                http_provider,
                wallet.clone().with_chain_id(anvil.chain_id()),
            ));

            let mock_adapter = deploy_and_get_mock_adapter(client.clone())
                .await
                .map_err(|e| anyhow!("Failed to deploy mock adapter: {}", e))?;
            let adapter_address = mock_adapter.address();
            println!("Mock Adapter deployed at: {}", adapter_address);

            let controller_address = random_address();
            let config = Config::default();
            let chain_identity = GeneralMainChainIdentity::new(
                chain_id,
                wallet.clone(),
                ws_provider.clone(),
                anvil.ws_endpoint(),
                controller_address,
                adapter_address,
                random_address(),
                config
                    .get_time_limits()
                    .contract_transaction_retry_descriptor,
                config.get_time_limits().contract_view_retry_descriptor,
                None,
            );
            println!(
                "Chain identity created with adapter address: {}",
                adapter_address
            );

            let randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>> =
                Arc::new(RwLock::new(Box::new(InMemoryBLSTasksQueue::new())));
            println!("Randomness tasks cache created");

            let event_queue = Arc::new(RwLock::new(EventQueue::new()));
            println!("Event queue created");

            let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = Arc::new(
                RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>),
            );

            Ok(TestEnvironment {
                _anvil: anvil, // Keep anvil alive
                ws_provider,
                wallet,
                id_address,
                chain_id,
                client,
                chain_identity_arc,
                randomness_tasks_cache,
                event_queue,
            })
        }

        fn create_test_listener(&self) -> NewRandomnessTaskListener<G2Curve> {
            let listener_descriptor = ListenerDescriptor {
                chain_id: self.chain_id,
                l_type: ListenerType::NewRandomnessTask,
                interval_millis: 1000,
                use_jitter: true,
                reset_descriptor: FixedIntervalRetryDescriptor {
                    interval_millis: 5000,
                    max_attempts: 3,
                    use_jitter: true,
                },
            };

            NewRandomnessTaskListener::<G2Curve>::new(
                listener_descriptor,
                self.id_address,
                self.chain_identity_arc.clone(),
                self.randomness_tasks_cache.clone(),
                self.event_queue.clone(),
            )
        }

        async fn setup_event_subscription(
            &self,
        ) -> tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>> {
            let (sender, receiver) = tokio::sync::mpsc::channel(100);

            let subscriber = TestSubscriber {
                name: "test_subscriber".to_string(),
                sender,
            };

            let topic = Topic::NewRandomnessTask(self.chain_id);
            println!("Subscribing to topic: {:?}", topic);

            let mut eq_write = self.event_queue.write().await;
            eq_write.subscribe(topic, Box::new(subscriber));
            println!("Subscribed to event queue");

            receiver
        }

        fn create_test_event_params(&self) -> TestEventParams {
            TestEventParams {
                request_id: [1u8; 32],
                group_index: 1u32,
                sub_id: 1u64,
                seed: U256::from(123456),
                requester: self.wallet.address(),
                params: vec![0u8, 1u8, 2u8],
                request_confirmations: 5u16,
                callback_gas_limit: 100000u32,
                callback_max_gas_price: U256::from(10000000000u64),
                estimated_payment: U256::from(1000000),
            }
        }

        async fn emit_test_event(&self, params: &TestEventParams) -> NodeResult<u64> {
            println!("Emitting RandomnessRequest event from mock adapter...");

            let mock_adapter = deploy_and_get_mock_adapter(self.client.clone())
                .await
                .map_err(|e| anyhow!("Failed to get mock adapter: {}", e))?;

            let tx_request = mock_adapter.emit_randomness_request(
                params.request_id,
                params.sub_id,
                params.group_index,
                0,
                params.params.clone().into(),
                params.requester,
                params.seed,
                params.request_confirmations,
                params.callback_gas_limit,
                params.callback_max_gas_price,
                params.estimated_payment,
            );

            let tx = tx_request
                .send()
                .await
                .map_err(|e| anyhow!("Failed to send transaction: {:?}", e))?;

            println!("Transaction sent: {:?}", tx.tx_hash());

            let receipt = tx.await?;
            let block_number = receipt.unwrap().block_number.unwrap().as_u64();
            println!("Transaction mined in block: {}", block_number);

            Ok(block_number)
        }

        async fn get_current_block_number(&self) -> NodeResult<u64> {
            let current_block = self
                .ws_provider
                .get_block(BlockNumber::Latest)
                .await?
                .ok_or_else(|| anyhow!("Cannot get latest block"))?;
            Ok(current_block.number.unwrap().as_u64())
        }
    }

    impl TestEventParams {
        fn verify_against_event(
            &self,
            event: &NewRandomnessTask,
            expected_chain_id: u64,
            _current_block_number: u64,
        ) -> NodeResult<()> {
            println!(
                "Received NewRandomnessTask event with chain_id: {}",
                event.chain_id
            );
            assert_eq!(event.chain_id, expected_chain_id);
            assert_eq!(event.randomness_task.request_id, self.request_id);
            assert_eq!(event.randomness_task.subscription_id, self.sub_id);
            assert_eq!(event.randomness_task.group_index, self.group_index);
            assert_eq!(
                event.randomness_task.request_type,
                RandomnessRequestType::Randomness
            );
            assert_eq!(event.randomness_task.params, self.params);
            assert_eq!(event.randomness_task.requester, self.requester);
            assert_eq!(event.randomness_task.seed, self.seed);
            assert_eq!(
                event.randomness_task.request_confirmations,
                self.request_confirmations
            );
            assert_eq!(
                event.randomness_task.callback_gas_limit,
                self.callback_gas_limit
            );
            assert_eq!(
                event.randomness_task.callback_max_gas_price,
                self.callback_max_gas_price
            );

            let assignment_block = event.randomness_task.assignment_block_height;
            println!("Assignment block height: {}", assignment_block);
            assert!(
                assignment_block > 0,
                "Assignment block height should be greater than 0"
            );

            Ok(())
        }

        async fn verify_in_cache(
            &self,
            cache: &Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
        ) -> NodeResult<()> {
            let cache_read = cache.read().await;
            println!("Checking if task is in cache...");
            let contains = cache_read.contains(&self.request_id.to_vec()).await?;
            assert!(contains, "Task should be saved to cache");

            let task = cache_read.get(&self.request_id.to_vec()).await?;
            assert_eq!(task.subscription_id, self.sub_id);
            assert_eq!(task.group_index, self.group_index);
            assert_eq!(task.request_type, RandomnessRequestType::Randomness);
            println!("Verified task is correctly stored in cache");

            Ok(())
        }
    }

    async fn wait_for_event(
        mut event_receiver: tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>>,
    ) -> NodeResult<NewRandomnessTask> {
        println!("Waiting for event to be received...");
        let received_event = timeout(Duration::from_secs(10), event_receiver.recv())
            .await
            .map_err(|_| {
                println!("Timeout: No event received after 10 seconds");
                anyhow!("Timeout: No event received")
            })?
            .ok_or_else(|| {
                println!("Error: Event channel closed");
                anyhow!("Error: Event channel closed")
            })?;

        println!("Event received!");

        if let Some(event) = received_event.downcast_ref::<NewRandomnessTask>() {
            Ok(event.clone())
        } else {
            println!("Received unexpected event type");
            Err(anyhow!("Received unexpected event type").into())
        }
    }

    fn test_listener_methods(env: &TestEnvironment) {
        println!("Testing listener methods...");
        let test_listener = env.create_test_listener();

        assert_eq!(test_listener.chain_id(), env.chain_id);

        let display_string = format!("{}", test_listener);
        assert_eq!(display_string, "NewRandomnessTaskListener");

        println!("Listener methods verified");
    }

    async fn test_handle_interruption(env: &TestEnvironment) -> NodeResult<()> {
        println!("Testing handle_interruption method");
        let test_listener = env.create_test_listener();
        let interruption_result = test_listener.handle_interruption().await;
        assert!(interruption_result.is_ok(), "Handle interruption failed");
        println!("Handle interruption test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_new_randomness_task_listener() -> NodeResult<()> {
        println!("Starting test_new_randomness_task_listener");

        let env = TestEnvironment::new().await?;

        let event_receiver = env.setup_event_subscription().await;

        let listener = env.create_test_listener();
        println!("Listener created");

        let test_params = env.create_test_event_params();

        let current_block_number = env.get_current_block_number().await?;
        println!("Current block number: {}", current_block_number);

        println!("Starting listener.listen()...");
        let listen_handle = tokio::spawn(async move {
            let result = listener.listen().await;
            if result.is_err() {
                println!("Listener.listen() failed: {:?}", result);
            }
            result
        });

        tokio::time::sleep(Duration::from_millis(2000)).await;

        env.emit_test_event(&test_params).await?;

        let received_event = wait_for_event(event_receiver).await?;
        test_params.verify_against_event(&received_event, env.chain_id, current_block_number)?;

        listen_handle.abort();

        test_params
            .verify_in_cache(&env.randomness_tasks_cache)
            .await?;

        test_handle_interruption(&env).await?;
        test_listener_methods(&env);

        println!("Test completed successfully");
        Ok(())
    }
}
