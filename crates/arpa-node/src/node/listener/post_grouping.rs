use super::Listener;
use crate::node::{
    error::NodeResult,
    event::dkg_post_process::DKGPostProcess,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_core::DKGStatus;
use arpa_node_dal::{BlockInfoFetcher, GroupInfoFetcher, GroupInfoUpdater};
use async_trait::async_trait;
use log::info;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;

pub const DEFAULT_DKG_TIMEOUT_DURATION: usize = 10 * 4;

pub struct PostGroupingListener<
    B: BlockInfoFetcher,
    G: GroupInfoFetcher<C> + GroupInfoUpdater<C>,
    C: PairingCurve,
> {
    block_cache: Arc<RwLock<B>>,
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
    c: PhantomData<C>,
}

impl<B: BlockInfoFetcher, G: GroupInfoFetcher<C> + GroupInfoUpdater<C>, C: PairingCurve>
    PostGroupingListener<B, G, C>
{
    pub fn new(
        block_cache: Arc<RwLock<B>>,
        group_cache: Arc<RwLock<G>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        PostGroupingListener {
            block_cache,
            group_cache,
            eq,
            c: PhantomData,
        }
    }
}

#[async_trait]
impl<
        B: BlockInfoFetcher + Sync + Send,
        G: GroupInfoFetcher<C> + GroupInfoUpdater<C> + Sync + Send,
        C: PairingCurve + Sync + Send,
    > EventPublisher<DKGPostProcess> for PostGroupingListener<B, G, C>
{
    async fn publish(&self, event: DKGPostProcess) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<
        B: BlockInfoFetcher + Sync + Send,
        G: GroupInfoFetcher<C> + GroupInfoUpdater<C> + Sync + Send,
        C: PairingCurve + Sync + Send,
    > Listener for PostGroupingListener<B, G, C>
{
    async fn start(mut self) -> NodeResult<()> {
        loop {
            let dkg_status = self.group_cache.read().await.get_dkg_status();

            if let Ok(dkg_status) = dkg_status {
                match dkg_status {
                    DKGStatus::None => {}
                    DKGStatus::InPhase
                    | DKGStatus::CommitSuccess
                    | DKGStatus::WaitForPostProcess => {
                        let dkg_start_block_height =
                            self.group_cache.read().await.get_dkg_start_block_height()?;

                        let block_height = self.block_cache.read().await.get_block_height();

                        info!("dkg_start_block_height: {},current_block_height: {}, timeuout_dkg_block_height:{}",
                        dkg_start_block_height,block_height,dkg_start_block_height + DEFAULT_DKG_TIMEOUT_DURATION);

                        if block_height > dkg_start_block_height + DEFAULT_DKG_TIMEOUT_DURATION {
                            let group_index =
                                self.group_cache.read().await.get_index().unwrap_or(0);

                            let group_epoch =
                                self.group_cache.read().await.get_epoch().unwrap_or(0);

                            let res = self
                                .group_cache
                                .write()
                                .await
                                .update_dkg_status(group_index, group_epoch, DKGStatus::None)
                                .await?;

                            if res {
                                self.publish(DKGPostProcess {
                                    group_index,
                                    group_epoch,
                                })
                                .await;
                            }
                        }
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
