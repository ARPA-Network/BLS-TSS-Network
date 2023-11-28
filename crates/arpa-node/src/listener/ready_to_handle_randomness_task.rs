use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::ready_to_handle_randomness_task::ReadyToHandleRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::adapter::AdapterViews;
use arpa_core::RandomnessTask;
use arpa_dal::{BLSTasksHandler, BlockInfoHandler, GroupInfoHandler};
use async_trait::async_trait;
use ethers::{providers::Middleware, types::Address};
use log::info;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

pub struct ReadyToHandleRandomnessTaskListener<PC: Curve> {
    chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
    randomness_task_exclusive_window: usize,
}

impl<PC: Curve> ReadyToHandleRandomnessTaskListener<PC> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
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
impl<PC: Curve + Sync + Send> EventPublisher<ReadyToHandleRandomnessTask>
    for ReadyToHandleRandomnessTaskListener<PC>
{
    async fn publish(&self, event: ReadyToHandleRandomnessTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for ReadyToHandleRandomnessTaskListener<PC> {
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

    async fn handle_interruption(&self) -> NodeResult<()> {
        info!(
            "Handle interruption for ReadyToHandleRandomnessTaskListener, chain_id:{}.",
            self.chain_id
        );
        self.chain_identity
            .read()
            .await
            .get_provider()
            .get_net_version()
            .await?;

        Ok(())
    }
}
