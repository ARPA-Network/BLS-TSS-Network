use super::Listener;
use crate::node::{
    dal::cache::{InMemoryBLSTasksQueue, InMemoryBlockInfoCache},
    dal::{
        types::GroupRelayConfirmationTask,
        {BLSTasksUpdater, BlockInfoFetcher, GroupInfoFetcher},
    },
    error::NodeResult,
    event::ready_to_handle_group_relay_confirmation_task::ReadyToHandleGroupRelayConfirmationTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct MockReadyToHandleGroupRelayConfirmationTaskListener<G: GroupInfoFetcher + Sync + Send> {
    chain_id: usize,
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    group_cache: Arc<RwLock<G>>,
    group_relay_confirmation_tasks_cache:
        Arc<RwLock<InMemoryBLSTasksQueue<GroupRelayConfirmationTask>>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoFetcher + Sync + Send> MockReadyToHandleGroupRelayConfirmationTaskListener<G> {
    pub fn new(
        chain_id: usize,
        block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
        group_cache: Arc<RwLock<G>>,
        group_relay_confirmation_tasks_cache: Arc<
            RwLock<InMemoryBLSTasksQueue<GroupRelayConfirmationTask>>,
        >,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockReadyToHandleGroupRelayConfirmationTaskListener {
            chain_id,
            block_cache,
            group_cache,
            group_relay_confirmation_tasks_cache,
            eq,
        }
    }
}

impl<G: GroupInfoFetcher + Sync + Send> EventPublisher<ReadyToHandleGroupRelayConfirmationTask>
    for MockReadyToHandleGroupRelayConfirmationTaskListener<G>
{
    fn publish(&self, event: ReadyToHandleGroupRelayConfirmationTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl<G: GroupInfoFetcher + Sync + Send> Listener
    for MockReadyToHandleGroupRelayConfirmationTaskListener<G>
{
    async fn start(mut self) -> NodeResult<()> {
        loop {
            let is_bls_ready = self.group_cache.read().get_state();

            if let Ok(true) = is_bls_ready {
                let current_group_index = self.group_cache.read().get_index()?;

                let current_block_height = self.block_cache.read().get_block_height();

                let available_tasks = self
                    .group_relay_confirmation_tasks_cache
                    .write()
                    .check_and_get_available_tasks(current_block_height, current_group_index)?;

                if !available_tasks.is_empty() {
                    self.publish(ReadyToHandleGroupRelayConfirmationTask {
                        chain_id: self.chain_id,
                        tasks: available_tasks,
                    });
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
