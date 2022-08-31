use super::Subscriber;
use crate::node::{
    algorithm::bls::{BLSCore, MockBLSCore},
    committer::{
        client::MockCommitterClient, CommitterClient, CommitterClientHandler, CommitterService,
    },
    contract_client::types::Group as ContractGroup,
    contract_client::{controller::ControllerViews, rpc_mock::controller::MockControllerClient},
    dal::cache::{GroupRelayResultCache, InMemorySignatureResultCache},
    dal::{
        types::{ChainIdentity, GroupRelayTask, TaskType},
        {GroupInfoFetcher, SignatureResultCacheFetcher, SignatureResultCacheUpdater},
    },
    error::{NodeError, NodeResult},
    event::{ready_to_handle_group_relay_task::ReadyToHandleGroupRelayTask, types::Topic, Event},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use async_trait::async_trait;
use log::{error, info};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct ReadyToHandleGroupRelayTaskSubscriber<G: GroupInfoFetcher + Sync + Send> {
    main_chain_identity: Arc<RwLock<ChainIdentity>>,
    group_cache: Arc<RwLock<G>>,
    group_relay_signature_cache: Arc<RwLock<InMemorySignatureResultCache<GroupRelayResultCache>>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl<G: GroupInfoFetcher + Sync + Send> ReadyToHandleGroupRelayTaskSubscriber<G> {
    pub fn new(
        main_chain_identity: Arc<RwLock<ChainIdentity>>,
        group_cache: Arc<RwLock<G>>,
        group_relay_signature_cache: Arc<
            RwLock<InMemorySignatureResultCache<GroupRelayResultCache>>,
        >,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        ReadyToHandleGroupRelayTaskSubscriber {
            main_chain_identity,
            group_cache,
            group_relay_signature_cache,
            eq,
            ts,
        }
    }
}

#[async_trait]
pub trait GroupRelayHandler {
    async fn handle(self) -> NodeResult<()>;
}

pub struct MockGroupRelayHandler<G: GroupInfoFetcher + Sync + Send> {
    controller_address: String,
    id_address: String,
    tasks: Vec<GroupRelayTask>,
    group_cache: Arc<RwLock<G>>,
    group_relay_signature_cache: Arc<RwLock<InMemorySignatureResultCache<GroupRelayResultCache>>>,
}

impl<G: GroupInfoFetcher + Sync + Send> CommitterClientHandler<MockCommitterClient, G>
    for MockGroupRelayHandler<G>
{
    fn get_id_address(&self) -> &str {
        &self.id_address
    }

    fn get_group_cache(&self) -> Arc<RwLock<G>> {
        self.group_cache.clone()
    }
}

#[async_trait]
impl<G: GroupInfoFetcher + Sync + Send> GroupRelayHandler for MockGroupRelayHandler<G> {
    async fn handle(self) -> NodeResult<()> {
        let client =
            MockControllerClient::new(self.controller_address.clone(), self.id_address.clone());

        let committers = self.prepare_committer_clients()?;

        for task in self.tasks {
            let relayed_group = client.get_group(task.relayed_group_index).await?;

            let relayed_group: ContractGroup = relayed_group.into();

            if relayed_group.epoch != task.relayed_group_epoch {
                continue;
            }

            let relayed_group_as_bytes = bincode::serialize(&relayed_group)?;

            let bls_core = MockBLSCore {};

            let partial_signature = bls_core.partial_sign(
                self.group_cache.read().get_secret_share()?,
                &relayed_group_as_bytes,
            )?;

            let threshold = self.group_cache.read().get_threshold()?;

            let current_group_index = self.group_cache.read().get_index()?;

            if self.group_cache.read().is_committer(&self.id_address)? {
                if !self
                    .group_relay_signature_cache
                    .read()
                    .contains(task.controller_global_epoch)
                {
                    self.group_relay_signature_cache.write().add(
                        current_group_index,
                        task.controller_global_epoch,
                        relayed_group,
                        threshold,
                    )?;
                }

                self.group_relay_signature_cache
                    .write()
                    .add_partial_signature(
                        task.controller_global_epoch,
                        self.id_address.clone(),
                        partial_signature.clone(),
                    )?;
            }

            for committer in committers.iter() {
                let retry_strategy = FixedInterval::from_millis(2000).take(3);

                if let Err(err) = RetryIf::spawn(
                    retry_strategy,
                    || {
                        committer.clone().commit_partial_signature(
                            0,
                            TaskType::GroupRelay,
                            relayed_group_as_bytes.clone(),
                            task.controller_global_epoch,
                            partial_signature.clone(),
                        )
                    },
                    |e: &NodeError| {
                        error!(
                            "send partial signature to committer {0} failed. Retry... Error: {1:?}",
                            committer.get_id_address(),
                            e
                        );
                        true
                    },
                )
                .await
                {
                    error!("{:?}", err);
                }
            }
        }

        Ok(())
    }
}

impl<G: GroupInfoFetcher + Sync + Send + 'static> Subscriber
    for ReadyToHandleGroupRelayTaskSubscriber<G>
{
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        info!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut ReadyToHandleGroupRelayTask;

            let ReadyToHandleGroupRelayTask { tasks } = *Box::from_raw(struct_ptr);

            let controller_address = self
                .main_chain_identity
                .read()
                .get_provider_rpc_endpoint()
                .to_string();

            let id_address = self.main_chain_identity.read().get_id_address().to_string();

            let group_cache_for_handler = self.group_cache.clone();

            let group_relay_signature_cache_for_handler = self.group_relay_signature_cache.clone();

            self.ts.write().add_task(async move {
                let handler = MockGroupRelayHandler {
                    controller_address,
                    id_address,
                    tasks,
                    group_cache: group_cache_for_handler,
                    group_relay_signature_cache: group_relay_signature_cache_for_handler,
                };

                if let Err(e) = handler.handle().await {
                    error!("{:?}", e);
                }
            });
        }

        Ok(())
    }

    fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write()
            .subscribe(Topic::ReadyToHandleGroupRelayTask, subscriber);
    }
}
