use super::types::Listener;
use crate::node::{
    dal::{
        api::{GroupInfoFetcher, SignatureResultCacheUpdater},
        cache::{GroupRelayResultCache, InMemorySignatureResultCache},
    },
    error::errors::NodeResult,
    event::ready_to_fulfill_group_relay_task::ReadyToFulfillGroupRelayTask,
    queue::event_queue::{EventPublisher, EventQueue},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct MockGroupRelaySignatureAggregationListener<G: GroupInfoFetcher + Sync + Send> {
    id_address: String,
    group_cache: Arc<RwLock<G>>,
    group_relay_signature_cache: Arc<RwLock<InMemorySignatureResultCache<GroupRelayResultCache>>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoFetcher + Sync + Send> MockGroupRelaySignatureAggregationListener<G> {
    pub fn new(
        id_address: String,
        group_cache: Arc<RwLock<G>>,
        group_relay_signature_cache: Arc<
            RwLock<InMemorySignatureResultCache<GroupRelayResultCache>>,
        >,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockGroupRelaySignatureAggregationListener {
            id_address,
            group_cache,
            group_relay_signature_cache,
            eq,
        }
    }
}

impl<G: GroupInfoFetcher + Sync + Send> EventPublisher<ReadyToFulfillGroupRelayTask>
    for MockGroupRelaySignatureAggregationListener<G>
{
    fn publish(&self, event: ReadyToFulfillGroupRelayTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl<G: GroupInfoFetcher + Sync + Send> Listener for MockGroupRelaySignatureAggregationListener<G> {
    async fn start(mut self) -> NodeResult<()> {
        loop {
            let is_committer = self.group_cache.read().is_committer(&self.id_address);

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
