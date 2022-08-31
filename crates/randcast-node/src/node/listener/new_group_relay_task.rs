use super::Listener;
use crate::node::{
    contract_client::rpc_mock::controller::{ControllerMockHelper, MockControllerClient},
    dal::{cache::InMemoryBLSTasksQueue, types::GroupRelayTask},
    dal::{
        types::ChainIdentity,
        {BLSTasksFetcher, BLSTasksUpdater},
    },
    error::{NodeError, NodeResult},
    event::new_group_relay_task::NewGroupRelayTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use async_trait::async_trait;
use log::{error, info};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct MockNewGroupRelayTaskListener {
    main_chain_identity: Arc<RwLock<ChainIdentity>>,
    group_relay_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<GroupRelayTask>>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl MockNewGroupRelayTaskListener {
    pub fn new(
        main_chain_identity: Arc<RwLock<ChainIdentity>>,
        group_relay_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<GroupRelayTask>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockNewGroupRelayTaskListener {
            main_chain_identity,
            group_relay_tasks_cache,
            eq,
        }
    }
}

impl EventPublisher<NewGroupRelayTask> for MockNewGroupRelayTaskListener {
    fn publish(&self, event: NewGroupRelayTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl Listener for MockNewGroupRelayTaskListener {
    async fn start(mut self) -> NodeResult<()> {
        let rpc_endpoint = self
            .main_chain_identity
            .read()
            .get_provider_rpc_endpoint()
            .to_string();

        let id_address = self.main_chain_identity.read().get_id_address().to_string();

        let client = MockControllerClient::new(rpc_endpoint, id_address);

        let retry_strategy = FixedInterval::from_millis(2000);

        loop {
            if let Err(err) = RetryIf::spawn(
                retry_strategy.clone(),
                || async {
                    let task_reply = client.emit_group_relay_task().await;

                    if let Ok(group_relay_task) = task_reply {
                        if let Ok(false) = self
                            .group_relay_tasks_cache
                            .read()
                            .contains(group_relay_task.controller_global_epoch)
                        {
                            info!("received new group relay task. {:?}", group_relay_task);

                            self.group_relay_tasks_cache
                                .write()
                                .add(group_relay_task.clone())?;

                            self.publish(NewGroupRelayTask::new(group_relay_task));
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
