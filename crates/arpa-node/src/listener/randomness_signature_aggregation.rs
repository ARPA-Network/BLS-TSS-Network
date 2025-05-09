use super::Listener;
use crate::{
    error::NodeResult,
    event::ready_to_fulfill_randomness_task::ReadyToFulfillRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_dal::cache::RandomnessResultCache;
use arpa_dal::{BlockInfoHandler, GroupInfoHandler, SignatureResultCacheHandler};
use async_trait::async_trait;
use ethers::types::Address;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct RandomnessSignatureAggregationListener<PC: Curve> {
    chain_id: usize,
    id_address: Address,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    randomness_signature_cache:
        Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> std::fmt::Display for RandomnessSignatureAggregationListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RandomnessSignatureAggregationListener")
    }
}

impl<PC: Curve> RandomnessSignatureAggregationListener<PC> {
    pub fn new(
        chain_id: usize,
        id_address: Address,
        block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        randomness_signature_cache: Arc<
            RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>,
        >,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        RandomnessSignatureAggregationListener {
            chain_id,
            id_address,
            block_cache,
            group_cache,
            randomness_signature_cache,
            eq,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> EventPublisher<ReadyToFulfillRandomnessTask>
    for RandomnessSignatureAggregationListener<PC>
{
    async fn publish(&self, event: ReadyToFulfillRandomnessTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for RandomnessSignatureAggregationListener<PC> {
    async fn listen(&self) -> NodeResult<()> {
        let is_committer = self.group_cache.read().await.is_committer(self.id_address);

        if let Ok(true) = is_committer {
            let current_block_height = self.block_cache.read().await.get_block_height();

            let ready_signatures = self
                .randomness_signature_cache
                .write()
                .await
                .get_ready_to_commit_signatures(current_block_height)
                .await?;

            if !ready_signatures.is_empty() {
                self.publish(ReadyToFulfillRandomnessTask {
                    chain_id: self.chain_id,
                    tasks: ready_signatures,
                })
                .await;
            }
        }

        Ok(())
    }

    async fn handle_interruption(&self) -> NodeResult<()> {
        Ok(())
    }

    async fn chain_id(&self) -> usize {
        self.chain_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{types::GeneralContext, Context};    
    use crate::context::chain::types::GeneralMainChain;
    use crate::context::ContextFetcher;
    use crate::context::chain::Chain;
    use crate::event::Event;
    use threshold_bls::schemes::bn254::G2Curve;
    use threshold_bls::schemes::bn254::G2Scheme;    
    use crate::queue::EventSubscriber;
    use crate::event::types::Topic;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use arpa_core::{
        ComponentTaskType, Config, GeneralMainChainIdentity, ListenerType, RandomnessTask,
        PLACEHOLDER_ADDRESS, DKGStatus,RandomnessRequestType
    };
    use arpa_dal::{
        cache::{
            InMemoryBLSTasksQueue, InMemoryGroupInfoCache, InMemoryNodeInfoCache,
            InMemorySignatureResultCache, RandomnessResultCache
        },
        BLSTasksHandler, GroupInfoHandler, NodeInfoHandler, SignatureResultCacheHandler
    };
    use ethers::{
        providers::{Provider, Ws},
        types::Address,
        utils::Anvil,
    };
    use std::time::Duration;
    use tokio::time::timeout;
    use crate::scheduler::TaskScheduler;

    
    type NodeContext<PC, S> = Arc<RwLock<GeneralContext<PC, S>>>;

    async fn mock_set_as_committer<PC: Curve + Send + Sync>(
        group_cache: &mut Box<dyn GroupInfoHandler<PC>>,
        address: Address,
    ) {
        let task = arpa_core::DKGTask {
            group_index: 1,
            epoch: 1,
            size: 3,
            threshold: 2,
            assignment_block_height: 100,
            members: vec![address, Address::random(), Address::random()],
            coordinator_address: Address::random()
        };
        
        group_cache.save_task_info(0, task).await.unwrap();
        
        group_cache.update_dkg_status(1, 1, DKGStatus::CommitSuccess).await.unwrap();
        
        group_cache.save_committers(1, 1, vec![address]).await.unwrap();
    }

    async fn mock_set_as_non_committer<PC: Curve + Send + Sync>(
        group_cache: &mut Box<dyn GroupInfoHandler<PC>>,
        address: Address,
    ) {
        let task = arpa_core::DKGTask {
            group_index: 1,
            epoch: 1,
            size: 3,
            threshold: 2,
            assignment_block_height: 100,
            members: vec![address, Address::random(), Address::random()],
            coordinator_address: Address::random()
        };
        
        group_cache.save_task_info(0, task).await.unwrap();
        
        group_cache.update_dkg_status(1, 1, DKGStatus::CommitSuccess).await.unwrap();
        
        group_cache.save_committers(1, 1, vec![Address::random()]).await.unwrap();
    }

    async fn mock_set_block_height(
        block_cache: &mut Box<dyn BlockInfoHandler>,
        height: u64,
    ) {
        block_cache.set_block_height(height as usize);
    }

    async fn create_test_randomness_task(
        request_id: u64,
        fulfillment_block_number: u64,
    ) -> RandomnessTask {
        RandomnessTask {
            request_id: request_id.to_be_bytes().to_vec(),
            subscription_id: 0,
            group_index: 1,
            request_type: RandomnessRequestType::Randomness,
            params: vec![1, 2, 3],
            requester: Address::random(),
            seed: ethers::types::U256::from(123),
            request_confirmations: 10,
            callback_gas_limit: 100000,
            callback_max_gas_price: ethers::types::U256::from(1000000000),
            assignment_block_height: fulfillment_block_number as usize,
        }
    }

    async fn mock_add_signature_result<PC: Curve + Send + Sync>(
        signature_cache: &mut Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>,
        task: RandomnessTask,
    ) {
        signature_cache.add(
            task.group_index as usize, 
            task.clone(), 
            vec![1, 2, 3, 4],
            2
        ).await.unwrap();
        
        let addresses = [Address::random(), Address::random()];
        for (i, addr) in addresses.iter().enumerate() {
            let partial_sig_data = vec![i as u8, 42, 255];
            
            signature_cache.add_partial_signature(
                task.request_id.clone(),
                *addr,
                i,
                partial_sig_data,
            ).await.unwrap();
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
                if let Some(ready_event) = payload.as_any().downcast_ref::<ReadyToFulfillRandomnessTask>() {
                    let cloned_event = ReadyToFulfillRandomnessTask {
                        chain_id: ready_event.chain_id,
                        tasks: ready_event.tasks.clone(),
                    };
                    let boxed = Box::new(cloned_event) as Box<dyn std::any::Any + Send>;
                    self.sender.send(boxed).await.map_err(|e| {
                        let err: crate::error::NodeError = anyhow::anyhow!("Failed to send event: {}", e).into();
                        err
                    })?;
                }
                Ok(())
            }
            
            async fn subscribe(self) {
            }
        }
        
        impl DebuggableSubscriber for TestSubscriber {}
        
        let subscriber = TestSubscriber {
            name: subscriber_name.to_string(),
            sender,
        };
        let dummy_event = ReadyToFulfillRandomnessTask { chain_id: 0, tasks: vec![] };
        let topic = dummy_event.topic();
        
        eq.subscribe(topic, Box::new(subscriber));
        
        receiver
    }
    
    async fn build_context() -> NodeContext<G2Curve, G2Scheme> {
        let config = Config::default();

        let fake_wallet = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
            .parse()
            .unwrap();

        let node_cache: Arc<RwLock<Box<dyn NodeInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryNodeInfoCache::<G2Curve>::new(Address::random())),
        ));

        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::new(PLACEHOLDER_ADDRESS)),
        ));

