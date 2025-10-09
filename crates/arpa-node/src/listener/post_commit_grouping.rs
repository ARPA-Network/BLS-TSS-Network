use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::dkg_success::DKGSuccess,
    queue::{event_queue::EventQueue, EventPublisher},
};
use alloy::providers::Provider;
use arpa_contract_client::controller::ControllerViews;
use arpa_core::{DKGStatus, ListenerDescriptor};
use arpa_dal::GroupInfoHandler;
use async_trait::async_trait;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PostCommitGroupingListener<PC: Curve> {
    listener_descriptor: ListenerDescriptor,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> std::fmt::Display for PostCommitGroupingListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PostCommitGroupingListener")
    }
}

impl<PC: Curve> PostCommitGroupingListener<PC> {
    pub fn new(
        listener_descriptor: ListenerDescriptor,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        PostCommitGroupingListener {
            listener_descriptor,
            chain_identity,
            group_cache,
            eq,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + Send + Sync + 'static> EventPublisher<DKGSuccess<PC>>
    for PostCommitGroupingListener<PC>
{
    async fn publish(&self, event: DKGSuccess<PC>) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send + 'static> Listener for PostCommitGroupingListener<PC> {
    async fn listen(&self) -> NodeResult<()> {
        let dkg_status = self.group_cache.read().await.get_dkg_status();

        if let Ok(DKGStatus::CommitSuccess) = dkg_status {
            let chain_id = self.listener_descriptor.chain_id;

            let group_index = self.group_cache.read().await.get_index()?;

            let client = self.chain_identity.read().await.build_controller_client();

            let id_address = self.chain_identity.read().await.get_id_address();

            if let Ok(group) = client.get_group(group_index).await {
                if group.state {
                    self.publish(DKGSuccess {
                        chain_id,
                        id_address,
                        group,
                    })
                    .await;
                }
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
    use crate::event::types::Topic;
    use crate::event::Event;
    use crate::queue::EventSubscriber;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use crate::test_contracts::mockcontroller::{
        deploy_with_args_and_get_mock_controller, MockController,
    };

    use ethers::middleware::SignerMiddleware;
    use ethers::providers::{Http, Provider, Ws};
    use ethers::signers::{LocalWallet, Signer};
    use ethers::types::{Address, U256};
    use ethers::utils::{Anvil, AnvilInstance};

    use threshold_bls::schemes::bn254::G2Curve;

    use arpa_core::{
        Config, DKGStatus, FixedIntervalRetryDescriptor, GeneralMainChainIdentity, Group,
        ListenerType, Member,
    };
    use arpa_dal::{cache::InMemoryGroupInfoCache, GroupInfoHandler};

    use anyhow::anyhow;
    use std::time::Duration;
    use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};
    use tokio::time::timeout;

    async fn mock_subscribe_to_events<PC: Curve + Send + Sync + 'static>(
        eq: &mut EventQueue,
        subscriber_name: &str,
    ) -> tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>> {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);

        struct TestSubscriber<PC: Curve + Send + Sync + 'static> {
            name: String,
            sender: tokio::sync::mpsc::Sender<Box<dyn std::any::Any + Send>>,
            pc: PhantomData<PC>,
        }

        impl<PC: Curve + Send + Sync + 'static> std::fmt::Debug for TestSubscriber<PC> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "TestSubscriber({})", self.name)
            }
        }

        #[async_trait]
        impl<PC: Curve + Send + Sync + 'static> Subscriber for TestSubscriber<PC> {
            async fn notify(&self, _topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()> {
                if let Some(success_event) = payload.as_any().downcast_ref::<DKGSuccess<PC>>() {
                    let cloned_event = DKGSuccess {
                        chain_id: success_event.chain_id,
                        id_address: success_event.id_address,
                        group: success_event.group.clone(),
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

        impl<PC: Curve + Send + Sync + 'static> DebuggableSubscriber for TestSubscriber<PC> {}

        let subscriber: TestSubscriber<PC> = TestSubscriber {
            name: subscriber_name.to_string(),
            sender,
            pc: PhantomData,
        };

        let dummy_id_address = random_address();
        let dummy_group = Group::<PC> {
            index: 1,
            epoch: 1,
            size: 3,
            threshold: 2,
            state: true,
            public_key: None,
            members: BTreeMap::new(),
            committers: vec![random_address()],
            c: PhantomData,
        };

        let dummy_event = DKGSuccess::<PC> {
            chain_id: 0,
            id_address: dummy_id_address,
            group: dummy_group,
        };

        let topic = dummy_event.topic();

        eq.subscribe(topic, Box::new(subscriber));

        receiver
    }

    async fn setup_test_environment() -> NodeResult<(
        AnvilInstance,
        Address,
        usize,
        MockController<SignerMiddleware<Provider<Http>, LocalWallet>>,
    )> {
        let anvil = Anvil::new().spawn();

        let http_provider = Provider::<Http>::try_from(anvil.endpoint())
            .map_err(|e| anyhow!("Failed to create HTTP provider: {}", e))?;

        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let id_address = wallet.address();
        let chain_id = anvil.chain_id() as usize;

        let client = Arc::new(SignerMiddleware::new(
            http_provider,
            wallet.clone().with_chain_id(anvil.chain_id()),
        ));

        let node_registry_address = random_address();

        let controller = deploy_with_args_and_get_mock_controller(client, node_registry_address)
            .await
            .map_err(|e| anyhow!("Failed to deploy mock controller: {}", e))?;

        Ok((anvil, id_address, chain_id, controller))
    }

    fn setup_chain_identity(
        chain_id: u64,
        wallet: LocalWallet,
        ws_provider: Arc<Provider<Ws>>,
        ws_endpoint: String,
        controller_address: Address,
    ) -> GeneralMainChainIdentity {
        let config = Config::default();

        GeneralMainChainIdentity::new(
            chain_id,
            wallet.clone(),
            ws_provider.clone(),
            ws_endpoint,
            controller_address,
            random_address(),
            random_address(),
            config
                .get_time_limits()
                .contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        )
    }

    async fn setup_contract_group(
        controller: &MockController<SignerMiddleware<Provider<Http>, LocalWallet>>,
        group_index: usize,
        epoch: usize,
        size: usize,
        threshold: usize,
        group_state: bool,
        member_addresses: Vec<Address>,
    ) -> NodeResult<()> {
        let empty_public_key: [U256; 4] = [U256::zero(), U256::zero(), U256::zero(), U256::zero()];

        let contract_call = controller.set_group(
            group_index.into(),
            epoch.into(),
            size.into(),
            threshold.into(),
            group_state,
            empty_public_key,
            member_addresses.clone(),
        );

        let pending_tx = contract_call
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        let receipt = pending_tx
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?;

        println!("Group set in block: {:?}", receipt);
        Ok(())
    }

    async fn setup_committers(
        controller: &MockController<SignerMiddleware<Provider<Http>, LocalWallet>>,
        group_index: usize,
        committer_addresses: Vec<Address>,
    ) -> NodeResult<()> {
        let tx = controller.set_committers(group_index.into(), committer_addresses.clone());

        let receipt = tx
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?;

        println!(
            "Committers set in block: {}",
            receipt.unwrap().block_number.unwrap()
        );
        Ok(())
    }

    async fn setup_group_cache(
        id_address: Address,
        chain_id: u64,
        group_index: usize,
        epoch: usize,
        size: usize,
        threshold: usize,
        member_addresses: Vec<Address>,
        dkg_status: DKGStatus,
    ) -> NodeResult<Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>>> {
        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::new(id_address)),
        ));

        {
            let mut group_cache_write = group_cache.write().await;
            let dkg_task = arpa_core::DKGTask {
                group_index,
                epoch,
                size,
                threshold,
                assignment_block_height: 100,
                members: member_addresses.clone(),
                coordinator_address: random_address(),
            };

            group_cache_write.save_task_info(chain_id, dkg_task).await?;
            group_cache_write
                .update_dkg_status(group_index, epoch, dkg_status)
                .await?;
        }

        Ok(group_cache)
    }

    fn create_listener(
        chain_id: u64,
        chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>>,
        event_queue: Arc<RwLock<EventQueue>>,
    ) -> PostCommitGroupingListener<G2Curve> {
        let listener_descriptor = ListenerDescriptor {
            chain_id,
            l_type: ListenerType::PostCommitGrouping,
            interval_millis: 1000,
            use_jitter: true,
            reset_descriptor: FixedIntervalRetryDescriptor {
                interval_millis: 5000,
                max_attempts: 3,
                use_jitter: true,
            },
        };

        PostCommitGroupingListener::<G2Curve>::new(
            listener_descriptor,
            chain_identity_arc,
            group_cache,
            event_queue,
        )
    }

    fn assert_dkg_success_event(
        event: &DKGSuccess<G2Curve>,
        expected_chain_id: u64,
        expected_id_address: Address,
        expected_group_index: usize,
        expected_epoch: usize,
        expected_size: usize,
        expected_threshold: usize,
        expected_state: bool,
    ) {
        assert_eq!(event.chain_id, expected_chain_id);
        assert_eq!(event.id_address, expected_id_address);
        assert_eq!(event.group.index, expected_group_index);
        assert_eq!(event.group.epoch, expected_epoch);
        assert_eq!(event.group.size, expected_size);
        assert_eq!(event.group.threshold, expected_threshold);
        assert_eq!(event.group.state, expected_state);
    }

    async fn wait_and_verify_event(
        event_receiver: &mut tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>>,
        expected_chain_id: u64,
        expected_id_address: Address,
        expected_group_index: usize,
        expected_epoch: usize,
        expected_size: usize,
        expected_threshold: usize,
        expected_state: bool,
    ) -> NodeResult<()> {
        let received_event = timeout(Duration::from_secs(5), event_receiver.recv())
            .await
            .map_err(|_| anyhow!("Timeout: No event received"))?
            .ok_or_else(|| anyhow!("Error: Event channel closed"))?;

        if let Some(success_event) = received_event.downcast_ref::<DKGSuccess<G2Curve>>() {
            assert_dkg_success_event(
                success_event,
                expected_chain_id,
                expected_id_address,
                expected_group_index,
                expected_epoch,
                expected_size,
                expected_threshold,
                expected_state,
            );
        } else {
            return Err(anyhow!("Received unexpected event type").into());
        }

        Ok(())
    }

    async fn assert_no_event_received(
        event_receiver: &mut tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>>,
        timeout_millis: u64,
        error_message: &str,
    ) {
        let timeout_result =
            timeout(Duration::from_millis(timeout_millis), event_receiver.recv()).await;
        assert!(timeout_result.is_err(), "{}", error_message);
    }

    #[tokio::test]
    async fn test_post_commit_grouping_listener() -> NodeResult<()> {
        let (anvil, id_address, chain_id, controller) = setup_test_environment().await?;
        let controller_address = controller.address();
        println!("MockController deployed at: {}", controller_address);

        let ws_provider = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await?);
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let chain_identity = setup_chain_identity(
            chain_id,
            wallet.clone(),
            ws_provider.clone(),
            anvil.ws_endpoint(),
            controller_address,
        );

        let group_index = 1;
        let epoch = 1;
        let size = 3;
        let threshold = 2;
        let group_state = true;

        let mut member_addresses = Vec::new();
        member_addresses.push(id_address);
        member_addresses.push(random_address());
        member_addresses.push(random_address());

        let committer_addresses = vec![id_address];

        setup_contract_group(
            &controller,
            group_index,
            epoch,
            size,
            threshold,
            group_state,
            member_addresses.clone(),
        )
        .await?;
        setup_committers(&controller, group_index, committer_addresses.clone()).await?;

        let group_cache = setup_group_cache(
            id_address,
            chain_id,
            group_index,
            epoch,
            size,
            threshold,
            member_addresses.clone(),
            DKGStatus::CommitSuccess,
        )
        .await?;

        {
            let group_cache_read = group_cache.read().await;
            assert_eq!(group_cache_read.get_index().unwrap(), group_index);
            assert_eq!(group_cache_read.get_dkg_start_block_height().unwrap(), 100);
        }

        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = Arc::new(
            RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>),
        );

