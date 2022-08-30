use super::types::Listener;
use crate::node::{
    dal::cache::{InMemoryBLSTasksQueue, InMemoryBlockInfoCache},
    dal::{
        api::{BLSTasksUpdater, BlockInfoFetcher, GroupInfoFetcher},
        types::GroupRelayTask,
    },
    error::errors::NodeResult,
    event::ready_to_handle_group_relay_task::ReadyToHandleGroupRelayTask,
    queue::event_queue::{EventPublisher, EventQueue},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct MockReadyToHandleGroupRelayTaskListener<G: GroupInfoFetcher + Sync + Send> {
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    group_cache: Arc<RwLock<G>>,
    group_relay_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<GroupRelayTask>>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoFetcher + Sync + Send> MockReadyToHandleGroupRelayTaskListener<G> {
    pub fn new(
        block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
        group_cache: Arc<RwLock<G>>,
        group_relay_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<GroupRelayTask>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockReadyToHandleGroupRelayTaskListener {
            block_cache,
            group_cache,
            group_relay_tasks_cache,
            eq,
        }
    }
}

impl<G: GroupInfoFetcher + Sync + Send> EventPublisher<ReadyToHandleGroupRelayTask>
    for MockReadyToHandleGroupRelayTaskListener<G>
{
    fn publish(&self, event: ReadyToHandleGroupRelayTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl<G: GroupInfoFetcher + Sync + Send> Listener for MockReadyToHandleGroupRelayTaskListener<G> {
    async fn start(mut self) -> NodeResult<()> {
        loop {
            let is_bls_ready = self.group_cache.read().get_state();

            if let Ok(true) = is_bls_ready {
                let current_group_index = self.group_cache.read().get_index()?;

                let current_block_height = self.block_cache.read().get_block_height();

                let available_tasks = self
                    .group_relay_tasks_cache
                    .write()
                    .check_and_get_available_tasks(current_block_height, current_group_index)?;

                if !available_tasks.is_empty() {
                    self.publish(ReadyToHandleGroupRelayTask {
                        tasks: available_tasks,
                    });
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