        let randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>> =
            Arc::new(RwLock::new(Box::new(InMemoryBLSTasksQueue::new())));

        let randomness_result_cache: Arc<
            RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>,
        > = Arc::new(RwLock::new(Box::new(InMemorySignatureResultCache::<
            RandomnessResultCache,
        >::new())));

        let avnil = Anvil::new().spawn();

        let provider = Arc::new(Provider::<Ws>::connect(avnil.ws_endpoint()).await.unwrap());

        let contract_transaction_retry_descriptor = config
            .get_time_limits()
            .contract_transaction_retry_descriptor;

        let contract_view_retry_descriptor =
            config.get_time_limits().contract_view_retry_descriptor;

        let main_chain_identity = GeneralMainChainIdentity::new(
            config.get_main_chain_id(),
            fake_wallet,
            provider,
            avnil.ws_endpoint(),
            Address::random(),
            Address::random(),
            Address::random(),
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
        );

        let main_chain = GeneralMainChain::<G2Curve, G2Scheme>::new(
            "main chain".to_string(),
            false,
            main_chain_identity.clone(),
            node_cache.clone(),
            group_cache.clone(),
            randomness_tasks_cache,
            randomness_result_cache,
            *config.get_time_limits(),
            config.get_listeners().clone(),
        );

