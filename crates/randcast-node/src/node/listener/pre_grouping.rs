use super::types::Listener;
use crate::node::{
    contract_client::controller_client::{ControllerMockHelper, MockControllerClient},
    dal::{api::GroupInfoFetcher, types::ChainIdentity},
    error::errors::{NodeError, NodeResult},
    event::new_dkg_task::NewDKGTask,
    queue::event_queue::{EventPublisher, EventQueue},
};
use async_trait::async_trait;
use log::error;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct MockPreGroupingListener<G: GroupInfoFetcher + Sync + Send> {
    main_chain_identity: Arc<RwLock<ChainIdentity>>,
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoFetcher + Sync + Send> MockPreGroupingListener<G> {
    pub fn new(
        main_chain_identity: Arc<RwLock<ChainIdentity>>,
        group_cache: Arc<RwLock<G>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockPreGroupingListener {
            main_chain_identity,
            group_cache,
            eq,
        }
    }
}

impl<G: GroupInfoFetcher + Sync + Send> EventPublisher<NewDKGTask> for MockPreGroupingListener<G> {
    fn publish(&self, event: NewDKGTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl<G: GroupInfoFetcher + Sync + Send> Listener for MockPreGroupingListener<G> {
    async fn start(mut self) -> NodeResult<()> {
        let rpc_endpoint = self
            .main_chain_identity
            .read()
            .get_provider_rpc_endpoint()
            .to_string();

        let id_address = self.main_chain_identity.read().get_id_address().to_string();

        let client = MockControllerClient::new(rpc_endpoint, id_address);

        let retry_strategy = FixedInterval::from_millis(1000);

        loop {
            if let Err(err) = RetryIf::spawn(
                retry_strategy.clone(),
                || async {
                    if let Ok(dkg_task) = client.emit_dkg_task().await {
                        if let Some((_, node_index)) =
                            dkg_task.members.iter().find(|(id_address, _)| {
                                **id_address == self.main_chain_identity.read().get_id_address()
                            })
                        {
                            let cache_index = self.group_cache.read().get_index().unwrap_or(0);

                            let cache_epoch = self.group_cache.read().get_epoch().unwrap_or(0);

                            if cache_index != dkg_task.group_index || cache_epoch != dkg_task.epoch
                            {
                                let self_index = *node_index;

                                self.publish(NewDKGTask {
                                    dkg_task,
                                    self_index,
                                });
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

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
