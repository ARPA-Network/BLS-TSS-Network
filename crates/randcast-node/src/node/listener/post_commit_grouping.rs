use super::Listener;
use crate::node::{
    contract_client::{controller::ControllerViews, rpc_mock::controller::MockControllerClient},
    dal::{
        types::{ChainIdentity, DKGStatus},
        {GroupInfoFetcher, GroupInfoUpdater},
    },
    error::{NodeError, NodeResult},
    event::dkg_success::DKGSuccess,
    queue::{event_queue::EventQueue, EventPublisher},
};
use async_trait::async_trait;
use log::error;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct MockPostCommitGroupingListener<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> {
    main_chain_identity: Arc<RwLock<ChainIdentity>>,
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> MockPostCommitGroupingListener<G> {
    pub fn new(
        main_chain_identity: Arc<RwLock<ChainIdentity>>,
        group_cache: Arc<RwLock<G>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockPostCommitGroupingListener {
            main_chain_identity,
            group_cache,
            eq,
        }
    }
}

impl<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> EventPublisher<DKGSuccess>
    for MockPostCommitGroupingListener<G>
{
    fn publish(&self, event: DKGSuccess) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> Listener
    for MockPostCommitGroupingListener<G>
{
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
