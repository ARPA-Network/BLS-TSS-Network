use crate::node::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    committer::{
        client::GeneralCommitterClient, CommitterClient, CommitterClientHandler, CommitterService,
    },
    error::NodeResult,
    event::{ready_to_handle_randomness_task::ReadyToHandleRandomnessTask, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use arpa_node_core::{
    u256_to_vec, BLSTaskType, ExponentialBackoffRetryDescriptor, RandomnessTask, SubscriberType,
    TaskType,
};
use arpa_node_dal::{
    cache::RandomnessResultCache, BLSTasksFetcher, GroupInfoFetcher, SignatureResultCacheFetcher,
    SignatureResultCacheUpdater,
};
use async_trait::async_trait;
use ethers::types::{Address, U256};
use log::{debug, error, info};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;

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
    commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl<
        G: GroupInfoFetcher<PC>,
        T: BLSTasksFetcher<RandomnessTask>,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + SignatureResultCacheFetcher<RandomnessResultCache>,
        PC: PairingCurve,
    > ReadyToHandleRandomnessTaskSubscriber<G, T, C, PC>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chain_id: usize,
        id_address: Address,
        group_cache: Arc<RwLock<G>>,
        randomness_tasks_cache: Arc<RwLock<T>>,
        randomness_signature_cache: Arc<RwLock<C>>,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
        commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor,
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
            commit_partial_signature_retry_descriptor,
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
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<PC>,
    commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor,
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

    fn get_commit_partial_signature_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor {
        self.commit_partial_signature_retry_descriptor
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher<PC> + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + Sync + Send + 'static,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + SignatureResultCacheFetcher<RandomnessResultCache>
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + Sync + Send + 'static,
    > RandomnessHandler for GeneralRandomnessHandler<G, T, C, PC>
{
    async fn handle(self) -> NodeResult<()> {
        for task in self.tasks.iter() {
            let actual_seed = [
                &u256_to_vec(&task.seed)[..],
                &u256_to_vec(&U256::from(task.assignment_block_height))[..],
            ]
            .concat();

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
                    .contains(&task.request_id)
                    .await?;
                if !contained_res {
                    let task = self
                        .randomness_tasks_cache
                        .read()
                        .await
                        .get(&task.request_id)
                        .await?;

                    self.randomness_signature_cache
                        .write()
                        .await
                        .add(current_group_index, task, actual_seed.to_vec(), threshold)
                        .await?;
                }

                self.randomness_signature_cache
                    .write()
                    .await
                    .add_partial_signature(
                        task.request_id.clone(),
                        self.id_address,
                        partial_signature.clone(),
                    )
                    .await?;
            }

            let committers = self.prepare_committer_clients().await?;

            for committer in committers.into_iter() {
                let chain_id = self.chain_id;
                let request_id = task.request_id.clone();
                let actual_seed = actual_seed.clone();
                let partial_signature = partial_signature.clone();

                self.ts.write().await.add_task(
                    TaskType::Subscriber(SubscriberType::SendingPartialSignature),
                    async move {
                        let committer_id = committer.get_committer_id_address();

                        match committer
                            .commit_partial_signature(
                                chain_id,
                                BLSTaskType::Randomness,
                                request_id,
                                actual_seed,
                                partial_signature,
                            )
                            .await
                        {
                            Ok(true) => {
                                info!(
                                    "Partial signature sent and accepted by committer: {:?}",
                                    committer_id
                                );
                            }
                            Ok(false) => {
                                info!(
                                    "Partial signature is not accepted by committer: {:?}",
                                    committer_id
                                );
                            }
                            Err(e) => {
                                error!(
                                    "Error while sending partial signature to committer: {:?}, caused by: {:?}",
                                    committer_id, e
                                );
                            }
                        }
                    },
                )?;
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

        let task_scheduler_for_handler = self.ts.clone();

        let commit_partial_signature_retry_descriptor =
            self.commit_partial_signature_retry_descriptor;

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
                    ts: task_scheduler_for_handler,
                    c: PhantomData::<PC>,
                    commit_partial_signature_retry_descriptor,
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
