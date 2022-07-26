use super::Listener;
use crate::node::{
    error::{NodeError, NodeResult},
    event::dkg_success::DKGSuccess,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_contract_client::adapter::{AdapterClientBuilder, AdapterViews};
use arpa_node_core::{ChainIdentity, DKGStatus};
use arpa_node_dal::{GroupInfoFetcher, GroupInfoUpdater};
use async_trait::async_trait;
use log::error;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct PostCommitGroupingListener<
    G: GroupInfoFetcher + GroupInfoUpdater,
    I: ChainIdentity + AdapterClientBuilder,
> {
    main_chain_identity: Arc<RwLock<I>>,
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoFetcher + GroupInfoUpdater, I: ChainIdentity + AdapterClientBuilder>
    PostCommitGroupingListener<G, I>
{
    pub fn new(
        main_chain_identity: Arc<RwLock<I>>,
        group_cache: Arc<RwLock<G>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        PostCommitGroupingListener {
            main_chain_identity,
            group_cache,
            eq,
        }
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send,
        I: ChainIdentity + AdapterClientBuilder + Sync + Send,
    > EventPublisher<DKGSuccess> for PostCommitGroupingListener<G, I>
{
    async fn publish(&self, event: DKGSuccess) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send,
        I: ChainIdentity + AdapterClientBuilder + Sync + Send,
    > Listener for PostCommitGroupingListener<G, I>
{
    async fn start(mut self) -> NodeResult<()> {
        let id_address = self.main_chain_identity.read().await.get_id_address();

        let client = self
            .main_chain_identity
            .read()
            .await
            .build_adapter_client(id_address);

        let retry_strategy = FixedInterval::from_millis(2000);

        loop {
            if let Err(err) = RetryIf::spawn(
                retry_strategy.clone(),
                || async {
                    let dkg_status = self.group_cache.read().await.get_dkg_status();

                    if let Ok(DKGStatus::CommitSuccess) = dkg_status {
                        let group_index = self.group_cache.read().await.get_index()?;

                        let group_epoch = self.group_cache.read().await.get_epoch()?;

                        if let Ok(group) = client.get_group(group_index).await {
                            if group.state {
                                let res = self
                                    .group_cache
                                    .write()
                                    .await
                                    .update_dkg_status(
                                        group_index,
                                        group_epoch,
                                        DKGStatus::WaitForPostProcess,
                                    )
                                    .await?;

                                if res {
                                    self.publish(DKGSuccess { group }).await;
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
