use super::Listener;
use crate::node::{
    error::NodeResult,
    event::ready_to_fulfill_group_relay_confirmation_task::ReadyToFulfillGroupRelayConfirmationTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_dal::{
    cache::GroupRelayConfirmationResultCache, GroupInfoFetcher, SignatureResultCacheUpdater,
};
use async_trait::async_trait;
use ethers::types::Address;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct GroupRelayConfirmationSignatureAggregationListener<
    G: GroupInfoFetcher,
    C: SignatureResultCacheUpdater<GroupRelayConfirmationResultCache>,
> {
    chain_id: usize,
    id_address: Address,
    group_cache: Arc<RwLock<G>>,
    group_relay_confirmation_signature_cache: Arc<RwLock<C>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoFetcher, C: SignatureResultCacheUpdater<GroupRelayConfirmationResultCache>>
    GroupRelayConfirmationSignatureAggregationListener<G, C>
{
    pub fn new(
        chain_id: usize,
        id_address: Address,
        group_cache: Arc<RwLock<G>>,
        group_relay_confirmation_signature_cache: Arc<RwLock<C>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        GroupRelayConfirmationSignatureAggregationListener {
            chain_id,
            id_address,
            group_cache,
            group_relay_confirmation_signature_cache,
            eq,
        }
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send,
        C: SignatureResultCacheUpdater<GroupRelayConfirmationResultCache> + Sync + Send,
    > EventPublisher<ReadyToFulfillGroupRelayConfirmationTask>
    for GroupRelayConfirmationSignatureAggregationListener<G, C>
{
    async fn publish(&self, event: ReadyToFulfillGroupRelayConfirmationTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send,
        C: SignatureResultCacheUpdater<GroupRelayConfirmationResultCache> + Sync + Send,
    > Listener for GroupRelayConfirmationSignatureAggregationListener<G, C>
{
    async fn start(mut self) -> NodeResult<()> {
        loop {
            let is_committer = self.group_cache.read().await.is_committer(self.id_address);

            if let Ok(true) = is_committer {
                let ready_signatures = self
                    .group_relay_confirmation_signature_cache
                    .write()
                    .await
                    .get_ready_to_commit_signatures();

                if !ready_signatures.is_empty() {
                    self.publish(ReadyToFulfillGroupRelayConfirmationTask {
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
