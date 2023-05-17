use super::Listener;
use crate::node::{
    error::NodeResult,
    event::ready_to_handle_randomness_task::ReadyToHandleRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_contract_client::adapter::{AdapterClientBuilder, AdapterViews};
use arpa_node_core::{ChainIdentity, RandomnessTask};
use arpa_node_dal::{BLSTasksUpdater, BlockInfoFetcher, GroupInfoFetcher};
use async_trait::async_trait;
use ethers::types::Address;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;

pub struct ReadyToHandleRandomnessTaskListener<
    B: BlockInfoFetcher,
    G: GroupInfoFetcher<PC>,
    T: BLSTasksUpdater<RandomnessTask>,
    I: ChainIdentity + AdapterClientBuilder,
    PC: PairingCurve,
> {
    chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
    block_cache: Arc<RwLock<B>>,
    group_cache: Arc<RwLock<G>>,
    randomness_tasks_cache: Arc<RwLock<T>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
    randomness_task_exclusive_window: usize,
}

impl<
        B: BlockInfoFetcher,
        G: GroupInfoFetcher<PC>,
        T: BLSTasksUpdater<RandomnessTask>,
        I: ChainIdentity + AdapterClientBuilder,
        PC: PairingCurve,
    > ReadyToHandleRandomnessTaskListener<B, G, T, I, PC>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<I>>,
        block_cache: Arc<RwLock<B>>,
        group_cache: Arc<RwLock<G>>,
        randomness_tasks_cache: Arc<RwLock<T>>,
        eq: Arc<RwLock<EventQueue>>,
        randomness_task_exclusive_window: usize,
    ) -> Self {
        ReadyToHandleRandomnessTaskListener {
            chain_id,
            id_address,
            chain_identity,
            block_cache,
            group_cache,
            randomness_tasks_cache,
            eq,
            pc: PhantomData,
            randomness_task_exclusive_window,
        }
    }
}

#[async_trait]
impl<
        B: BlockInfoFetcher + Sync + Send,
        G: GroupInfoFetcher<PC> + Sync + Send,
        T: BLSTasksUpdater<RandomnessTask> + Sync + Send,
        I: ChainIdentity + AdapterClientBuilder + Sync + Send,
        PC: PairingCurve + Sync + Send,
    > EventPublisher<ReadyToHandleRandomnessTask>
    for ReadyToHandleRandomnessTaskListener<B, G, T, I, PC>
{
    async fn publish(&self, event: ReadyToHandleRandomnessTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<
        B: BlockInfoFetcher + Sync + Send,
        G: GroupInfoFetcher<PC> + Sync + Send,
        T: BLSTasksUpdater<RandomnessTask> + Sync + Send,
        I: ChainIdentity + AdapterClientBuilder + Sync + Send,
        PC: PairingCurve + Sync + Send,
    > Listener for ReadyToHandleRandomnessTaskListener<B, G, T, I, PC>
{
    async fn listen(&self) -> NodeResult<()> {
        let is_bls_ready = self.group_cache.read().await.get_state();

        if let Ok(true) = is_bls_ready {
            let current_group_index = self.group_cache.read().await.get_index()?;

            let current_block_height = self.block_cache.read().await.get_block_height();

            let available_tasks = self
                .randomness_tasks_cache
                .write()
                .await
                .check_and_get_available_tasks(
                    current_block_height,
                    current_group_index,
                    self.randomness_task_exclusive_window,
                )
                .await?;

            if available_tasks.is_empty() {
                return Ok(());
            }

            let mut tasks_to_process: Vec<RandomnessTask> = vec![];

            let client = self
                .chain_identity
                .read()
                .await
                .build_adapter_client(self.id_address);

            for task in available_tasks {
                if let Ok(true) = client.is_task_pending(&task.request_id).await {
                    tasks_to_process.push(task);
                }
            }

            if !tasks_to_process.is_empty() {
                self.publish(ReadyToHandleRandomnessTask {
                    chain_id: self.chain_id,
                    tasks: tasks_to_process,
                })
                .await;
            }
        }

        Ok(())
    }
}
