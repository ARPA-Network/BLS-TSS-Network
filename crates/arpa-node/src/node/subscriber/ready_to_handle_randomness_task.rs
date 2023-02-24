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
use arpa_node_core::{address_to_string, RandomnessTask, TaskType as BLSTaskType};
use arpa_node_dal::{
    cache::RandomnessResultCache, BLSTasksFetcher, GroupInfoFetcher, SignatureResultCacheFetcher,
    SignatureResultCacheUpdater,
};
use async_trait::async_trait;
use ethers::types::{Address, U256};
use log::{debug, error};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;
use tokio_retry::{strategy::FixedInterval, RetryIf};

use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};

#[derive(Debug)]
pub struct ReadyToHandleRandomnessTaskSubscriber<
    G: GroupInfoFetcher<PC>,
    T: BLSTasksFetcher<RandomnessTask>,
    C: SignatureResultCacheUpdater<RandomnessResultCache>
        + SignatureResultCacheFetcher<RandomnessResultCache>,
    PC: PairingCurve,
> {
    pub chain_id: usize,
    id_address: Address,
    group_cache: Arc<RwLock<G>>,
    randomness_tasks_cache: Arc<RwLock<T>>,
    randomness_signature_cache: Arc<RwLock<C>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<PC>,
}

impl<
        G: GroupInfoFetcher<PC>,
        T: BLSTasksFetcher<RandomnessTask>,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + SignatureResultCacheFetcher<RandomnessResultCache>,
        PC: PairingCurve,
    > ReadyToHandleRandomnessTaskSubscriber<G, T, C, PC>
{
    pub fn new(
        chain_id: usize,
        id_address: Address,
        group_cache: Arc<RwLock<G>>,
        randomness_tasks_cache: Arc<RwLock<T>>,
        randomness_signature_cache: Arc<RwLock<C>>,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        ReadyToHandleRandomnessTaskSubscriber {
            chain_id,
            id_address,
            group_cache,
            randomness_tasks_cache,
            randomness_signature_cache,
            eq,
            ts,
            c: PhantomData,
        }
    }
}

#[async_trait]
pub trait RandomnessHandler {
    async fn handle(self) -> NodeResult<()>;
}

pub struct GeneralRandomnessHandler<
    G: GroupInfoFetcher<PC>,
    T: BLSTasksFetcher<RandomnessTask>,
    C: SignatureResultCacheUpdater<RandomnessResultCache>
        + SignatureResultCacheFetcher<RandomnessResultCache>,
    PC: PairingCurve,
> {
    chain_id: usize,
    id_address: Address,
    tasks: Vec<RandomnessTask>,
    group_cache: Arc<RwLock<G>>,
    randomness_tasks_cache: Arc<RwLock<T>>,
    randomness_signature_cache: Arc<RwLock<C>>,
    c: PhantomData<PC>,
}

#[async_trait]
impl<
        G: GroupInfoFetcher<PC> + Sync + Send,
        T: BLSTasksFetcher<RandomnessTask> + Sync + Send,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + SignatureResultCacheFetcher<RandomnessResultCache>
            + Sync
            + Send,
        PC: PairingCurve + Sync + Send,
    > CommitterClientHandler<GeneralCommitterClient, G, PC>
    for GeneralRandomnessHandler<G, T, C, PC>
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
        G: GroupInfoFetcher<PC> + Sync + Send,
        T: BLSTasksFetcher<RandomnessTask> + Sync + Send,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + SignatureResultCacheFetcher<RandomnessResultCache>
            + Sync
            + Send,
        PC: PairingCurve + Sync + Send + 'static,
    > RandomnessHandler for GeneralRandomnessHandler<G, T, C, PC>
{
    async fn handle(self) -> NodeResult<()> {
        let committers = self.prepare_committer_clients().await?;

        for task in self.tasks {
            let mut seed_bytes = vec![0u8; 32];
            task.seed.to_big_endian(&mut seed_bytes);
            let mut block_num_bytes = vec![0u8; 32];
            U256::from(task.assignment_block_height).to_big_endian(&mut block_num_bytes);
            let actual_seed = [&seed_bytes[..], &block_num_bytes[..]].concat();

            let partial_signature = SimpleBLSCore::<PC>::partial_sign(
                self.group_cache.read().await.get_secret_share()?,
                &actual_seed,
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
                    .contains(&task.request_id);
                if !contained_res {
                    let task = self
                        .randomness_tasks_cache
                        .read()
                        .await
                        .get(&task.request_id)
                        .await?;

                    self.randomness_signature_cache.write().await.add(
                        current_group_index,
                        task,
                        actual_seed.to_vec(),
                        threshold,
                    )?;
                }

                self.randomness_signature_cache
                    .write()
                    .await
                    .add_partial_signature(
                        task.request_id.clone(),
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
                            actual_seed.to_vec(),
                            task.request_id.clone(),
                            partial_signature.clone(),
                        )
                    },
                    |e: &NodeError| {
                        error!(
                            "send partial signature to committer {0} failed. Retry... Error: {1:?}",
                            address_to_string(committer.get_id_address()),
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
        G: GroupInfoFetcher<PC> + std::fmt::Debug + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + std::fmt::Debug + Sync + Send + 'static,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + SignatureResultCacheFetcher<RandomnessResultCache>
            + std::fmt::Debug
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Sync + Send + 'static,
    > Subscriber for ReadyToHandleRandomnessTaskSubscriber<G, T, C, PC>
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

        let randomness_tasks_cache_for_handler = self.randomness_tasks_cache.clone();

        let randomness_signature_cache_for_handler = self.randomness_signature_cache.clone();

        self.ts.write().await.add_task(
            TaskType::Subscriber(SubscriberType::ReadyToHandleRandomnessTask),
            async move {
                let handler = GeneralRandomnessHandler {
                    chain_id,
                    id_address,
                    tasks,
                    group_cache: group_cache_for_handler,
                    randomness_tasks_cache: randomness_tasks_cache_for_handler,
                    randomness_signature_cache: randomness_signature_cache_for_handler,
                    c: PhantomData::<PC>,
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
        G: GroupInfoFetcher<PC> + std::fmt::Debug + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + std::fmt::Debug + Sync + Send + 'static,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + SignatureResultCacheFetcher<RandomnessResultCache>
            + std::fmt::Debug
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Sync + Send + 'static,
    > DebuggableSubscriber for ReadyToHandleRandomnessTaskSubscriber<G, T, C, PC>
{
}
