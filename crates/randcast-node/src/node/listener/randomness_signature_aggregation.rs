use super::types::Listener;
use crate::node::{
    dao::{
        api::{GroupInfoFetcher, SignatureResultCacheUpdater},
        cache::{InMemoryGroupInfoCache, InMemorySignatureResultCache, RandomnessResultCache},
    },
    error::errors::NodeResult,
    event::ready_to_fulfill_randomness_task::ReadyToFulfillRandomnessTask,
    queue::event_queue::{EventPublisher, EventQueue},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct MockRandomnessSignatureAggregationListener {
    chain_id: usize,
    id_address: String,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    randomness_signature_cache: Arc<RwLock<InMemorySignatureResultCache<RandomnessResultCache>>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl MockRandomnessSignatureAggregationListener {
    pub fn new(
        chain_id: usize,
        id_address: String,
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
        randomness_signature_cache: Arc<
            RwLock<InMemorySignatureResultCache<RandomnessResultCache>>,
        >,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockRandomnessSignatureAggregationListener {
            chain_id,
            id_address,
            group_cache,
            randomness_signature_cache,
            eq,
        }
    }
}

impl EventPublisher<ReadyToFulfillRandomnessTask> for MockRandomnessSignatureAggregationListener {
    fn publish(&self, event: ReadyToFulfillRandomnessTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl Listener for MockRandomnessSignatureAggregationListener {
    async fn start(mut self) -> NodeResult<()> {
        loop {
            let is_committer = self.group_cache.read().is_committer(&self.id_address);

            if let Ok(true) = is_committer {
                let ready_signatures = self
                    .randomness_signature_cache
                    .write()
                    .get_ready_to_commit_signatures();

                if !ready_signatures.is_empty() {
                    self.publish(ReadyToFulfillRandomnessTask {
                        chain_id: self.chain_id,
                        tasks: ready_signatures,
                    });
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
