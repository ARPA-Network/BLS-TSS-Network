use super::Listener;
use crate::node::{
    error::{NodeError, NodeResult},
    event::new_randomness_task::NewRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_contract_client::adapter::{AdapterClientBuilder, AdapterLogs};
use arpa_node_core::{ChainIdentity, RandomnessTask};
use arpa_node_dal::{BLSTasksFetcher, BLSTasksUpdater};
use async_trait::async_trait;
use ethers::types::Address;
use log::{error, info};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_retry::{strategy::FixedInterval, RetryIf};

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
    async fn start(mut self) -> NodeResult<()> {
        let client = self
            .chain_identity
            .read()
            .await
            .build_adapter_client(self.id_address);

        let retry_strategy = FixedInterval::from_millis(2000);

        if let Err(err) = RetryIf::spawn(
            retry_strategy.clone(),
            || async {
                let chain_id = self.chain_id;
                let randomness_tasks_cache = self.randomness_tasks_cache.clone();
                let eq = self.eq.clone();

                client
                    .subscribe_randomness_task(move |randomness_task| {
                        let randomness_tasks_cache = randomness_tasks_cache.clone();
                        let eq = eq.clone();

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

        Ok(())
    }
}
