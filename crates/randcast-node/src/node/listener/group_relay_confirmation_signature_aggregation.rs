use super::types::Listener;
use crate::node::{
    dao::{
        api::{GroupInfoFetcher, SignatureResultCacheUpdater},
        cache::{
            GroupRelayConfirmationResultCache, InMemoryGroupInfoCache, InMemorySignatureResultCache,
        },
    },
    error::errors::NodeResult,
    event::ready_to_fulfill_group_relay_confirmation_task::ReadyToFulfillGroupRelayConfirmationTask,
    queue::event_queue::{EventPublisher, EventQueue},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct MockGroupRelayConfirmationSignatureAggregationListener {
    chain_id: usize,
    id_address: String,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    group_relay_confirmation_signature_cache:
        Arc<RwLock<InMemorySignatureResultCache<GroupRelayConfirmationResultCache>>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl MockGroupRelayConfirmationSignatureAggregationListener {
    pub fn new(
        chain_id: usize,
        id_address: String,
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
        group_relay_confirmation_signature_cache: Arc<
            RwLock<InMemorySignatureResultCache<GroupRelayConfirmationResultCache>>,
        >,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockGroupRelayConfirmationSignatureAggregationListener {
            chain_id,
            id_address,
            group_cache,
            group_relay_confirmation_signature_cache,
            eq,
        }
    }
}

impl EventPublisher<ReadyToFulfillGroupRelayConfirmationTask>
    for MockGroupRelayConfirmationSignatureAggregationListener
{
    fn publish(&self, event: ReadyToFulfillGroupRelayConfirmationTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl Listener for MockGroupRelayConfirmationSignatureAggregationListener {
    async fn start(mut self) -> NodeResult<()> {
        loop {
            let is_committer = self.group_cache.read().is_committer(&self.id_address);

            if let Ok(true) = is_committer {
                let ready_signatures = self
                    .group_relay_confirmation_signature_cache
                    .write()
                    .get_ready_to_commit_signatures();

                if !ready_signatures.is_empty() {
                    self.publish(ReadyToFulfillGroupRelayConfirmationTask {
                        chain_id: self.chain_id,
                        tasks: ready_signatures,
                    });
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
