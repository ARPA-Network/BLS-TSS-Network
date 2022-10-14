use super::Listener;
use crate::node::{
    error::{NodeError, NodeResult},
    event::new_dkg_task::NewDKGTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_contract_client::controller::{ControllerClientBuilder, ControllerLogs};
use arpa_node_core::ChainIdentity;
use arpa_node_dal::GroupInfoFetcher;
use async_trait::async_trait;
use log::error;
use std::sync::Arc;
use tokio::sync::RwLock;
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

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send,
        I: ChainIdentity + ControllerClientBuilder + Sync + Send,
    > EventPublisher<NewDKGTask> for PreGroupingListener<G, I>
{
    async fn publish(&self, event: NewDKGTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send + 'static,
        I: ChainIdentity + ControllerClientBuilder + Sync + Send,
    > Listener for PreGroupingListener<G, I>
{
    async fn start(mut self) -> NodeResult<()> {
        let client = self
            .main_chain_identity
            .read()
            .await
            .build_controller_client();

        let retry_strategy = FixedInterval::from_millis(1000);

        if let Err(err) = RetryIf::spawn(
            retry_strategy.clone(),
            || async {
                let self_id_address = self.main_chain_identity.read().await.get_id_address();
                let group_cache = self.group_cache.clone();
                let eq = self.eq.clone();

                client
                    .subscribe_dkg_task(move |dkg_task| {
                        let group_cache = group_cache.clone();
                        let eq = eq.clone();

                        async move {
                            if let Some((_, node_index)) = dkg_task
                                .members
                                .iter()
                                .find(|(id_address, _)| **id_address == self_id_address)
                            {
                                let cache_index = group_cache.read().await.get_index().unwrap_or(0);

                                let cache_epoch = group_cache.read().await.get_epoch().unwrap_or(0);

                                if cache_index != dkg_task.group_index
                                    || cache_epoch != dkg_task.epoch
                                {
                                    let self_index = *node_index;

                                    eq.read()
                                        .await
                                        .publish(NewDKGTask {
                                            dkg_task,
                                            self_index,
                                        })
                                        .await;
                                }
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
