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

#[cfg(feature = "unittest")]
mod tests {
    use super::*;
    use crate::event::types::Topic;
    use crate::queue::EventSubscriber;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use crate::test_contracts::mockcontroller::deploy_with_args_and_get_mock_controller;
    use anyhow::anyhow;
    use arpa_core::{
        Config, DKGTask, FixedIntervalRetryDescriptor, GeneralMainChainIdentity, ListenerType,
    };
    use arpa_dal::{cache::InMemoryGroupInfoCache, GroupInfoHandler};
    use ethers::middleware::SignerMiddleware;
    use ethers::signers::{LocalWallet, Signer};
    use ethers::types::{Address, U256};
    use ethers::{
        providers::{Http, Provider, Ws},
        utils::{Anvil, AnvilInstance},
    };
    use std::sync::Arc;
    use std::time::Duration;
    use threshold_bls::schemes::bn254::G2Curve;
    use tokio::time::timeout;

    struct TestEnvironment {
        anvil: AnvilInstance,
        wallet: LocalWallet,
        id_address: Address,
        chain_id: usize,
        controller_address: Address,
        adapter_address: Address,
        node_registry_address: Address,
    }

    impl TestEnvironment {
        fn new() -> Self {
            let anvil = Anvil::new().spawn();
            let wallet: LocalWallet = anvil.keys()[0].clone().into();
            let id_address = wallet.address();
            let chain_id = anvil.chain_id() as usize;
            let controller_address = Address::random();
            let adapter_address = Address::random();
            let node_registry_address = Address::random();

            Self {
                anvil,
                wallet,
                id_address,
                chain_id,
                controller_address,
                adapter_address,
                node_registry_address,
            }
        }

        async fn deploy_controller(
            &self,
        ) -> NodeResult<
            crate::test_contracts::mockcontroller::MockController<
                SignerMiddleware<Provider<Http>, LocalWallet>,
            >,
        > {
            let http_provider = Provider::<Http>::try_from(self.anvil.endpoint())
                .map_err(|e| anyhow!("Failed to create HTTP provider: {}", e))?;

            let client = Arc::new(SignerMiddleware::new(
                http_provider,
                self.wallet.clone().with_chain_id(self.anvil.chain_id()),
            ));

            deploy_with_args_and_get_mock_controller(client, self.node_registry_address)
                .await
                .map_err(|e| anyhow!("Failed to deploy mock controller contract: {}", e).into())
        }

        async fn create_chain_identity(&self) -> GeneralMainChainIdentity {
            let ws_provider = Arc::new(
                Provider::<Ws>::connect(self.anvil.ws_endpoint())
                    .await
                    .unwrap(),
            );
            let config = Config::default();

            GeneralMainChainIdentity::new(
                self.chain_id,
                self.wallet.clone(),
                ws_provider,
                self.anvil.ws_endpoint(),
                self.controller_address,
                self.adapter_address,
                self.node_registry_address,
                config
                    .get_time_limits()
                    .contract_transaction_retry_descriptor,
                config.get_time_limits().contract_view_retry_descriptor,
                None,
            )
        }
    }

    struct ListenerComponents {
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>>,
        event_queue: Arc<RwLock<EventQueue>>,
        listener: PreGroupingListener<G2Curve>,
    }

    impl ListenerComponents {
        async fn new(env: &TestEnvironment) -> Self {
            let chain_identity = env.create_chain_identity().await;
            let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> =
                Arc::new(RwLock::new(Box::new(
                    InMemoryGroupInfoCache::<G2Curve>::new(env.id_address),
                )));
            let event_queue = Arc::new(RwLock::new(EventQueue::new()));
            let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = Arc::new(
                RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>),
            );

            let listener_descriptor = create_listener_descriptor(env.chain_id);
            let listener = PreGroupingListener::<G2Curve>::new(
                listener_descriptor,
                chain_identity_arc.clone(),
                group_cache.clone(),
                event_queue.clone(),
            );

