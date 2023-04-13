use super::Listener;
use crate::node::{
    error::NodeResult,
    event::new_randomness_task::NewRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_contract_client::adapter::{AdapterClientBuilder, AdapterLogs};
use arpa_node_core::{ChainIdentity, RandomnessTask};
use arpa_node_dal::{BLSTasksFetcher, BLSTasksUpdater};
use async_trait::async_trait;
use ethers::types::Address;
use log::info;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct NewRandomnessTaskListener<
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
    I: ChainIdentity + AdapterClientBuilder,
> {
    chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
    randomness_tasks_cache: Arc<RwLock<T>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
        I: ChainIdentity + AdapterClientBuilder,
    > NewRandomnessTaskListener<T, I>
{
    pub fn new(
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<I>>,
        randomness_tasks_cache: Arc<RwLock<T>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        NewRandomnessTaskListener {
            chain_id,
            id_address,
            chain_identity,
            randomness_tasks_cache,
            eq,
        }
    }
}

#[async_trait]
impl<
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send,
        I: ChainIdentity + AdapterClientBuilder + Sync + Send,
    > EventPublisher<NewRandomnessTask> for NewRandomnessTaskListener<T, I>
{
    async fn publish(&self, event: NewRandomnessTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
        I: ChainIdentity + AdapterClientBuilder + Sync + Send,
    > Listener for NewRandomnessTaskListener<T, I>
{
    async fn listen(&self) -> NodeResult<()> {
        let client = self
            .chain_identity
            .read()
            .await
            .build_adapter_client(self.id_address);
        let chain_id = self.chain_id;

        client
            .subscribe_randomness_task(move |randomness_task| {
                let randomness_tasks_cache = self.randomness_tasks_cache.clone();
                let eq = self.eq.clone();

                async move {
                    let contained_res = randomness_tasks_cache
                        .read()
                        .await
                        .contains(&randomness_task.request_id)
                        .await;
                    if let Ok(false) = contained_res {
                        info!("received new randomness task. {:?}", randomness_task);

                        randomness_tasks_cache
                            .write()
                            .await
                            .add(randomness_task.clone())
                            .await
                            .map_err(anyhow::Error::from)?;

                        eq.read()
                            .await
                            .publish(NewRandomnessTask::new(chain_id, randomness_task))
                            .await;
                    }
                    Ok(())
                }
            })
            .await?;

        Ok(())
    }
}
