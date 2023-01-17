use crate::node::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    committer::{
        client::GeneralCommitterClient, CommitterClient, CommitterClientHandler, CommitterService,
    },
    error::{NodeError, NodeResult},
    event::{ready_to_handle_randomness_task::ReadyToHandleRandomnessTask, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, SubscriberType, TaskScheduler, TaskType},
};
use arpa_node_core::{RandomnessTask, TaskType as BLSTaskType};
use arpa_node_dal::{
    cache::RandomnessResultCache, GroupInfoFetcher, SignatureResultCacheFetcher,
    SignatureResultCacheUpdater,
};
use async_trait::async_trait;
use ethers::types::Address;
use log::{debug, error};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_retry::{strategy::FixedInterval, RetryIf};

use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};

#[derive(Debug)]
pub struct ReadyToHandleRandomnessTaskSubscriber<
    G: GroupInfoFetcher,
    C: SignatureResultCacheUpdater<RandomnessResultCache>
        + SignatureResultCacheFetcher<RandomnessResultCache>,
> {
    pub chain_id: usize,
    id_address: Address,
    group_cache: Arc<RwLock<G>>,
    randomness_signature_cache: Arc<RwLock<C>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl<
        G: GroupInfoFetcher,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + SignatureResultCacheFetcher<RandomnessResultCache>,
    > ReadyToHandleRandomnessTaskSubscriber<G, C>
{
    pub fn new(
        chain_id: usize,
        id_address: Address,
        group_cache: Arc<RwLock<G>>,
        randomness_signature_cache: Arc<RwLock<C>>,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        ReadyToHandleRandomnessTaskSubscriber {
            chain_id,
            id_address,
            group_cache,
            randomness_signature_cache,
            eq,
            ts,
        }
    }
}

#[async_trait]
pub trait RandomnessHandler {
    async fn handle(self) -> NodeResult<()>;
}

pub struct GeneralRandomnessHandler<
    G: GroupInfoFetcher,
    C: SignatureResultCacheUpdater<RandomnessResultCache>
        + SignatureResultCacheFetcher<RandomnessResultCache>,
> {
    chain_id: usize,
    id_address: Address,
    tasks: Vec<RandomnessTask>,
    group_cache: Arc<RwLock<G>>,
    randomness_signature_cache: Arc<RwLock<C>>,
}

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + SignatureResultCacheFetcher<RandomnessResultCache>
            + Sync
            + Send,
    > CommitterClientHandler<GeneralCommitterClient, G> for GeneralRandomnessHandler<G, C>
{
    async fn get_id_address(&self) -> Address {
        self.id_address
    }

    fn get_group_cache(&self) -> Arc<RwLock<G>> {
        self.group_cache.clone()
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + SignatureResultCacheFetcher<RandomnessResultCache>
            + Sync
            + Send,
    > RandomnessHandler for GeneralRandomnessHandler<G, C>
{
    async fn handle(self) -> NodeResult<()> {
        let committers = self.prepare_committer_clients().await?;

        for task in self.tasks {
            let bls_core = SimpleBLSCore {};

            let partial_signature = bls_core.partial_sign(
                self.group_cache.read().await.get_secret_share()?,
                task.message.as_bytes(),
            )?;

            let threshold = self.group_cache.read().await.get_threshold()?;

            let current_group_index = self.group_cache.read().await.get_index()?;

            if self
                .group_cache
                .read()
                .await
                .is_committer(self.id_address)?
            {
                let contained_res = self
                    .randomness_signature_cache
                    .read()
                    .await
                    .contains(task.index);
                if !contained_res {
                    self.randomness_signature_cache.write().await.add(
                        current_group_index,
                        task.index,
                        task.message.clone(),
                        threshold,
                    )?;
                }

                self.randomness_signature_cache
                    .write()
                    .await
                    .add_partial_signature(
                        task.index,
                        self.id_address,
                        partial_signature.clone(),
                    )?;
            }

            for committer in committers.iter() {
                let retry_strategy = FixedInterval::from_millis(2000).take(3);

                let chain_id = self.chain_id;

                if let Err(err) = RetryIf::spawn(
                    retry_strategy,
                    || {
                        committer.clone().commit_partial_signature(
                            chain_id,
                            BLSTaskType::Randomness,
                            task.message.as_bytes().to_vec(),
                            task.index,
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

#[async_trait]
impl<
        G: GroupInfoFetcher + std::fmt::Debug + Sync + Send + 'static,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + SignatureResultCacheFetcher<RandomnessResultCache>
            + std::fmt::Debug
            + Sync
            + Send
            + 'static,
    > Subscriber for ReadyToHandleRandomnessTaskSubscriber<G, C>
{
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let ReadyToHandleRandomnessTask { tasks, .. } = payload
            .as_any()
            .downcast_ref::<ReadyToHandleRandomnessTask>()
            .unwrap()
            .clone();

        let chain_id = self.chain_id;

        let id_address = self.id_address;

        let group_cache_for_handler = self.group_cache.clone();

        let randomness_signature_cache_for_handler = self.randomness_signature_cache.clone();

        self.ts.write().await.add_task(
            TaskType::Subscriber(SubscriberType::ReadyToHandleRandomnessTask),
            async move {
                let handler = GeneralRandomnessHandler {
                    chain_id,
                    id_address,
                    tasks,
                    group_cache: group_cache_for_handler,
                    randomness_signature_cache: randomness_signature_cache_for_handler,
                };

                if let Err(e) = handler.handle().await {
                    error!("{:?}", e);
                }
            },
        )?;

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let chain_id = self.chain_id;

        let subscriber = Box::new(self);

        eq.write()
            .await
            .subscribe(Topic::ReadyToHandleRandomnessTask(chain_id), subscriber);
    }
}

impl<
        G: GroupInfoFetcher + std::fmt::Debug + Sync + Send + 'static,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + SignatureResultCacheFetcher<RandomnessResultCache>
            + std::fmt::Debug
            + Sync
            + Send
            + 'static,
    > DebuggableSubscriber for ReadyToHandleRandomnessTaskSubscriber<G, C>
{
}
