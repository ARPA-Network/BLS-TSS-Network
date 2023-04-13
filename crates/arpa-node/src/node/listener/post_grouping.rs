use super::Listener;
use crate::node::{
    error::NodeResult,
    event::dkg_post_process::DKGPostProcess,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_core::{DKGStatus, CONFIG};
use arpa_node_dal::{BlockInfoFetcher, GroupInfoFetcher};
use async_trait::async_trait;
use log::info;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;

pub struct PostGroupingListener<B: BlockInfoFetcher, G: GroupInfoFetcher<PC>, PC: PairingCurve> {
    block_cache: Arc<RwLock<B>>,
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<B: BlockInfoFetcher, G: GroupInfoFetcher<PC>, PC: PairingCurve>
    PostGroupingListener<B, G, PC>
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
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<
        B: BlockInfoFetcher + Sync + Send,
        G: GroupInfoFetcher<PC> + Sync + Send,
        PC: PairingCurve + Sync + Send,
    > EventPublisher<DKGPostProcess> for PostGroupingListener<B, G, PC>
{
    async fn publish(&self, event: DKGPostProcess) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<
        B: BlockInfoFetcher + Sync + Send,
        G: GroupInfoFetcher<PC> + Sync + Send,
        PC: PairingCurve + Sync + Send,
    > Listener for PostGroupingListener<B, G, PC>
{
    async fn listen(&self) -> NodeResult<()> {
        let dkg_status = self.group_cache.read().await.get_dkg_status();

        if let Ok(dkg_status) = dkg_status {
            match dkg_status {
                DKGStatus::None => {}
                DKGStatus::InPhase | DKGStatus::CommitSuccess | DKGStatus::WaitForPostProcess => {
                    let dkg_start_block_height =
                        self.group_cache.read().await.get_dkg_start_block_height()?;

                    let block_height = self.block_cache.read().await.get_block_height();

                    let dkg_timeout_block_height = dkg_start_block_height
                        + CONFIG
                            .get()
                            .unwrap()
                            .time_limits
                            .unwrap()
                            .dkg_timeout_duration;

                    info!("checking post process... dkg_start_block_height: {}, current_block_height: {}, timeuout_dkg_block_height: {}",
                    dkg_start_block_height,block_height,dkg_timeout_block_height);

                    if block_height > dkg_timeout_block_height {
                        let group_index = self.group_cache.read().await.get_index().unwrap_or(0);

                        let group_epoch = self.group_cache.read().await.get_epoch().unwrap_or(0);

                        self.publish(DKGPostProcess {
                            group_index,
                            group_epoch,
                        })
                        .await;
                    }
                }
            }
        }

        Ok(())
    }
}
