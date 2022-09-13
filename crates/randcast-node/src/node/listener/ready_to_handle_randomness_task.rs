use super::Listener;
use crate::node::{
    contract_client::adapter::{AdapterClientBuilder, AdapterViews},
    dal::ChainIdentity,
    dal::{
        types::RandomnessTask,
        {BLSTasksUpdater, BlockInfoFetcher, GroupInfoFetcher},
    },
    error::{NodeError, NodeResult},
    event::ready_to_handle_randomness_task::ReadyToHandleRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use async_trait::async_trait;
use ethers::types::Address;
use log::error;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct ReadyToHandleRandomnessTaskListener<
    B: BlockInfoFetcher,
    G: GroupInfoFetcher,
    T: BLSTasksUpdater<RandomnessTask>,
    I: ChainIdentity + AdapterClientBuilder,
> {
    chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
    block_cache: Arc<RwLock<B>>,
    group_cache: Arc<RwLock<G>>,
    randomness_tasks_cache: Arc<RwLock<T>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<
        B: BlockInfoFetcher,
        G: GroupInfoFetcher,
        T: BLSTasksUpdater<RandomnessTask>,
        I: ChainIdentity + AdapterClientBuilder,
    > ReadyToHandleRandomnessTaskListener<B, G, T, I>
{
    pub fn new(
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<I>>,
        block_cache: Arc<RwLock<B>>,
        group_cache: Arc<RwLock<G>>,
        randomness_tasks_cache: Arc<RwLock<T>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        ReadyToHandleRandomnessTaskListener {
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

impl<
        B: BlockInfoFetcher,
        G: GroupInfoFetcher,
        T: BLSTasksUpdater<RandomnessTask>,
        I: ChainIdentity + AdapterClientBuilder,
    > EventPublisher<ReadyToHandleRandomnessTask>
    for ReadyToHandleRandomnessTaskListener<B, G, T, I>
{
    fn publish(&self, event: ReadyToHandleRandomnessTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl<
        B: BlockInfoFetcher + Sync + Send,
        G: GroupInfoFetcher + Sync + Send,
        T: BLSTasksUpdater<RandomnessTask> + Sync + Send,
        I: ChainIdentity + AdapterClientBuilder + Sync + Send,
    > Listener for ReadyToHandleRandomnessTaskListener<B, G, T, I>
{
    async fn start(mut self) -> NodeResult<()> {
        let client = self
            .chain_identity
            .read()
            .build_adapter_client(self.id_address);

        let retry_strategy = FixedInterval::from_millis(1000);

        loop {
            if let Err(err) = RetryIf::spawn(
                retry_strategy.clone(),
                || async {
                    let is_bls_ready = self.group_cache.read().get_state();

                    if let Ok(true) = is_bls_ready {
                        let current_group_index = self.group_cache.read().get_index()?;

                        let current_block_height = self.block_cache.read().get_block_height();

                        let available_tasks = self
                            .randomness_tasks_cache
                            .write()
                            .check_and_get_available_tasks(
                                current_block_height,
                                current_group_index,
                            )?;

                        let mut tasks_to_process: Vec<RandomnessTask> = vec![];

                        for task in available_tasks {
                            if let Ok(false) =
                                client.get_signature_task_completion_state(task.index).await
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

                    NodeResult::Ok(())
                },
                |e: &NodeError| {
                    error!("listener is interrupted. Retry... Error: {:?}, ", e);
                    true
                },
            )
            .await
            {
                error!("{:?}", err);
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
