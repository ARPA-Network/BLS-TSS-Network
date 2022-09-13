use super::Listener;
use crate::node::{
    dal::{cache::InMemoryBlockInfoCache, types::DKGStatus},
    dal::{BlockInfoFetcher, GroupInfoFetcher, GroupInfoUpdater},
    error::NodeResult,
    event::dkg_post_process::DKGPostProcess,
    queue::{event_queue::EventQueue, EventPublisher},
};
use async_trait::async_trait;
use log::info;
use parking_lot::RwLock;
use std::sync::Arc;

pub const DEFAULT_DKG_TIMEOUT_DURATION: usize = 10 * 4;

pub struct MockPostGroupingListener<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> {
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> MockPostGroupingListener<G> {
    pub fn new(
        block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
        group_cache: Arc<RwLock<G>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockPostGroupingListener {
            block_cache,
            group_cache,
            eq,
        }
    }
}

impl<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> EventPublisher<DKGPostProcess>
    for MockPostGroupingListener<G>
{
    fn publish(&self, event: DKGPostProcess) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> Listener
    for MockPostGroupingListener<G>
{
    async fn start(mut self) -> NodeResult<()> {
        loop {
            let dkg_status = self.group_cache.read().get_dkg_status();

            if let Ok(dkg_status) = dkg_status {
                match dkg_status {
                    DKGStatus::None => {}
                    DKGStatus::InPhase
                    | DKGStatus::CommitSuccess
                    | DKGStatus::WaitForPostProcess => {
                        let dkg_start_block_height =
                            self.group_cache.read().get_dkg_start_block_height()?;

                        let block_height = self.block_cache.read().get_block_height();

                        info!("dkg_start_block_height: {},current_block_height: {}, timeuout_dkg_block_height:{}",
                        dkg_start_block_height,block_height,dkg_start_block_height + DEFAULT_DKG_TIMEOUT_DURATION);

                        if block_height > dkg_start_block_height + DEFAULT_DKG_TIMEOUT_DURATION {
                            let group_index = self.group_cache.read().get_index().unwrap_or(0);

                            let group_epoch = self.group_cache.read().get_epoch().unwrap_or(0);

                            let res = self.group_cache.write().update_dkg_status(
                                group_index,
                                group_epoch,
                                DKGStatus::None,
                            )?;

                            if res {
                                self.publish(DKGPostProcess {
                                    group_index,
                                    group_epoch,
                                });
                            }
                        }
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
