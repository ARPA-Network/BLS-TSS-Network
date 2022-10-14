use super::Listener;
use crate::node::{
    error::NodeResult,
    event::ready_to_fulfill_randomness_task::ReadyToFulfillRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_dal::{cache::RandomnessResultCache, GroupInfoFetcher, SignatureResultCacheUpdater};
use async_trait::async_trait;
use ethers::types::Address;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct RandomnessSignatureAggregationListener<
    G: GroupInfoFetcher,
    C: SignatureResultCacheUpdater<RandomnessResultCache>,
> {
    chain_id: usize,
    id_address: Address,
    group_cache: Arc<RwLock<G>>,
    randomness_signature_cache: Arc<RwLock<C>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoFetcher, C: SignatureResultCacheUpdater<RandomnessResultCache>>
    RandomnessSignatureAggregationListener<G, C>
{
    pub fn new(
        chain_id: usize,
        id_address: Address,
        group_cache: Arc<RwLock<G>>,
        randomness_signature_cache: Arc<RwLock<C>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        RandomnessSignatureAggregationListener {
            chain_id,
            id_address,
            group_cache,
            randomness_signature_cache,
            eq,
        }
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send,
        C: SignatureResultCacheUpdater<RandomnessResultCache> + Sync + Send,
    > EventPublisher<ReadyToFulfillRandomnessTask>
    for RandomnessSignatureAggregationListener<G, C>
{
    async fn publish(&self, event: ReadyToFulfillRandomnessTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send,
        C: SignatureResultCacheUpdater<RandomnessResultCache> + Sync + Send,
    > Listener for RandomnessSignatureAggregationListener<G, C>
{
    async fn start(mut self) -> NodeResult<()> {
        loop {
            let is_committer = self.group_cache.read().await.is_committer(self.id_address);

            if let Ok(true) = is_committer {
                let ready_signatures = self
                    .randomness_signature_cache
                    .write()
                    .await
                    .get_ready_to_commit_signatures();

                if !ready_signatures.is_empty() {
                    self.publish(ReadyToFulfillRandomnessTask {
                        chain_id: self.chain_id,
                        tasks: ready_signatures,
                    })
                    .await;
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
