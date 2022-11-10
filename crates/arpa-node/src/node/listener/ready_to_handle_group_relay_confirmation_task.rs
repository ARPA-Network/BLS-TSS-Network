use super::Listener;
use crate::node::{
    error::NodeResult,
    event::ready_to_handle_group_relay_confirmation_task::ReadyToHandleGroupRelayConfirmationTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_core::GroupRelayConfirmationTask;
use arpa_node_dal::{BLSTasksUpdater, BlockInfoFetcher, GroupInfoFetcher};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ReadyToHandleGroupRelayConfirmationTaskListener<
    B: BlockInfoFetcher,
    G: GroupInfoFetcher,
    T: BLSTasksUpdater<GroupRelayConfirmationTask>,
> {
    chain_id: usize,
    block_cache: Arc<RwLock<B>>,
    group_cache: Arc<RwLock<G>>,
    group_relay_confirmation_tasks_cache: Arc<RwLock<T>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<B: BlockInfoFetcher, G: GroupInfoFetcher, T: BLSTasksUpdater<GroupRelayConfirmationTask>>
    ReadyToHandleGroupRelayConfirmationTaskListener<B, G, T>
{
    pub fn new(
        chain_id: usize,
        block_cache: Arc<RwLock<B>>,
        group_cache: Arc<RwLock<G>>,
        group_relay_confirmation_tasks_cache: Arc<RwLock<T>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        ReadyToHandleGroupRelayConfirmationTaskListener {
            chain_id,
            block_cache,
            group_cache,
            group_relay_confirmation_tasks_cache,
            eq,
        }
    }
}

#[async_trait]
impl<
        B: BlockInfoFetcher + Sync + Send,
        G: GroupInfoFetcher + Sync + Send,
        T: BLSTasksUpdater<GroupRelayConfirmationTask> + Sync + Send,
    > EventPublisher<ReadyToHandleGroupRelayConfirmationTask>
    for ReadyToHandleGroupRelayConfirmationTaskListener<B, G, T>
{
    async fn publish(&self, event: ReadyToHandleGroupRelayConfirmationTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<
        B: BlockInfoFetcher + Sync + Send,
        G: GroupInfoFetcher + Sync + Send,
        T: BLSTasksUpdater<GroupRelayConfirmationTask> + Sync + Send,
    > Listener for ReadyToHandleGroupRelayConfirmationTaskListener<B, G, T>
{
    async fn start(mut self) -> NodeResult<()> {
        loop {
            let is_bls_ready = self.group_cache.read().await.get_state();

            if let Ok(true) = is_bls_ready {
                let current_group_index = self.group_cache.read().await.get_index()?;

                let current_block_height = self.block_cache.read().await.get_block_height();

                let available_tasks = self
                    .group_relay_confirmation_tasks_cache
                    .write()
                    .await
                    .check_and_get_available_tasks(current_block_height, current_group_index)
                    .await?;

                if !available_tasks.is_empty() {
                    self.publish(ReadyToHandleGroupRelayConfirmationTask {
                        chain_id: self.chain_id,
                        tasks: available_tasks,
                    })
                    .await;
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
