use super::types::Listener;
use crate::node::{
    contract_client::adapter_client::{AdapterMockHelper, MockAdapterClient},
    dal::{
        api::BLSTasksFetcher,
        types::{ChainIdentity, GroupRelayConfirmationTask},
    },
    dal::{api::BLSTasksUpdater, cache::InMemoryBLSTasksQueue},
    error::errors::{NodeError, NodeResult},
    event::new_group_relay_confirmation_task::NewGroupRelayConfirmationTask,
    queue::event_queue::{EventPublisher, EventQueue},
};
use async_trait::async_trait;
use log::{error, info};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct MockNewGroupRelayConfirmationTaskListener {
    chain_id: usize,
    id_address: String,
    chain_identity: Arc<RwLock<ChainIdentity>>,
    group_relay_confirmation_tasks_cache:
        Arc<RwLock<InMemoryBLSTasksQueue<GroupRelayConfirmationTask>>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl MockNewGroupRelayConfirmationTaskListener {
    pub fn new(
        chain_id: usize,
        id_address: String,
        chain_identity: Arc<RwLock<ChainIdentity>>,
        group_relay_confirmation_tasks_cache: Arc<
            RwLock<InMemoryBLSTasksQueue<GroupRelayConfirmationTask>>,
        >,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockNewGroupRelayConfirmationTaskListener {
            chain_id,
            id_address,
            chain_identity,
            group_relay_confirmation_tasks_cache,
            eq,
        }
    }
}

impl EventPublisher<NewGroupRelayConfirmationTask> for MockNewGroupRelayConfirmationTaskListener {
    fn publish(&self, event: NewGroupRelayConfirmationTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl Listener for MockNewGroupRelayConfirmationTaskListener {
    async fn start(mut self) -> NodeResult<()> {
        let rpc_endpoint = self
            .chain_identity
            .read()
            .get_provider_rpc_endpoint()
            .to_string();

        let client = MockAdapterClient::new(rpc_endpoint, self.id_address.to_string());

        let retry_strategy = FixedInterval::from_millis(2000);

        loop {
            if let Err(err) = RetryIf::spawn(
                retry_strategy.clone(),
                || async {
                    let task_reply = client.emit_group_relay_confirmation_task().await;

                    if let Ok(group_relay_confirmation_task) = task_reply {
                        if let Ok(false) = self
                            .group_relay_confirmation_tasks_cache
                            .read()
                            .contains(group_relay_confirmation_task.index)
                        {
                            info!(
                                "received new group_relay_confirmation task. {:?}",
                                group_relay_confirmation_task
                            );

                            self.group_relay_confirmation_tasks_cache
                                .write()
                                .add(group_relay_confirmation_task.clone())?;

                            self.publish(NewGroupRelayConfirmationTask::new(
                                self.chain_id,
                                group_relay_confirmation_task,
                            ));
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

            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
        }
    }
}
