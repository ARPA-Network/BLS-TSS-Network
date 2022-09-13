use super::Listener;
use crate::node::{
    dal::{
        cache::GroupRelayResultCache,
        {GroupInfoFetcher, SignatureResultCacheUpdater},
    },
    error::NodeResult,
    event::ready_to_fulfill_group_relay_task::ReadyToFulfillGroupRelayTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use async_trait::async_trait;
use ethers::types::Address;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct GroupRelaySignatureAggregationListener<
    G: GroupInfoFetcher,
    C: SignatureResultCacheUpdater<GroupRelayResultCache>,
> {
    id_address: Address,
    group_cache: Arc<RwLock<G>>,
    group_relay_signature_cache: Arc<RwLock<C>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoFetcher, C: SignatureResultCacheUpdater<GroupRelayResultCache>>
    GroupRelaySignatureAggregationListener<G, C>
{
    pub fn new(
        id_address: Address,
        group_cache: Arc<RwLock<G>>,
        group_relay_signature_cache: Arc<RwLock<C>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        GroupRelaySignatureAggregationListener {
            id_address,
            group_cache,
            group_relay_signature_cache,
            eq,
        }
    }
}

impl<G: GroupInfoFetcher, C: SignatureResultCacheUpdater<GroupRelayResultCache>>
    EventPublisher<ReadyToFulfillGroupRelayTask> for GroupRelaySignatureAggregationListener<G, C>
{
    fn publish(&self, event: ReadyToFulfillGroupRelayTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send,
        C: SignatureResultCacheUpdater<GroupRelayResultCache> + Sync + Send,
    > Listener for GroupRelaySignatureAggregationListener<G, C>
{
    async fn start(mut self) -> NodeResult<()> {
        loop {
            let is_committer = self.group_cache.read().is_committer(self.id_address);

            if let Ok(true) = is_committer {
                let ready_signatures = self
                    .group_relay_signature_cache
                    .write()
                    .get_ready_to_commit_signatures();

                if !ready_signatures.is_empty() {
                    self.publish(ReadyToFulfillGroupRelayTask {
                        tasks: ready_signatures,
                    });
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
