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
    TaskType,
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
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        PreGroupingListener {
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
                    let chain_id = self.chain_id().await;

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

    async fn chain_id(&self) -> usize {
        self.chain_identity.read().await.get_chain_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::signers::{LocalWallet, Signer};
    use ethers_middleware::SignerMiddleware;
    use ethers::contract::{ContractFactory, abigen};
    use ethers_contract::EthEvent;
    use ethers_core::types::{U256, Bytes};
    use threshold_bls::schemes::bn254::G2Curve;
    use crate::queue::EventSubscriber;
    use crate::event::types::Topic;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use arpa_core::{
        GeneralMainChainIdentity, Config, DKGTask
    };
    use arpa_dal::{
        cache::InMemoryGroupInfoCache,
        GroupInfoHandler
    };
    use ethers::{
        providers::{Provider, Ws, Http},
        types::Address,
        utils::Anvil,
    };
    use std::time::Duration;
    use tokio::time::timeout;
    use anyhow::anyhow;
    use std::sync::Arc;

    #[derive(Clone, Debug, EthEvent)]
    #[ethevent(
        name = "DkgTask",
        abi = "DkgTask(uint256,uint256,uint256,uint256,uint256,address[],uint256,address)"
    )]
    struct DkgTaskFilter {
        #[ethevent(indexed)]
        global_epoch: U256,
        #[ethevent(indexed)]
        group_index: U256,
        #[ethevent(indexed)]
        group_epoch: U256,
        size: U256,
        threshold: U256,
        members: Vec<Address>,
        assignment_block_height: U256,
        coordinator_address: Address,
    }

    abigen!(
        MockController,
        r#"[
            event DkgTask(uint256 indexed globalEpoch, uint256 indexed groupIndex, uint256 indexed groupEpoch, uint256 size, uint256 threshold, address[] members, uint256 assignmentBlockHeight, address coordinatorAddress)
            function emitDkgTaskEvent(uint256 globalEpoch, uint256 groupIndex, uint256 groupEpoch, uint256 size, uint256 threshold, address[] memory members, uint256 assignmentBlockHeight, address coordinatorAddress) external
        ]"#,
    );

    async fn deploy_mock_controller(
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    ) -> Result<Address, Box<dyn std::error::Error>> {
        println!("Deploying mock controller contract...");

        const CONTROLLER_ABI: &str = r#"[{"anonymous":false,"inputs":[{"indexed":true,"internalType":"uint256","name":"globalEpoch","type":"uint256"},{"indexed":true,"internalType":"uint256","name":"groupIndex","type":"uint256"},{"indexed":true,"internalType":"uint256","name":"groupEpoch","type":"uint256"},{"indexed":false,"internalType":"uint256","name":"size","type":"uint256"},{"indexed":false,"internalType":"uint256","name":"threshold","type":"uint256"},{"indexed":false,"internalType":"address[]","name":"members","type":"address[]"},{"indexed":false,"internalType":"uint256","name":"assignmentBlockHeight","type":"uint256"},{"indexed":false,"internalType":"address","name":"coordinatorAddress","type":"address"}],"name":"DkgTask","type":"event"},{"inputs":[{"internalType":"uint256","name":"globalEpoch","type":"uint256"},{"internalType":"uint256","name":"groupIndex","type":"uint256"},{"internalType":"uint256","name":"groupEpoch","type":"uint256"},{"internalType":"uint256","name":"size","type":"uint256"},{"internalType":"uint256","name":"threshold","type":"uint256"},{"internalType":"address[]","name":"members","type":"address[]"},{"internalType":"uint256","name":"assignmentBlockHeight","type":"uint256"},{"internalType":"address","name":"coordinatorAddress","type":"address"}],"name":"emitDkgTaskEvent","outputs":[],"stateMutability":"nonpayable","type":"function"}]"#;
        const CONTROLLER_BYTECODE: &str = "6080604052348015600e575f5ffd5b5061027b8061001c5f395ff3fe608060405234801561000f575f5ffd5b5060043610610029575f3560e01c80630fc68a0c1461002d575b5f5ffd5b61004061003b3660046100bd565b610042565b005b8587897fbbd25d64683f157b2e3544d3d6430e14102db1e49592cf4dcaf827e2ded517ee888888888860405161007c9594939291906101d0565b60405180910390a45050505050505050565b634e487b7160e01b5f52604160045260245ffd5b80356001600160a01b03811681146100b8575f5ffd5b919050565b5f5f5f5f5f5f5f5f610100898b0312156100d5575f5ffd5b883597506020890135965060408901359550606089013594506080890135935060a089013567ffffffffffffffff81111561010e575f5ffd5b8901601f81018b1361011e575f5ffd5b803567ffffffffffffffff8111156101385761013861008e565b8060051b604051601f19603f830116810181811067ffffffffffffffff821117156101655761016561008e565b60405291825260208184018101929081018e841115610182575f5ffd5b6020850194505b838510156101a85761019a856100a2565b815260209485019401610189565b50955050505060c089013591506101c160e08a016100a2565b90509295985092959890939650565b5f60a0820187835286602084015260a0604084015280865180835260c0850191506020880192505f5b818110156102205783516001600160a01b03168352602093840193909201916001016101f9565b5050606084019590955250506001600160a01b0391909116608090910152939250505056fea2646970667358221220ebfcdf26441c60454de8d2aa6420871f15970228f3a1fa2e65e23d478f21eac064736f6c634300081b0033";
        
        let controller_factory = ContractFactory::new(
            serde_json::from_str(CONTROLLER_ABI).expect("Invalid CONTROLLER_ABI"),
            CONTROLLER_BYTECODE.parse::<Bytes>().expect("Invalid CONTROLLER_BYTECODE"),
            client.clone(),
        );
        
        let controller_contract_deployed = controller_factory.deploy(())?.send().await?;
        let controller_address = controller_contract_deployed.address();
        println!("Controller contract deployed at: {}", controller_address);
        
        Ok(controller_address)
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
        );
        
        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::new(id_address)),
        ));
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        
        let listener = PreGroupingListener::<G2Curve>::new(
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
        
        let controller_address = deploy_mock_controller(
            client.clone(), 
        ).await.map_err(|e| anyhow!("Failed to deploy mock controller contract: {}", e))?;
        
        println!("Controller contract deployed at: {}", controller_address);
        
        let adapter_address = Address::random();
        let node_registry_address = Address::random();
        
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
        
        let listener = PreGroupingListener::<G2Curve>::new(
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
        let mock_controller = MockController::new(controller_address, client.clone());
        
        let members = vec![id_address, Address::random(), Address::random()];
        let global_epoch = U256::from(1);
        let group_index = U256::from(2);
        let group_epoch = U256::from(1);
        let size = U256::from(3);
        let threshold = U256::from(2);
        let assignment_block_height = U256::from(100);
        let coordinator_address = Address::random();
        
        mock_controller.emit_dkg_task_event(
            global_epoch,
            group_index,
            group_epoch,
            size,
            threshold,
            members.clone(),
            assignment_block_height,
            coordinator_address
        )
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send transaction: {}", e))?
        .await
        .map_err(|e| anyhow!("Transaction failed: {}", e))?;
        
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
        
        let controller_address = deploy_mock_controller(
            client.clone(), 
        ).await.map_err(|e| anyhow!("Failed to deploy mock controller contract: {}", e))?;
        
        let adapter_address = Address::random();
        let node_registry_address = Address::random();
        
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
        
        let listener = PreGroupingListener::<G2Curve>::new(
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
        
        let mock_controller = MockController::new(controller_address, client.clone());
        
        let members = vec![Address::random(), Address::random(), Address::random()];
        let global_epoch = U256::from(1);
        let group_index = U256::from(2);
        let group_epoch = U256::from(1);
        let size = U256::from(3);
        let threshold = U256::from(2);
        let assignment_block_height = U256::from(100);
        let coordinator_address = Address::random();
        
        mock_controller.emit_dkg_task_event(
            global_epoch,
            group_index,
            group_epoch,
            size,
            threshold,
            members.clone(),
            assignment_block_height,
            coordinator_address
        )
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send transaction: {}", e))?
        .await
        .map_err(|e| anyhow!("Transaction failed: {}", e))?;
        
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
        
        let controller_address = deploy_mock_controller(
            client.clone(), 
        ).await.map_err(|e| anyhow!("Failed to deploy mock controller contract: {}", e))?;
        
        let adapter_address = Address::random();
        let node_registry_address = Address::random();
        
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
        
        let listener = PreGroupingListener::<G2Curve>::new(
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
        
        let mock_controller = MockController::new(controller_address, client.clone());
        
        let members = vec![id_address, Address::random(), Address::random()];
        let global_epoch = U256::from(1);
        let group_index = U256::from(2);
        let group_epoch = U256::from(1);
        let size = U256::from(3);
        let threshold = U256::from(2);
        let assignment_block_height = U256::from(100);
        let coordinator_address = Address::random();
        
        mock_controller.emit_dkg_task_event(
            global_epoch,
            group_index,
            group_epoch,
            size,
            threshold,
            members.clone(),
            assignment_block_height,
            coordinator_address
        )
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send transaction: {}", e))?
        .await
        .map_err(|e| anyhow!("Transaction failed: {}", e))?;
        
        let timeout_result = timeout(Duration::from_millis(500), event_receiver.recv()).await;
        assert!(timeout_result.is_err(), "Unexpectedly received an event for same task");
        
        Ok(())
    }
}