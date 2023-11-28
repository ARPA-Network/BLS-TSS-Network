use super::Listener;
use crate::{
    error::NodeResult,
    event::dkg_post_process::DKGPostProcess,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_core::DKGStatus;
use arpa_dal::{BlockInfoHandler, GroupInfoHandler};
use async_trait::async_trait;
use log::info;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

pub struct PostGroupingListener<PC: Curve> {
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
    dkg_timeout_duration: usize,
}

impl<PC: Curve> PostGroupingListener<PC> {
    pub fn new(
        block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
        dkg_timeout_duration: usize,
    ) -> Self {
        PostGroupingListener {
            block_cache,
            group_cache,
            eq,
            pc: PhantomData,
            dkg_timeout_duration,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> EventPublisher<DKGPostProcess> for PostGroupingListener<PC> {
    async fn publish(&self, event: DKGPostProcess) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for PostGroupingListener<PC> {
    async fn listen(&self) -> NodeResult<()> {
        let dkg_status = self.group_cache.read().await.get_dkg_status();

        if let Ok(dkg_status) = dkg_status {
            match dkg_status {
                DKGStatus::None => {}
                DKGStatus::InPhase | DKGStatus::CommitSuccess | DKGStatus::WaitForPostProcess => {
                    let dkg_start_block_height =
                        self.group_cache.read().await.get_dkg_start_block_height()?;

                    let block_height = self.block_cache.read().await.get_block_height();

                    let dkg_timeout_block_height =
                        dkg_start_block_height + self.dkg_timeout_duration;

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

    async fn handle_interruption(&self) -> NodeResult<()> {
        info!("Handle interruption for PostGroupingListener");

        Ok(())
    }
}