        let listener = create_listener(
            chain_id,
            chain_identity_arc.clone(),
            group_cache.clone(),
            event_queue.clone(),
        );

        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            mock_subscribe_to_events::<G2Curve>(&mut *eq_write, "test_subscriber").await
        };

        let mut members = BTreeMap::new();
        members.insert(
            id_address,
            Member {
                index: 0,
                dkg_index: Some(1),
                id_address,
                rpc_endpoint: None,
                partial_public_key: None,
            },
        );

        let test_group = Group::<G2Curve> {
            index: group_index,
            epoch,
            size,
            threshold,
            state: group_state,
            public_key: None,
            members,
            committers: committer_addresses.clone(),
            c: PhantomData,
        };

        listener
            .publish(DKGSuccess {
                chain_id,
                id_address,
                group: test_group.clone(),
            })
            .await;

        wait_and_verify_event(
            &mut event_receiver,
            chain_id,
            id_address,
            group_index,
            epoch,
            size,
            threshold,
            group_state,
        )
        .await?;

        let listen_result = listener.listen().await;
        assert!(listen_result.is_ok(), "Listen method should not fail");

        wait_and_verify_event(
            &mut event_receiver,
            chain_id,
            id_address,
            group_index,
            epoch,
            size,
            threshold,
            group_state,
        )
        .await?;

        {
            let mut group_cache_write = group_cache.write().await;
            group_cache_write
                .update_dkg_status(group_index, epoch, DKGStatus::None)
                .await?;
        }

        let new_event_queue = Arc::new(RwLock::new(EventQueue::new()));
        let listener_for_not_ready = create_listener(
            chain_id,
            chain_identity_arc.clone(),
            group_cache.clone(),
            new_event_queue.clone(),
        );

        let mut event_receiver_not_ready = {
            let mut eq_write = new_event_queue.write().await;
            mock_subscribe_to_events::<G2Curve>(&mut *eq_write, "test_subscriber").await
        };

        listener_for_not_ready.listen().await?;

        assert_no_event_received(
            &mut event_receiver_not_ready,
            500,
            "Unexpectedly received an event when DKG status is not CommitSuccess",
        )
        .await;

        {
            let mut group_cache_write = group_cache.write().await;
            group_cache_write
                .update_dkg_status(group_index, epoch, DKGStatus::CommitSuccess)
                .await?;
        }

        setup_contract_group(
            &controller,
            group_index,
            epoch,
            size,
            threshold,
            false,
            member_addresses.clone(),
        )
        .await?;

        let new_event_queue2 = Arc::new(RwLock::new(EventQueue::new()));
        let listener_for_inactive_group = create_listener(
            chain_id,
            chain_identity_arc.clone(),
            group_cache.clone(),
            new_event_queue2.clone(),
        );

        let mut event_receiver_inactive = {
            let mut eq_write = new_event_queue2.write().await;
            mock_subscribe_to_events::<G2Curve>(&mut *eq_write, "test_subscriber").await
        };

        listener_for_inactive_group.listen().await?;

        assert_no_event_received(
            &mut event_receiver_inactive,
            500,
            "Unexpectedly received an event when group state is false",
        )
        .await;

        let interruption_result = listener.handle_interruption().await;
        assert!(interruption_result.is_ok(), "Handle interruption failed");

        assert_eq!(listener.chain_id(), chain_id);

        let display_string = format!("{}", listener);
        assert_eq!(display_string, "PostCommitGroupingListener");

        Ok(())
    }
}
