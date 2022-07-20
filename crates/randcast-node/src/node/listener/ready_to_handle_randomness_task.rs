use super::types::Listener;
use crate::node::{
    contract_client::adapter_client::{AdapterViews, MockAdapterClient},
    dao::cache::{InMemoryBLSTasksQueue, InMemoryBlockInfoCache, InMemoryGroupInfoCache},
    dao::{
        api::{BLSTasksUpdater, BlockInfoFetcher, GroupInfoFetcher},
        types::{ChainIdentity, RandomnessTask},
    },
    error::errors::NodeResult,
    event::ready_to_handle_randomness_task::ReadyToHandleRandomnessTask,
    queue::event_queue::{EventPublisher, EventQueue},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct MockReadyToHandleRandomnessTaskListener {
    chain_id: usize,
    id_address: String,
    chain_identity: Arc<RwLock<ChainIdentity>>,
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    randomness_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<RandomnessTask>>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl MockReadyToHandleRandomnessTaskListener {
    pub fn new(
        chain_id: usize,
        id_address: String,
        chain_identity: Arc<RwLock<ChainIdentity>>,
        block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
        randomness_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<RandomnessTask>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockReadyToHandleRandomnessTaskListener {
            chain_id,
            id_address,
            chain_identity,
            block_cache,
            group_cache,
            randomness_tasks_cache,
            eq,
        }
    }
}

impl EventPublisher<ReadyToHandleRandomnessTask> for MockReadyToHandleRandomnessTaskListener {
    fn publish(&self, event: ReadyToHandleRandomnessTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl Listener for MockReadyToHandleRandomnessTaskListener {
    async fn start(mut self) -> NodeResult<()> {
        let rpc_endpoint = self
            .chain_identity
            .read()
            .get_provider_rpc_endpoint()
            .to_string();

        let mut client = MockAdapterClient::new(rpc_endpoint, self.id_address.clone()).await?;

        loop {
            let is_bls_ready = self.group_cache.read().get_state();

            if let Ok(true) = is_bls_ready {
                let current_group_index = self.group_cache.read().get_index()?;

                let current_block_height = self.block_cache.read().get_block_height();

                let available_tasks = self
                    .randomness_tasks_cache
                    .write()
                    .check_and_get_available_tasks(current_block_height, current_group_index);

                let mut tasks_to_process: Vec<RandomnessTask> = vec![];

                for task in available_tasks {
                    if let Ok(false) = client.get_signature_task_completion_state(task.index).await
                    {
                        tasks_to_process.push(task);
                    }
                }

                if !tasks_to_process.is_empty() {
                    self.publish(ReadyToHandleRandomnessTask {
                        chain_id: self.chain_id,
                        tasks: tasks_to_process,
                    });
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
