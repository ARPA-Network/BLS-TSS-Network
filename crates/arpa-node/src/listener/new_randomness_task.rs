use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::new_randomness_task::NewRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::adapter::AdapterLogs;
use arpa_core::{
    log::{build_task_related_payload, LogType},
    BLSTaskType, RandomnessTask, TaskType,
};
use arpa_dal::BLSTasksHandler;
use async_trait::async_trait;
use ethers::{providers::Middleware, types::Address};
use log::info;
use serde_json::json;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

pub struct NewRandomnessTaskListener<PC: Curve> {
    chain_id: usize,
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
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        NewRandomnessTaskListener {
            chain_id,
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
        let chain_id = self.chain_id;

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
                                self.chain_id,
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

    async fn chain_id(&self) -> usize {
        self.chain_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arpa_dal::cache::InMemoryBLSTasksQueue;
    use ethers::signers::{LocalWallet, Signer};
    use ethers_middleware::SignerMiddleware;
    use ethers::contract::{ContractFactory, abigen};
    use ethers::types::{U256, Bytes, BlockNumber};
    use threshold_bls::schemes::bn254::G2Curve;
    use crate::queue::EventSubscriber;
    use crate::event::types::Topic;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use arpa_core::{
        Config, GeneralMainChainIdentity, RandomnessRequestType, RandomnessTask
    };
    use arpa_dal::BLSTasksHandler;
    use ethers::{
        providers::{Provider, Ws, Http},
        types::Address,
        utils::Anvil,
    };
    use std::time::Duration;
    use tokio::time::timeout;
    use anyhow::anyhow;
    use std::sync::Arc;

    abigen!(
        MockAdapter,
        r#"[
            event RandomnessRequest(bytes32 indexed requestId, uint64 indexed subId, uint32 indexed groupIndex, uint8 requestType, bytes params, address sender, uint256 seed, uint16 requestConfirmations, uint32 callbackGasLimit, uint256 callbackMaxGasPrice, uint256 estimatedPayment)
            function emitRandomnessRequest(bytes32 requestId, uint64 subId, uint32 groupIndex, uint8 requestType, bytes calldata params, address sender, uint256 seed, uint16 requestConfirmations, uint32 callbackGasLimit, uint256 callbackMaxGasPrice, uint256 estimatedPayment) external
        ]"#,
    );

    async fn deploy_mock_adapter(
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    ) -> Result<Address, Box<dyn std::error::Error>> {
        println!("Deploying mock adapter contract...");

        const ADAPTER_ABI: &str = r#"[{"anonymous":false,"inputs":[{"indexed":true,"internalType":"bytes32","name":"requestId","type":"bytes32"},{"indexed":true,"internalType":"uint64","name":"subId","type":"uint64"},{"indexed":true,"internalType":"uint32","name":"groupIndex","type":"uint32"},{"indexed":false,"internalType":"enum IRequestTypeBase.RequestType","name":"requestType","type":"uint8"},{"indexed":false,"internalType":"bytes","name":"params","type":"bytes"},{"indexed":false,"internalType":"address","name":"sender","type":"address"},{"indexed":false,"internalType":"uint256","name":"seed","type":"uint256"},{"indexed":false,"internalType":"uint16","name":"requestConfirmations","type":"uint16"},{"indexed":false,"internalType":"uint32","name":"callbackGasLimit","type":"uint32"},{"indexed":false,"internalType":"uint256","name":"callbackMaxGasPrice","type":"uint256"},{"indexed":false,"internalType":"uint256","name":"estimatedPayment","type":"uint256"}],"name":"RandomnessRequest","type":"event"},{"inputs":[{"internalType":"bytes32","name":"requestId","type":"bytes32"},{"internalType":"uint64","name":"subId","type":"uint64"},{"internalType":"uint32","name":"groupIndex","type":"uint32"},{"internalType":"enum IRequestTypeBase.RequestType","name":"requestType","type":"uint8"},{"internalType":"bytes","name":"params","type":"bytes"},{"internalType":"address","name":"sender","type":"address"},{"internalType":"uint256","name":"seed","type":"uint256"},{"internalType":"uint16","name":"requestConfirmations","type":"uint16"},{"internalType":"uint32","name":"callbackGasLimit","type":"uint32"},{"internalType":"uint256","name":"callbackMaxGasPrice","type":"uint256"},{"internalType":"uint256","name":"estimatedPayment","type":"uint256"}],"name":"emitRandomnessRequest","outputs":[],"stateMutability":"nonpayable","type":"function"}]"#;
        const ADAPTER_BYTECODE: &str = "6080604052348015600e575f5ffd5b506102ec8061001c5f395ff3fe608060405234801561000f575f5ffd5b5060043610610029575f3560e01c8063b3af18b41461002d575b5f5ffd5b61004061003b36600461013c565b610042565b005b8963ffffffff168b67ffffffffffffffff168d7fd26299589dd9197a8dc30a0fa17b0fe7dd432bc3441aa5f5631ea1e14c1af7448c8c8c8c8c8c8c8c8c60405161009499989796959493929190610218565b60405180910390a4505050505050505050505050565b803563ffffffff811681146100bd575f5ffd5b919050565b8035600381106100bd575f5ffd5b5f5f83601f8401126100e0575f5ffd5b50813567ffffffffffffffff8111156100f7575f5ffd5b60208301915083602082850101111561010e575f5ffd5b9250929050565b80356001600160a01b03811681146100bd575f5ffd5b803561ffff811681146100bd575f5ffd5b5f5f5f5f5f5f5f5f5f5f5f5f6101608d8f031215610158575f5ffd5b8c359b5060208d013567ffffffffffffffff81168114610176575f5ffd5b9a5061018460408e016100aa565b995061019260608e016100c2565b985067ffffffffffffffff60808e013511156101ac575f5ffd5b6101bc8e60808f01358f016100d0565b90985096506101cd60a08e01610115565b955060c08d013594506101e260e08e0161012b565b93506101f16101008e016100aa565b9b9e9a9d50989b979a96999598509396929591949193505061012082013591610140013590565b5f60038b1061023557634e487b7160e01b5f52602160045260245ffd5b8a8252610100602083015288610100830152888a6101208401375f6101208a84010152610120601f19601f8b0116830101905061027d60408301896001600160a01b03169052565b866060830152610293608083018761ffff169052565b63ffffffff851660a083015260c082019390935260e0015297965050505050505056fea264697066735822122063806527f6653c276674a3868b2937ce83f46748f5756c48bed55f7ce02f2f2c64736f6c634300081b0033";

        let adapter_factory = ContractFactory::new(
            serde_json::from_str(ADAPTER_ABI).expect("Invalid ADAPTER_ABI"),
            ADAPTER_BYTECODE.parse::<Bytes>().expect("Invalid ADAPTER_BYTECODE"),
            client.clone(),
        );
        
        let adapter_contract_deployed = adapter_factory.deploy(())?.send().await?;
        let adapter_address = adapter_contract_deployed.address();
        println!("Mock Adapter contract deployed at: {}", adapter_address);

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
                if let Some(event) = payload.as_any().downcast_ref::<NewRandomnessTask>() {
                    println!("Received NewRandomnessTask event");
                    let cloned_event = NewRandomnessTask::new(
                        event.chain_id,
                        event.randomness_task.clone(),
                    );
                    let boxed = Box::new(cloned_event) as Box<dyn std::any::Any + Send>;
                    self.sender.send(boxed).await.map_err(|e| {
                        println!("Failed to send event: {}", e);
                        let err: crate::error::NodeError = anyhow!("Failed to send event: {}", e).into();
                        err
                    })?;
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
        
        let subscriber = TestSubscriber {
            name: subscriber_name.to_string(),
            sender,
        };

        let topic = Topic::NewRandomnessTask(chain_id);
        println!("Subscribing to topic: {:?}", topic);
        
        eq.subscribe(topic, Box::new(subscriber));
        println!("Subscribed to event queue");
        
        receiver
    }
    
    #[tokio::test]
    async fn test_new_randomness_task_listener() -> NodeResult<()> {
        println!("Starting test_new_randomness_task_listener");
        
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
        
        let adapter_address = deploy_mock_adapter(client.clone()).await
            .map_err(|e| anyhow!("Failed to deploy mock adapter: {}", e))?;
        
        println!("Mock Adapter deployed at: {}", adapter_address);
        
        let controller_address = Address::random();
        
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
        println!("Chain identity created with adapter address: {}", adapter_address);
        
        let randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>> =
            Arc::new(RwLock::new(Box::new(InMemoryBLSTasksQueue::new())));
        println!("Randomness tasks cache created");
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        println!("Event queue created");
        
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            println!("Setting up test subscriber");
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber", chain_id).await
        };
        
        let mut listener = NewRandomnessTaskListener::<G2Curve>::new(
            chain_id,
            id_address,
            chain_identity_arc.clone(),
            randomness_tasks_cache.clone(),
            event_queue.clone(),
        );
        println!("Listener created");

        println!("Initializing listener...");
        listener.initialize().await?;
        println!("Listener initialized");
        
        let request_id = [1u8; 32];
        let request_id_bytes32: [u8; 32] = request_id;
        let group_index = 1u32;
        let sub_id = 1u64;
        let seed = U256::from(123456);
        let requester = wallet.address();
        let params = vec![0u8, 1u8, 2u8];
        let request_confirmations = 5u16;
        let callback_gas_limit = 100000u32;
        let callback_max_gas_price = U256::from(10000000000u64);
        let estimated_payment = U256::from(1000000);
        
        println!("Test task parameters created");
        
        let current_block = ws_provider.get_block(BlockNumber::Latest).await?
            .ok_or_else(|| anyhow!("Cannot get latest block"))?;
        let current_block_number = current_block.number.unwrap().as_u64();
        println!("Current block number: {}", current_block_number);
        
        println!("Emitting RandomnessRequest event from mock adapter...");
        
        let mock_adapter = MockAdapter::new(adapter_address, client.clone());
        
        let tx_request = mock_adapter.emit_randomness_request(
            request_id_bytes32,
            sub_id,
            group_index,
            0,
            params.clone().into(),
            requester,
            seed,
            request_confirmations,
            callback_gas_limit,
            callback_max_gas_price,
            estimated_payment,
        );
        
        let tx = tx_request.send().await
            .map_err(|e| anyhow!("Failed to send transaction: {:?}", e))?;
        
        println!("Transaction sent: {:?}", tx.tx_hash());
        
        let receipt = tx.await?;
        println!("Transaction mined in block: {}", receipt.unwrap().block_number.unwrap());
        
        println!("Starting listener.listen()...");
        
        let listen_handle = tokio::spawn(async move {
            let result = listener.listen().await;
            if result.is_err() {
                println!("Listener.listen() failed: {:?}", result);
            }
            result
        });
        
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        println!("Waiting for event to be received...");
        let received_event = timeout(Duration::from_secs(10), event_receiver.recv()).await
            .map_err(|_| {
                println!("Timeout: No event received after 10 seconds");
                anyhow!("Timeout: No event received")
            })?
            .ok_or_else(|| {
                println!("Error: Event channel closed");
                anyhow!("Error: Event channel closed")
            })?;
        
        println!("Event received!");
        
        listen_handle.abort();
        
        if let Some(event) = received_event.downcast_ref::<NewRandomnessTask>() {
            println!("Received NewRandomnessTask event with chain_id: {}", event.chain_id);
            assert_eq!(event.chain_id, chain_id);
            assert_eq!(event.randomness_task.request_id, request_id);
            assert_eq!(event.randomness_task.subscription_id, sub_id);
            assert_eq!(event.randomness_task.group_index, group_index);
            assert_eq!(event.randomness_task.request_type, RandomnessRequestType::Randomness);
            assert_eq!(event.randomness_task.params, params);
            assert_eq!(event.randomness_task.requester, requester);
            assert_eq!(event.randomness_task.seed, seed);
            assert_eq!(event.randomness_task.request_confirmations, request_confirmations);
            assert_eq!(event.randomness_task.callback_gas_limit, callback_gas_limit);
            assert_eq!(event.randomness_task.callback_max_gas_price, callback_max_gas_price);
            
            let assignment_block = event.randomness_task.assignment_block_height;
            println!("Assignment block height: {}", assignment_block);
            assert!(assignment_block > 0, "Assignment block height should be greater than 0");
            
            assert!(
                (assignment_block as u64) <= current_block_number + 1,
                "Assignment block height should be close to current block"
            );
        } else {
            println!("Received unexpected event type");
            return Err(anyhow!("Received unexpected event type").into());
        }
        
        {
            let cache_read = randomness_tasks_cache.read().await;
            println!("Checking if task is in cache...");
            let contains = cache_read.contains(&request_id.to_vec()).await?;
            assert!(contains, "Task should be saved to cache");
            
            let task = cache_read.get(&request_id.to_vec()).await?;
            assert_eq!(task.subscription_id, sub_id);
            assert_eq!(task.group_index, group_index);
            assert_eq!(task.request_type, RandomnessRequestType::Randomness);
            println!("Verified task is correctly stored in cache");
        }
        
        println!("Testing handle_interruption method");
        let interruption_result = NewRandomnessTaskListener::<G2Curve>::new(
            chain_id,
            id_address,
            chain_identity_arc.clone(),
            randomness_tasks_cache.clone(),
            event_queue.clone(),
        ).handle_interruption().await;
        assert!(interruption_result.is_ok(), "Handle interruption failed");
        
        println!("Testing chain_id method");
        let test_listener = NewRandomnessTaskListener::<G2Curve>::new(
            chain_id,
            id_address,
            chain_identity_arc.clone(),
            randomness_tasks_cache.clone(),
            event_queue.clone(),
        );
        assert_eq!(test_listener.chain_id().await, chain_id);
        
        let display_string = format!("{}", test_listener);
        assert_eq!(display_string, "NewRandomnessTaskListener");
        
        println!("Test completed successfully");
        Ok(())
    }
}