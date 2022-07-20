use super::types::Listener;
use crate::node::{
    contract_client::controller_client::{ControllerViews, MockControllerClient},
    dao::{
        api::{GroupInfoFetcher, GroupInfoUpdater},
        cache::InMemoryGroupInfoCache,
        types::{ChainIdentity, DKGStatus},
    },
    error::errors::NodeResult,
    event::dkg_success::DKGSuccess,
    queue::event_queue::{EventPublisher, EventQueue},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct MockPostCommitGroupingListener {
    main_chain_identity: Arc<RwLock<ChainIdentity>>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl MockPostCommitGroupingListener {
    pub fn new(
        main_chain_identity: Arc<RwLock<ChainIdentity>>,
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockPostCommitGroupingListener {
            main_chain_identity,
            group_cache,
            eq,
        }
    }
}

impl EventPublisher<DKGSuccess> for MockPostCommitGroupingListener {
    fn publish(&self, event: DKGSuccess) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl Listener for MockPostCommitGroupingListener {
    async fn start(mut self) -> NodeResult<()> {
        let rpc_endpoint = self
            .main_chain_identity
            .read()
            .get_provider_rpc_endpoint()
            .to_string();

        let id_address = self.main_chain_identity.read().get_id_address().to_string();

        let mut client = MockControllerClient::new(rpc_endpoint, id_address).await?;

        loop {
            let dkg_status = self.group_cache.read().get_dkg_status();

            if let Ok(DKGStatus::CommitSuccess) = dkg_status {
                let group_index = self.group_cache.read().get_index()?;

                let group_epoch = self.group_cache.read().get_epoch()?;

                if let Ok(group) = client.get_group(group_index).await {
                    if group.state {
                        let res = self.group_cache.write().update_dkg_status(
                            group_index,
                            group_epoch,
                            DKGStatus::WaitForPostProcess,
                        )?;

                        if res {
                            self.publish(DKGSuccess { group });
                        }
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