        let context = GeneralContext::new(main_chain, config);

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(ComponentTaskType::Listener(0, ListenerType::NewRandomnessTask), async {
            })
            .unwrap();

        Arc::new(RwLock::new(context))
    }


    #[tokio::test]
    async fn test_randomness_signature_aggregation_listener() -> NodeResult<()> {
        let context = build_context().await;
        let context_lock = context.read().await;
        
        let chain_id = context_lock
            .get_main_chain()
            .get_chain_identity()
            .read()
            .await
            .get_chain_id();
        
        let id_address = context_lock.get_main_chain().get_chain_identity().read().await.get_id_address();
        let block_cache = context_lock.get_main_chain().get_block_cache();
        let group_cache = context_lock.get_main_chain().get_group_cache();
        let randomness_signature_cache = context_lock.get_main_chain().get_randomness_result_cache();
        let event_queue = context_lock.get_event_queue();

        let listener = RandomnessSignatureAggregationListener::<G2Curve>::new(
            chain_id,
            id_address,
            block_cache.clone(),
            group_cache.clone(),
            randomness_signature_cache.clone(),
            event_queue.clone(),
        );

        {
            let mut group_cache_write = group_cache.write().await;
            mock_set_as_committer(&mut group_cache_write, id_address).await;
        }

        let current_block_height = 1000u64;
        {
            let mut block_cache_write = block_cache.write().await;
            mock_set_block_height(&mut block_cache_write, current_block_height).await;
        }

        {
            let mut signature_cache_write = randomness_signature_cache.write().await;
            
            let ready_task = create_test_randomness_task(1, current_block_height - 10).await;
            let not_ready_task = create_test_randomness_task(2, current_block_height + 100).await;
            
            mock_add_signature_result::<G2Curve>(&mut signature_cache_write, ready_task).await;
            mock_add_signature_result::<G2Curve>(&mut signature_cache_write, not_ready_task).await;
        }

        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber").await
        };

        listener.listen().await?;

        let received_event = timeout(Duration::from_secs(1), event_receiver.recv()).await
            .map_err(|_| anyhow::anyhow!("Timeout: No event received"))?
            .ok_or_else(|| anyhow::anyhow!("Error: Event channel closed"))?;
        
        if let Some(ready_event) = received_event.downcast_ref::<ReadyToFulfillRandomnessTask>() {
            assert_eq!(ready_event.chain_id, chain_id);
            assert_eq!(ready_event.tasks.len(), 1);
        } else {
            return Err(anyhow::anyhow!("Received unexpected event type").into());
        }

        {
            let mut group_cache_write = group_cache.write().await;
            mock_set_as_non_committer(&mut group_cache_write, id_address).await;
        }

        let new_event_queue = Arc::new(RwLock::new(EventQueue::new()));
        let listener_for_non_committer = RandomnessSignatureAggregationListener::<G2Curve>::new(
            chain_id,
            id_address,
            block_cache.clone(),
            group_cache.clone(),
            randomness_signature_cache.clone(),
            new_event_queue.clone(),
        );

        let mut event_receiver = {
            let mut eq_write = new_event_queue.write().await;
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber").await
        };

        listener_for_non_committer.listen().await?;

        let timeout_result = timeout(Duration::from_millis(100), event_receiver.recv()).await;
        assert!(timeout_result.is_err(), "Unexpectedly received an event when node is not a committer");

        Ok(())
    }
}