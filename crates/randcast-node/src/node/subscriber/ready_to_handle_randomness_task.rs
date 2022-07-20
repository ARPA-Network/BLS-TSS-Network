use super::types::Subscriber;
use crate::node::{
    algorithm::bls::{BLSCore, MockBLSCore},
    committer::committer_client::{CommitterService, MockCommitterClient},
    dao::{
        api::SignatureResultCacheFetcher,
        types::{RandomnessTask, TaskType},
    },
    dao::{
        api::{GroupInfoFetcher, SignatureResultCacheUpdater},
        cache::{InMemoryGroupInfoCache, InMemorySignatureResultCache, RandomnessResultCache},
    },
    error::errors::NodeResult,
    event::{
        ready_to_handle_randomness_task::ReadyToHandleRandomnessTask,
        types::{Event, Topic},
    },
    queue::event_queue::{EventQueue, EventSubscriber},
    scheduler::dynamic::{DynamicTaskScheduler, SimpleDynamicTaskScheduler},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct ReadyToHandleRandomnessTaskSubscriber {
    pub chain_id: usize,
    id_address: String,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    randomness_signature_cache: Arc<RwLock<InMemorySignatureResultCache<RandomnessResultCache>>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl ReadyToHandleRandomnessTaskSubscriber {
    pub fn new(
        chain_id: usize,
        id_address: String,
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
        randomness_signature_cache: Arc<
            RwLock<InMemorySignatureResultCache<RandomnessResultCache>>,
        >,
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
    async fn handle(self, committer_clients: Vec<MockCommitterClient>) -> NodeResult<()>;

    async fn prepare_committer_clients(&self) -> NodeResult<Vec<MockCommitterClient>>;
}

pub struct MockRandomnessHandler {
    chain_id: usize,
    id_address: String,
    tasks: Vec<RandomnessTask>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    randomness_signature_cache: Arc<RwLock<InMemorySignatureResultCache<RandomnessResultCache>>>,
}

#[async_trait]
impl RandomnessHandler for MockRandomnessHandler {
    async fn handle(self, mut committer_clients: Vec<MockCommitterClient>) -> NodeResult<()> {
        for task in self.tasks {
            let bls_core = MockBLSCore {};

            let partial_signature = bls_core.partial_sign(
                self.group_cache.read().get_secret_share()?,
                task.message.as_bytes(),
            )?;

            let threshold = self.group_cache.read().get_threshold()?;

            let current_group_index = self.group_cache.read().get_index()?;

            if self.group_cache.read().is_committer(&self.id_address)? {
                if !self.randomness_signature_cache.read().contains(task.index) {
                    self.randomness_signature_cache.write().add(
                        current_group_index,
                        task.index,
                        task.message.clone(),
                        threshold,
                    )?;
                }

                self.randomness_signature_cache
                    .write()
                    .add_partial_signature(
                        task.index,
                        self.id_address.clone(),
                        partial_signature.clone(),
                    )?;
            }

            // TODO retry
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

            for committer in committer_clients.iter_mut() {
                committer
                    .commit_partial_signature(
                        self.chain_id,
                        TaskType::Randomness,
                        task.message.as_bytes().to_vec(),
                        task.index,
                        partial_signature.clone(),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    async fn prepare_committer_clients(&self) -> NodeResult<Vec<MockCommitterClient>> {
        let mut committers = self
            .group_cache
            .read()
            .get_committers()?
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>();

        committers.retain(|c| *c != self.id_address);

        let mut committer_clients = vec![];

        for committer in committers {
            let endpoint = self
                .group_cache
                .read()
                .get_member(&committer)?
                .rpc_endpint
                .as_ref()
                .unwrap()
                .to_string();

            // we retry some times here as building tonic connection needs the target rpc server available
            let mut i = 0;
            while i < 3 {
                if let Ok(committer_client) =
                    MockCommitterClient::new(self.id_address.clone(), endpoint.clone()).await
                {
                    committer_clients.push(committer_client);
                    break;
                }
                i += 1;
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            }
        }

        Ok(committer_clients)
    }
}

impl Subscriber for ReadyToHandleRandomnessTaskSubscriber {
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        println!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut ReadyToHandleRandomnessTask;

            let ReadyToHandleRandomnessTask { chain_id: _, tasks } = *Box::from_raw(struct_ptr);

            let chain_id = self.chain_id;

            let id_address = self.id_address.clone();

            let group_cache_for_handler = self.group_cache.clone();

            let randomness_signature_cache_for_handler = self.randomness_signature_cache.clone();

            self.ts.write().add_task(async move {
                let handler = MockRandomnessHandler {
                    chain_id,
                    id_address,
                    tasks,
                    group_cache: group_cache_for_handler,
                    randomness_signature_cache: randomness_signature_cache_for_handler,
                };

                if let Ok(committer_clients) = handler.prepare_committer_clients().await {
                    if let Err(e) = handler.handle(committer_clients).await {
                        println!("{:?}", e);
                    }
                }
            });
        }

        Ok(())
    }

    fn subscribe(self) {
        let eq = self.eq.clone();

        let chain_id = self.chain_id;

        let subscriber = Box::new(self);

        eq.write()
            .subscribe(Topic::ReadyToHandleRandomnessTask(chain_id), subscriber);
    }
}