            Self {
                group_cache,
                event_queue,
                listener,
            }
        }
    }

    struct DkgTaskParams {
        members: Vec<Address>,
        global_epoch: U256,
        group_index: U256,
        group_epoch: U256,
        size: U256,
        threshold: U256,
        assignment_block_height: U256,
        coordinator_address: Address,
    }

    impl DkgTaskParams {
        fn new(id_address: Address, include_self: bool) -> Self {
            let members = if include_self {
                vec![id_address, Address::random(), Address::random()]
            } else {
                vec![Address::random(), Address::random(), Address::random()]
            };

            Self {
                members,
                global_epoch: U256::from(1),
                group_index: U256::from(2),
                group_epoch: U256::from(1),
                size: U256::from(3),
                threshold: U256::from(2),
                assignment_block_height: U256::from(100),
                coordinator_address: Address::random(),
            }
        }
    }

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

    async fn setup_event_subscriber(
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
                if let Some(dkg_task_event) = payload.as_any().downcast_ref::<NewDKGTask>() {
                    let cloned_event = NewDKGTask {
                        chain_id: dkg_task_event.chain_id,
                        dkg_task: dkg_task_event.dkg_task.clone(),
                        self_index: dkg_task_event.self_index,
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

        eq.subscribe(Topic::NewDKGTask, Box::new(subscriber));
        receiver
    }

    async fn emit_dkg_event(
        controller: &crate::test_contracts::mockcontroller::MockController<
            SignerMiddleware<Provider<Http>, LocalWallet>,
        >,
        params: &DkgTaskParams,
    ) -> NodeResult<()> {
        let tx = controller.emit_dkg_task_event(
            params.global_epoch,
            params.group_index,
            params.group_epoch,
            params.size,
            params.threshold,
            params.members.clone(),
            params.assignment_block_height,
            params.coordinator_address,
        );

        let pending_tx = tx
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        pending_tx
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_pre_grouping_listener() -> NodeResult<()> {
        let env = TestEnvironment::new();
        let components = ListenerComponents::new(&env).await;

        let mut event_receiver = {
            let mut eq_write = components.event_queue.write().await;
            setup_event_subscriber(&mut *eq_write, "test_subscriber").await
        };

        let dkg_task = DKGTask {
            group_index: 1,
            epoch: 1,
            size: 3,
            threshold: 2,
            assignment_block_height: 100,
            members: vec![env.id_address, Address::random(), Address::random()],
            coordinator_address: Address::random(),
        };

        components
            .listener
            .publish(NewDKGTask {
                chain_id: env.chain_id,
                dkg_task: dkg_task.clone(),
                self_index: 0,
            })
            .await;

        let received_event = timeout(Duration::from_secs(1), event_receiver.recv())
            .await
            .map_err(|_| anyhow!("Timeout: No event received"))?
            .ok_or_else(|| anyhow!("Error: Event channel closed"))?;

        if let Some(task_event) = received_event.downcast_ref::<NewDKGTask>() {
            assert_eq!(task_event.chain_id, env.chain_id);
            assert_eq!(task_event.dkg_task.group_index, 1);
            assert_eq!(task_event.dkg_task.epoch, 1);
            assert_eq!(task_event.dkg_task.members[0], env.id_address);
            assert_eq!(task_event.self_index, 0);
        } else {
            return Err(anyhow!("Received unexpected event type").into());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_pre_grouping_listener_listen() -> NodeResult<()> {
        let env = TestEnvironment::new();
        let controller = env.deploy_controller().await?;
        let components = ListenerComponents::new(&env).await;

        let mut event_receiver = {
            let mut eq_write = components.event_queue.write().await;
            setup_event_subscriber(&mut *eq_write, "test_subscriber").await
        };

        tokio::spawn(async move {
            if let Err(e) = components.listener.listen().await {
                println!("Listener error: {:?}", e);
            }
        });

        tokio::time::sleep(Duration::from_millis(500)).await;

        let params = DkgTaskParams::new(env.id_address, true);
        emit_dkg_event(&controller, &params).await?;

        let received_event = timeout(Duration::from_secs(5), event_receiver.recv())
            .await
            .map_err(|_| anyhow!("Timeout: No event received after emitting DkgTask"))?
            .ok_or_else(|| anyhow!("Error: Event channel closed"))?;

        if let Some(task_event) = received_event.downcast_ref::<NewDKGTask>() {
            assert_eq!(task_event.chain_id, env.chain_id);
            assert_eq!(
                task_event.dkg_task.group_index,
                params.group_index.as_usize()
            );
            assert_eq!(task_event.dkg_task.epoch, params.group_epoch.as_usize());
            assert_eq!(task_event.dkg_task.members, params.members);
            assert_eq!(
                task_event.dkg_task.coordinator_address,
                params.coordinator_address
            );
            assert_eq!(task_event.self_index, 0);
        } else {
            return Err(anyhow!("Received unexpected event type").into());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_pre_grouping_listener_listen_not_member() -> NodeResult<()> {
        let env = TestEnvironment::new();
        let controller = env.deploy_controller().await?;
        let components = ListenerComponents::new(&env).await;

        let mut event_receiver = {
            let mut eq_write = components.event_queue.write().await;
            setup_event_subscriber(&mut *eq_write, "test_subscriber").await
        };

        tokio::spawn(async move {
            if let Err(e) = components.listener.listen().await {
                println!("Listener error: {:?}", e);
            }
        });

        tokio::time::sleep(Duration::from_millis(500)).await;

        let params = DkgTaskParams::new(env.id_address, false);
        emit_dkg_event(&controller, &params).await?;

        let timeout_result = timeout(Duration::from_millis(500), event_receiver.recv()).await;
        assert!(
            timeout_result.is_err(),
            "Unexpectedly received an event when not a member"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_pre_grouping_listener_listen_same_task() -> NodeResult<()> {
        let env = TestEnvironment::new();
        let controller = env.deploy_controller().await?;
        let components = ListenerComponents::new(&env).await;

        {
            let mut group_cache_write = components.group_cache.write().await;
            let dkg_task = DKGTask {
                group_index: 2,
                epoch: 1,
                size: 3,
                threshold: 2,
                assignment_block_height: 50,
                members: vec![env.id_address, Address::random(), Address::random()],
                coordinator_address: Address::random(),
            };

            group_cache_write.save_task_info(0, dkg_task).await?;

            assert_eq!(group_cache_write.get_index().unwrap(), 2);
            assert_eq!(group_cache_write.get_epoch().unwrap(), 1);
        }

        let mut event_receiver = {
            let mut eq_write = components.event_queue.write().await;
            setup_event_subscriber(&mut *eq_write, "test_subscriber").await
        };

        tokio::spawn(async move {
            if let Err(e) = components.listener.listen().await {
                println!("Listener error: {:?}", e);
            }
        });

        tokio::time::sleep(Duration::from_millis(500)).await;

        let params = DkgTaskParams::new(env.id_address, true);
        emit_dkg_event(&controller, &params).await?;

        let timeout_result = timeout(Duration::from_millis(500), event_receiver.recv()).await;
        assert!(
            timeout_result.is_err(),
            "Unexpectedly received an event for same task"
        );

        Ok(())
    }
}
