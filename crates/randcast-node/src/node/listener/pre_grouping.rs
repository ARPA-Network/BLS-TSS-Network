use super::types::Listener;
use crate::node::{
    contract_client::controller_client::{ControllerMockHelper, MockControllerClient},
    dao::{api::GroupInfoFetcher, cache::InMemoryGroupInfoCache, types::ChainIdentity},
    error::errors::NodeResult,
    event::new_dkg_task::NewDKGTask,
    queue::event_queue::{EventPublisher, EventQueue},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct MockPreGroupingListener {
    main_chain_identity: Arc<RwLock<ChainIdentity>>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl MockPreGroupingListener {
    pub fn new(
        main_chain_identity: Arc<RwLock<ChainIdentity>>,
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockPreGroupingListener {
            main_chain_identity,
            group_cache,
            eq,
        }
    }
}

impl EventPublisher<NewDKGTask> for MockPreGroupingListener {
    fn publish(&self, event: NewDKGTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl Listener for MockPreGroupingListener {
    async fn start(mut self) -> NodeResult<()> {
        let rpc_endpoint = self
            .main_chain_identity
            .read()
            .get_provider_rpc_endpoint()
            .to_string();

        let id_address = self.main_chain_identity.read().get_id_address().to_string();

        let mut client = MockControllerClient::new(rpc_endpoint, id_address).await?;

        loop {
            if let Ok(dkg_task) = client.emit_dkg_task().await {
                if let Some((_, node_index)) = dkg_task.members.iter().find(|(id_address, _)| {
                    **id_address == self.main_chain_identity.read().get_id_address()
                }) {
                    let cache_index = self.group_cache.read().get_index().unwrap_or(0);

                    let cache_epoch = self.group_cache.read().get_epoch().unwrap_or(0);

                    if cache_index != dkg_task.group_index || cache_epoch != dkg_task.epoch {
                        let self_index = *node_index;

                        self.publish(NewDKGTask {
                            dkg_task,
                            self_index,
                        });
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
