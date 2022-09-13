use super::Listener;
use crate::node::{
    contract_client::controller::{ControllerClientBuilder, ControllerLogs},
    dal::{ChainIdentity, GroupInfoFetcher},
    error::{NodeError, NodeResult},
    event::new_dkg_task::NewDKGTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use async_trait::async_trait;
use log::error;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct PreGroupingListener<G: GroupInfoFetcher, I: ChainIdentity + ControllerClientBuilder> {
    main_chain_identity: Arc<RwLock<I>>,
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoFetcher, I: ChainIdentity + ControllerClientBuilder> PreGroupingListener<G, I> {
    pub fn new(
        main_chain_identity: Arc<RwLock<I>>,
        group_cache: Arc<RwLock<G>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        PreGroupingListener {
            main_chain_identity,
            group_cache,
            eq,
        }
    }
}

impl<G: GroupInfoFetcher, I: ChainIdentity + ControllerClientBuilder> EventPublisher<NewDKGTask>
    for PreGroupingListener<G, I>
{
    fn publish(&self, event: NewDKGTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send + 'static,
        I: ChainIdentity + ControllerClientBuilder + Sync + Send,
    > Listener for PreGroupingListener<G, I>
{
    async fn start(mut self) -> NodeResult<()> {
        let client = self.main_chain_identity.read().build_controller_client();

        let retry_strategy = FixedInterval::from_millis(1000);

        if let Err(err) = RetryIf::spawn(
            retry_strategy.clone(),
            || async {
                let self_id_address = self.main_chain_identity.read().get_id_address();
                let group_cache = self.group_cache.clone();
                let eq = self.eq.clone();

                client
                    .subscribe_dkg_task(Box::new(move |dkg_task| {
                        if let Some((_, node_index)) = dkg_task
                            .members
                            .iter()
                            .find(|(id_address, _)| **id_address == self_id_address)
                        {
                            let cache_index = group_cache.read().get_index().unwrap_or(0);

                            let cache_epoch = group_cache.read().get_epoch().unwrap_or(0);

                            if cache_index != dkg_task.group_index || cache_epoch != dkg_task.epoch
                            {
                                let self_index = *node_index;

                                eq.read().publish(NewDKGTask {
                                    dkg_task,
                                    self_index,
                                });
                            }
                        }
                        Ok(())
                    }))
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
