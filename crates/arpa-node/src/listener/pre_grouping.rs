use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::new_dkg_task::NewDKGTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use alloy::providers::Provider;
use arpa_contract_client::controller::ControllerLogs;
use arpa_core::{
    log::{build_task_related_payload, LogType},
    ListenerDescriptor, TaskType,
};
use arpa_dal::GroupInfoHandler;
use async_trait::async_trait;
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

    fn chain_id(&self) -> u64 {
        self.listener_descriptor.chain_id
    }

    fn listener_descriptor(&self) -> ListenerDescriptor {
        self.listener_descriptor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::types::Topic;
    use crate::queue::EventSubscriber;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use alloy::node_bindings::{Anvil, AnvilInstance};
    use alloy::primitives::{Address, U256};
    use alloy::providers::WsConnect;
    use alloy::signers::local::PrivateKeySigner;
    use alloy::sol;
    use anyhow::anyhow;
    use arpa_core::{
        build_client, random_address, Config, DKGTask, FixedIntervalRetryDescriptor,
        GeneralMainChainIdentity, ListenerType, ProviderClientWithSigner,
    };
    use arpa_dal::{cache::InMemoryGroupInfoCache, GroupInfoHandler};
    use std::sync::Arc;
    use std::time::Duration;
    use threshold_bls::schemes::bn254::G2Curve;
    use tokio::time::timeout;

    sol! {
        #[sol(ignore_unlinked)]
        #[sol(rpc)]
        MockController,
        "test-contract/MockController.json"
    }

    struct TestEnvironment {
        _anvil: AnvilInstance,
        id_address: Address,
        chain_id: u64,
        client: ProviderClientWithSigner,
        chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>>,
        controller_address: Address,
    }

    impl TestEnvironment {
        async fn new() -> NodeResult<Self> {
            let anvil = Anvil::new().spawn();
            let wallet: PrivateKeySigner = anvil.keys()[0].clone().into();
            let id_address = wallet.address();
            let chain_id = anvil.chain_id();
            let adapter_address = random_address();
            let node_registry_address = random_address();
            let config = Config::default();
            let ws_connect = WsConnect::new(anvil.ws_endpoint());
            let client = build_client(wallet.clone(), chain_id, ws_connect.clone()).await?;

            let mock_controller = MockController::deploy(client.clone(), node_registry_address)
                .await
                .map_err(|e| anyhow!("Failed to deploy mock controller: {}", e))?;
            let controller_address = *mock_controller.address();

            let chain_identity = GeneralMainChainIdentity::new(
                chain_id,
                wallet.clone(),
                ws_connect,
                client.clone(),
                anvil.ws_endpoint(),
                controller_address,
                adapter_address,
                node_registry_address,
                config
                    .get_time_limits()
                    .contract_transaction_retry_descriptor,
                config.get_time_limits().contract_view_retry_descriptor,
                None,
            );

            let chain_identity_arc = Arc::new(RwLock::new(
                Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>
            ));

            Ok(Self {
                _anvil: anvil,
                id_address,
                chain_id,
                client,
                chain_identity_arc,
                controller_address,
            })
        }

        async fn emit_dkg_event(&self, params: &DkgTaskParams) -> NodeResult<()> {
            let controller = MockController::new(self.controller_address, self.client.clone());

            let tx = controller.emitDkgTaskEvent(
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
                .get_receipt()
                .await
                .map_err(|e| anyhow!("Transaction failed: {}", e))?;

            Ok(())
        }
    }

    struct ListenerComponents {
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>>,
        event_queue: Arc<RwLock<EventQueue>>,
        listener: PreGroupingListener<G2Curve>,
    }

    impl ListenerComponents {
        async fn new(env: &TestEnvironment) -> Self {
            let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> =
                Arc::new(RwLock::new(Box::new(
                    InMemoryGroupInfoCache::<G2Curve>::new(env.id_address),
                )));
            let event_queue = Arc::new(RwLock::new(EventQueue::new()));
            let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> =
                env.chain_identity_arc.clone();

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
                vec![id_address, random_address(), random_address()]
            } else {
                vec![random_address(), random_address(), random_address()]
            };

            Self {
                members,
                global_epoch: U256::from(1),
                group_index: U256::from(2),
                group_epoch: U256::from(1),
                size: U256::from(3),
                threshold: U256::from(2),
                assignment_block_height: U256::from(100),
                coordinator_address: random_address(),
            }
        }
    }

    fn create_listener_descriptor(chain_id: u64) -> ListenerDescriptor {
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

    #[tokio::test]
    async fn test_pre_grouping_listener() -> NodeResult<()> {
        let env = TestEnvironment::new().await?;
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
            members: vec![env.id_address, random_address(), random_address()],
            coordinator_address: random_address(),
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
        let env = TestEnvironment::new().await?;
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
        env.emit_dkg_event(&params).await?;

        let received_event = timeout(Duration::from_secs(5), event_receiver.recv())
            .await
            .map_err(|_| anyhow!("Timeout: No event received after emitting DkgTask"))?
            .ok_or_else(|| anyhow!("Error: Event channel closed"))?;

        if let Some(task_event) = received_event.downcast_ref::<NewDKGTask>() {
            assert_eq!(task_event.chain_id, env.chain_id);
            assert_eq!(
                task_event.dkg_task.group_index,
                params.group_index.to::<usize>()
            );
            assert_eq!(task_event.dkg_task.epoch, params.group_epoch.to::<usize>());
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
        let env = TestEnvironment::new().await?;
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
        env.emit_dkg_event(&params).await?;

        let timeout_result = timeout(Duration::from_millis(500), event_receiver.recv()).await;
        assert!(
            timeout_result.is_err(),
            "Unexpectedly received an event when not a member"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_pre_grouping_listener_listen_same_task() -> NodeResult<()> {
        let env = TestEnvironment::new().await?;
        let components = ListenerComponents::new(&env).await;

        {
            let mut group_cache_write = components.group_cache.write().await;
            let dkg_task = DKGTask {
                group_index: 2,
                epoch: 1,
                size: 3,
                threshold: 2,
                assignment_block_height: 50,
                members: vec![env.id_address, random_address(), random_address()],
                coordinator_address: random_address(),
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
        env.emit_dkg_event(&params).await?;

        let timeout_result = timeout(Duration::from_millis(500), event_receiver.recv()).await;
        assert!(
            timeout_result.is_err(),
            "Unexpectedly received an event for same task"
        );

        Ok(())
    }
}
