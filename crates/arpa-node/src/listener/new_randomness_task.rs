use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::new_randomness_task::NewRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::adapter::AdapterLogs;
use arpa_core::RandomnessTask;
use arpa_dal::BLSTasksHandler;
use async_trait::async_trait;
use ethers::{providers::Middleware, types::Address};
use log::info;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

pub struct NewRandomnessTaskListener<PC: Curve> {
    chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> NewRandomnessTaskListener<PC> {
    pub fn new(
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        NewRandomnessTaskListener {
            chain_id,
            id_address,
            chain_identity,
            randomness_tasks_cache,
            eq,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> EventPublisher<NewRandomnessTask> for NewRandomnessTaskListener<PC> {
    async fn publish(&self, event: NewRandomnessTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for NewRandomnessTaskListener<PC> {
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

    async fn handle_interruption(&self) -> NodeResult<()> {
        info!(
            "Handle interruption for NewRandomnessTaskListener, chain_id:{}.",
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
