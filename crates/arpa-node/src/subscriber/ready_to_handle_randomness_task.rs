use crate::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    committer::{
        client::GeneralCommitterClient, CommitterClient, CommitterClientHandler, CommitterService,
    },
    error::NodeResult,
    event::{ready_to_handle_randomness_task::ReadyToHandleRandomnessTask, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use arpa_core::{
    u256_to_vec, BLSTaskType, ExponentialBackoffRetryDescriptor, RandomnessTask, SubscriberType,
    TaskType,
};
use arpa_dal::cache::RandomnessResultCache;
use arpa_dal::{BLSTasksHandler, GroupInfoHandler, SignatureResultCacheHandler};
use async_trait::async_trait;
use ethers::types::{Address, U256};
use log::{debug, error, info};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::{
    group::Curve,
    sig::{SignatureScheme, ThresholdScheme},
};
use tokio::sync::RwLock;

use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};

#[derive(Debug)]
pub struct ReadyToHandleRandomnessTaskSubscriber<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
> {
    pub chain_id: usize,
    id_address: Address,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
    randomness_signature_cache:
        Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<PC>,
    s: PhantomData<S>,
    commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl<PC: Curve, S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>>
    ReadyToHandleRandomnessTaskSubscriber<PC, S>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chain_id: usize,
        id_address: Address,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
        randomness_signature_cache: Arc<
            RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>,
        >,
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
            s: PhantomData,
            commit_partial_signature_retry_descriptor,
        }
    }
}

#[async_trait]
pub trait RandomnessHandler {
    async fn handle(self) -> NodeResult<()>;
}

pub struct GeneralRandomnessHandler<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
> {
    chain_id: usize,
    id_address: Address,
    tasks: Vec<RandomnessTask>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
    randomness_signature_cache:
        Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<PC>,
    s: PhantomData<S>,
    commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

#[async_trait]
impl<
        PC: Curve + Sync + Send,
        S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar> + Sync + Send,
    > CommitterClientHandler<GeneralCommitterClient, PC> for GeneralRandomnessHandler<PC, S>
{
    async fn get_id_address(&self) -> Address {
        self.id_address
    }

    fn get_group_cache(&self) -> Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>> {
        self.group_cache.clone()
    }

    fn get_commit_partial_signature_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor {
        self.commit_partial_signature_retry_descriptor
    }
}

#[async_trait]
impl<
        PC: Curve + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Sync
            + Send
            + 'static,
    > RandomnessHandler for GeneralRandomnessHandler<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
    async fn handle(self) -> NodeResult<()> {
        for task in self.tasks.iter() {
            let actual_seed = [
                &u256_to_vec(&task.seed)[..],
                &u256_to_vec(&U256::from(task.assignment_block_height))[..],
            ]
            .concat();

            let partial_signature = SimpleBLSCore::<PC, S>::partial_sign(
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
                    TaskType::Subscriber(chain_id, SubscriberType::SendingPartialSignature),
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
        PC: Curve + std::fmt::Debug + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Sync
            + Send
            + 'static,
    > Subscriber for ReadyToHandleRandomnessTaskSubscriber<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
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
            TaskType::Subscriber(chain_id, SubscriberType::ReadyToHandleRandomnessTask),
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
                    s: PhantomData::<S>,
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
        PC: Curve + std::fmt::Debug + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Sync
            + Send
            + 'static,
    > DebuggableSubscriber for ReadyToHandleRandomnessTaskSubscriber<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
}
