use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::dkg_post_process::DKGPostProcess,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::controller::ControllerViews;
use arpa_core::{DKGStatus, ListenerDescriptor};
use arpa_dal::{BlockInfoHandler, GroupInfoHandler};
use async_trait::async_trait;
use log::info;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PostGroupingListener<PC: Curve> {
    listener_descriptor: ListenerDescriptor,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
    dkg_timeout_duration: usize,
}

impl<PC: Curve> std::fmt::Display for PostGroupingListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PostGroupingListener")
    }
}

impl<PC: Curve> PostGroupingListener<PC> {
    pub fn new(
        listener_descriptor: ListenerDescriptor,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
        dkg_timeout_duration: usize,
    ) -> Self {
        PostGroupingListener {
            listener_descriptor,
            chain_identity,
            block_cache,
            group_cache,
            eq,
            pc: PhantomData,
            dkg_timeout_duration,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send + 'static> EventPublisher<DKGPostProcess<PC>>
    for PostGroupingListener<PC>
{
    async fn publish(&self, event: DKGPostProcess<PC>) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send + 'static> Listener for PostGroupingListener<PC> {
    async fn listen(&self) -> NodeResult<()> {
        let dkg_status = self.group_cache.read().await.get_dkg_status();

        if let Ok(dkg_status) = dkg_status {
            match dkg_status {
                DKGStatus::None => {}
                DKGStatus::InPhase | DKGStatus::CommitSuccess | DKGStatus::WaitForPostProcess => {
                    let dkg_start_block_height =
                        self.group_cache.read().await.get_dkg_start_block_height()?;

                    let block_height = self.block_cache.read().await.get_block_height();

                    let dkg_timeout_block_height =
                        dkg_start_block_height + self.dkg_timeout_duration;

                    info!("checking post process... dkg_start_block_height: {}, current_block_height: {}, timeuout_dkg_block_height: {}",
                    dkg_start_block_height,block_height,dkg_timeout_block_height);

                    if block_height > dkg_timeout_block_height {
                        let group_index = self.group_cache.read().await.get_index().unwrap_or(0);

                        let group_epoch = self.group_cache.read().await.get_epoch().unwrap_or(0);

                        let client = self.chain_identity.read().await.build_controller_client();

                        if let Ok(group) = client.get_group(group_index).await {
                            self.publish(DKGPostProcess {
                                group_index,
                                group_epoch,
                                group,
                            })
                            .await;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_interruption(&self) -> NodeResult<()> {
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
    use crate::event::Event;
    use crate::queue::EventSubscriber;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use crate::test_contracts::mockcontroller::deploy_with_args_and_get_mock_controller;

    use ethers::middleware::SignerMiddleware;
    use ethers::providers::{Http, Provider, Ws};
    use ethers::signers::{LocalWallet, Signer};
    use ethers::types::{Address, U256};
    use ethers::utils::Anvil;

    use threshold_bls::schemes::bn254::G2Curve;

    use arpa_core::{
        Config, DKGStatus, FixedIntervalRetryDescriptor, GeneralMainChainIdentity, Group,
        ListenerType,
    };
    use arpa_dal::{
        cache::{InMemoryBlockInfoCache, InMemoryGroupInfoCache},
        GroupInfoHandler,
    };

    use anyhow::anyhow;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::timeout;

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
                if let Some(post_process_event) =
                    payload.as_any().downcast_ref::<DKGPostProcess<G2Curve>>()
                {
                    let cloned_event = DKGPostProcess {
                        group_index: post_process_event.group_index,
                        group_epoch: post_process_event.group_epoch,
                        group: post_process_event.group.clone(),
                    };
                    let boxed = Box::new(cloned_event) as Box<dyn std::any::Any + Send>;
                    self.sender.send(boxed).await.map_err(|e| {
                        let err: crate::error::NodeError =
                            anyhow!("Failed to send event: {}", e).into();
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
        let dummy_event = DKGPostProcess {
            group_index: 1,
            group_epoch: 1,
            group: Group::<G2Curve>::new(),
        };

        let topic = dummy_event.topic();

        eq.subscribe(topic, Box::new(subscriber));

        receiver
    }

    async fn create_test_caches(
        chain_id: u64,
        id_address: Address,
    ) -> (
        Arc<RwLock<Box<dyn BlockInfoHandler>>>,
        Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>>,
    ) {
        let block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>> = Arc::new(RwLock::new(Box::new(
            InMemoryBlockInfoCache::new(chain_id, 15),
        )));

        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::new(id_address)),
        ));

        (block_cache, group_cache)
    }

    async fn setup_dkg_task(
        group_cache: &Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>>,
        id_address: Address,
        dkg_status: DKGStatus,
    ) -> NodeResult<()> {
        let mut group_cache_write = group_cache.write().await;
        let dkg_task = arpa_core::DKGTask {
            group_index: 1,
            epoch: 1,
            size: 3,
            threshold: 2,
            assignment_block_height: 50,
            members: vec![id_address, random_address(), random_address()],
            coordinator_address: random_address(),
        };

        group_cache_write.save_task_info(0, dkg_task).await?;
        group_cache_write
            .update_dkg_status(1, 1, dkg_status)
            .await?;
        Ok(())
    }

    fn create_listener_descriptor(chain_id: u64) -> ListenerDescriptor {
        ListenerDescriptor {
            chain_id,
            l_type: ListenerType::PostGrouping,
            interval_millis: 1000,
            use_jitter: true,
            reset_descriptor: FixedIntervalRetryDescriptor {
                interval_millis: 5000,
                max_attempts: 3,
                use_jitter: true,
            },
        }
    }

    async fn create_mock_chain_identity<PC: Curve + Sync + Send + 'static>(
        controller_address: Option<Address>,
    ) -> NodeResult<Arc<RwLock<ChainIdentityHandlerType<PC>>>> {
        let anvil = Anvil::new().spawn();

        let provider = Arc::new(
            Provider::<Ws>::connect(anvil.ws_endpoint())
                .await
                .map_err(|e| anyhow!("Failed to connect to WS provider: {}", e))?,
        );

        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let chain_id = anvil.chain_id() as usize;

        let config = Config::default();

        let controller = controller_address.unwrap_or_else(Address::random);

        let chain_identity = GeneralMainChainIdentity::new(
            chain_id,
            wallet.clone(),
            provider,
            anvil.ws_endpoint(),
            controller,
            random_address(),
            random_address(),
            config
                .get_time_limits()
                .contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );

        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<PC>>> = Arc::new(RwLock::new(
            Box::new(chain_identity) as ChainIdentityHandlerType<PC>,
        ));

        Ok(chain_identity_arc)
    }

    async fn create_test_listener_with_chain_identity(
        chain_id: u64,
        id_address: Address,
        block_height: usize,
        dkg_status: DKGStatus,
        dkg_timeout_duration: usize,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>>,
    ) -> NodeResult<(
        PostGroupingListener<G2Curve>,
        tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>>,
    )> {
        let (block_cache, group_cache) = create_test_caches(chain_id, id_address).await;
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));

        setup_dkg_task(&group_cache, id_address, dkg_status).await?;

        {
            let mut block_cache_write = block_cache.write().await;
            block_cache_write.set_block_height(block_height);
        }

        let listener_descriptor = create_listener_descriptor(chain_id);

        let listener = PostGroupingListener::<G2Curve>::new(
            listener_descriptor,
            chain_identity,
            block_cache,
            group_cache,
            event_queue.clone(),
            dkg_timeout_duration,
        );

        let event_receiver = {
            let mut eq_write = event_queue.write().await;
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber").await
        };

        Ok((listener, event_receiver))
    }

    async fn create_test_listener(
        chain_id: u64,
        id_address: Address,
        block_height: usize,
        dkg_status: DKGStatus,
        dkg_timeout_duration: usize,
        controller_address: Option<Address>,
    ) -> NodeResult<(
        PostGroupingListener<G2Curve>,
        tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>>,
    )> {
        let chain_identity = create_mock_chain_identity::<G2Curve>(controller_address).await?;

        create_test_listener_with_chain_identity(
            chain_id,
            id_address,
            block_height,
            dkg_status,
            dkg_timeout_duration,
            chain_identity,
        )
        .await
    }

    async fn setup_contract_group(
        controller: &crate::test_contracts::mockcontroller::MockController<
            SignerMiddleware<Provider<Http>, LocalWallet>,
        >,
        id_address: Address,
    ) -> NodeResult<()> {
        let group_index = 1usize;
        let epoch = 1usize;
        let size = 3usize;
        let threshold = 2usize;
        let empty_public_key = [U256::zero(), U256::zero(), U256::zero(), U256::zero()];
        let member_addresses = vec![id_address, random_address(), random_address()];

        let tx_request = controller.set_group(
            group_index.into(),
            epoch.into(),
            size.into(),
            threshold.into(),
            false,
            empty_public_key,
            member_addresses.clone(),
        );

        let pending_tx = tx_request
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send setGroup transaction: {}", e))?;

        pending_tx
            .await
            .map_err(|e| anyhow!("setGroup transaction failed: {}", e))?;

        println!("Group setup complete");
        Ok(())
    }

    #[tokio::test]
    async fn test_post_grouping_listener_timeout() -> NodeResult<()> {
        let chain_id = 1;
        let id_address = random_address();
        let dkg_timeout_duration = 100;

        let anvil = Anvil::new().chain_id(chain_id as u64).spawn();

        let http_provider =
            Provider::<Http>::try_from(anvil.endpoint()).expect("Failed to create HTTP provider");
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let wallet = wallet.with_chain_id(chain_id as u64);
        let client = Arc::new(SignerMiddleware::new(http_provider.clone(), wallet.clone()));

        let node_registry_address = random_address();
        let controller =
            deploy_with_args_and_get_mock_controller(client.clone(), node_registry_address)
                .await
                .map_err(|e| anyhow!("Failed to deploy mock controller: {}", e))?;

        let controller_address = controller.address();
        println!("MockController deployed at: {}", controller_address);

        setup_contract_group(&controller, id_address).await?;

        let ws_provider = Arc::new(
            Provider::<Ws>::connect(anvil.ws_endpoint())
                .await
                .map_err(|e| anyhow!("Failed to connect to WS provider: {}", e))?,
        );

        let config = Config::default();
        let chain_identity = GeneralMainChainIdentity::new(
            chain_id,
            wallet.clone(),
            ws_provider,
            anvil.ws_endpoint(),
            controller_address,
            random_address(),
            random_address(),
            config
                .get_time_limits()
                .contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );

        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = Arc::new(
            RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>),
        );

        let (listener, mut event_receiver) = create_test_listener_with_chain_identity(
            chain_id,
            id_address,
            200, // block_height that triggers timeout (50 + 100 = 150 < 200)
            DKGStatus::InPhase,
            dkg_timeout_duration,
            chain_identity_arc,
        )
        .await?;

        listener.listen().await?;

        let received_event = timeout(Duration::from_secs(5), event_receiver.recv())
            .await
            .map_err(|_| anyhow!("Timeout: No event received"))?
            .ok_or_else(|| anyhow!("Error: Event channel closed"))?;

        if let Some(post_process_event) = received_event.downcast_ref::<DKGPostProcess<G2Curve>>() {
            assert_eq!(post_process_event.group_index, 1);
            assert_eq!(post_process_event.group_epoch, 1);
        } else {
            return Err(anyhow!("Received unexpected event type").into());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_post_grouping_listener_no_timeout() -> NodeResult<()> {
        let chain_id = 1;
        let id_address = random_address();
        let dkg_timeout_duration = 100;

        let (listener, mut event_receiver) = create_test_listener(
            chain_id,
            id_address,
            100, // block_height that doesn't trigger timeout (50 + 100 = 150 > 100)
            DKGStatus::InPhase,
            dkg_timeout_duration,
            None,
        )
        .await?;

        listener.listen().await?;

        let timeout_result = timeout(Duration::from_millis(100), event_receiver.recv()).await;
        assert!(
            timeout_result.is_err(),
            "Unexpectedly received an event when DKG is not timed out"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_post_grouping_listener_dkg_none() -> NodeResult<()> {
        let chain_id = 1;
        let id_address = random_address();
        let dkg_timeout_duration = 100;

        let (listener, mut event_receiver) = create_test_listener(
            chain_id,
            id_address,
            200, // Even with timeout block height, shouldn't trigger due to status
            DKGStatus::None,
            dkg_timeout_duration,
            None,
        )
        .await?;

        listener.listen().await?;

        let timeout_result = timeout(Duration::from_millis(100), event_receiver.recv()).await;
        assert!(
            timeout_result.is_err(),
            "Unexpectedly received an event when DKG status is None"
        );

        Ok(())
    }
}
