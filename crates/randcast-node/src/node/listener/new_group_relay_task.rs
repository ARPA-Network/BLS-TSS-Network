use super::types::Listener;
use crate::node::{
    contract_client::controller_client::{ControllerMockHelper, MockControllerClient},
    dao::{
        api::{BLSTasksFetcher, BLSTasksUpdater},
        types::ChainIdentity,
    },
    dao::{cache::InMemoryBLSTasksQueue, types::GroupRelayTask},
    error::errors::NodeResult,
    event::new_group_relay_task::NewGroupRelayTask,
    queue::event_queue::{EventPublisher, EventQueue},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

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

        let mut client = MockControllerClient::new(rpc_endpoint, id_address).await?;

        loop {
            let task_reply = client.emit_group_relay_task().await;

            if let Ok(group_relay_task) = task_reply {
                if !self
                    .group_relay_tasks_cache
                    .read()
                    .contains(group_relay_task.controller_global_epoch)
                {
                    println!("received new group relay task. {:?}", group_relay_task);

                    self.group_relay_tasks_cache
                        .write()
                        .add(group_relay_task.clone())?;

                    self.publish(NewGroupRelayTask::new(group_relay_task));
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
        }
    }
}
