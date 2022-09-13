use super::Listener;
use crate::node::{
    dal::{
        types::GroupRelayTask,
        {BLSTasksUpdater, BlockInfoFetcher, GroupInfoFetcher},
    },
    error::NodeResult,
    event::ready_to_handle_group_relay_task::ReadyToHandleGroupRelayTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct ReadyToHandleGroupRelayTaskListener<
    B: BlockInfoFetcher,
    G: GroupInfoFetcher,
    T: BLSTasksUpdater<GroupRelayTask>,
> {
    block_cache: Arc<RwLock<B>>,
    group_cache: Arc<RwLock<G>>,
    group_relay_tasks_cache: Arc<RwLock<T>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<B: BlockInfoFetcher, G: GroupInfoFetcher, T: BLSTasksUpdater<GroupRelayTask>>
    ReadyToHandleGroupRelayTaskListener<B, G, T>
{
    pub fn new(
        block_cache: Arc<RwLock<B>>,
        group_cache: Arc<RwLock<G>>,
        group_relay_tasks_cache: Arc<RwLock<T>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        ReadyToHandleGroupRelayTaskListener {
            block_cache,
            group_cache,
            group_relay_tasks_cache,
            eq,
        }
    }
}

impl<B: BlockInfoFetcher, G: GroupInfoFetcher, T: BLSTasksUpdater<GroupRelayTask>>
    EventPublisher<ReadyToHandleGroupRelayTask> for ReadyToHandleGroupRelayTaskListener<B, G, T>
{
    fn publish(&self, event: ReadyToHandleGroupRelayTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl<
        B: BlockInfoFetcher + Sync + Send,
        G: GroupInfoFetcher + Sync + Send,
        T: BLSTasksUpdater<GroupRelayTask> + Sync + Send,
    > Listener for ReadyToHandleGroupRelayTaskListener<B, G, T>
{
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
