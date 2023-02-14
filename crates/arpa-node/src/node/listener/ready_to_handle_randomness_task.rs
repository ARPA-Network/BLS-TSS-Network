use super::Listener;
use crate::node::{
    error::{NodeError, NodeResult},
    event::ready_to_handle_randomness_task::ReadyToHandleRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_contract_client::adapter::{AdapterClientBuilder, AdapterViews};
use arpa_node_core::{ChainIdentity, RandomnessTask};
use arpa_node_dal::{BLSTasksUpdater, BlockInfoFetcher, GroupInfoFetcher};
use async_trait::async_trait;
use ethers::types::Address;
use log::error;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct ReadyToHandleRandomnessTaskListener<
    B: BlockInfoFetcher,
    G: GroupInfoFetcher<C>,
    T: BLSTasksUpdater<RandomnessTask>,
    I: ChainIdentity + AdapterClientBuilder<C>,
    C: PairingCurve,
> {
    chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
    block_cache: Arc<RwLock<B>>,
    group_cache: Arc<RwLock<G>>,
    randomness_tasks_cache: Arc<RwLock<T>>,
    eq: Arc<RwLock<EventQueue>>,
    c: PhantomData<C>,
}

impl<
        B: BlockInfoFetcher,
        G: GroupInfoFetcher<C>,
        T: BLSTasksUpdater<RandomnessTask>,
        I: ChainIdentity + AdapterClientBuilder<C>,
        C: PairingCurve,
    > ReadyToHandleRandomnessTaskListener<B, G, T, I, C>
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
            c: PhantomData,
        }
    }
}

#[async_trait]
impl<
        B: BlockInfoFetcher + Sync + Send,
        G: GroupInfoFetcher<C> + Sync + Send,
        T: BLSTasksUpdater<RandomnessTask> + Sync + Send,
        I: ChainIdentity + AdapterClientBuilder<C> + Sync + Send,
        C: PairingCurve + Sync + Send,
    > EventPublisher<ReadyToHandleRandomnessTask>
    for ReadyToHandleRandomnessTaskListener<B, G, T, I, C>
{
    async fn publish(&self, event: ReadyToHandleRandomnessTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<
        B: BlockInfoFetcher + Sync + Send,
        G: GroupInfoFetcher<C> + Sync + Send,
        T: BLSTasksUpdater<RandomnessTask> + Sync + Send,
        I: ChainIdentity + AdapterClientBuilder<C> + Sync + Send,
        C: PairingCurve + Sync + Send,
    > Listener for ReadyToHandleRandomnessTaskListener<B, G, T, I, C>
{
    async fn start(mut self) -> NodeResult<()> {
        let client = self
            .chain_identity
            .read()
            .await
            .build_adapter_client(self.id_address);

        let retry_strategy = FixedInterval::from_millis(1000);

        loop {
            if let Err(err) = RetryIf::spawn(
                retry_strategy.clone(),
                || async {
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
                            )
                            .await?;

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
                            })
                            .await;
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
