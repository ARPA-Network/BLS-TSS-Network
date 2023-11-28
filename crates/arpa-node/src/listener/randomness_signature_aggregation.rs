use super::Listener;
use crate::{
    error::NodeResult,
    event::ready_to_fulfill_randomness_task::ReadyToFulfillRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_dal::cache::RandomnessResultCache;
use arpa_dal::{BlockInfoHandler, GroupInfoHandler, SignatureResultCacheHandler};
use async_trait::async_trait;
use ethers::types::Address;
use log::info;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

pub struct RandomnessSignatureAggregationListener<PC: Curve> {
    chain_id: usize,
    id_address: Address,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    randomness_signature_cache:
        Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> RandomnessSignatureAggregationListener<PC> {
    pub fn new(
        chain_id: usize,
        id_address: Address,
        block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        randomness_signature_cache: Arc<
            RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>,
        >,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        RandomnessSignatureAggregationListener {
            chain_id,
            id_address,
            block_cache,
            group_cache,
            randomness_signature_cache,
            eq,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> EventPublisher<ReadyToFulfillRandomnessTask>
    for RandomnessSignatureAggregationListener<PC>
{
    async fn publish(&self, event: ReadyToFulfillRandomnessTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for RandomnessSignatureAggregationListener<PC> {
    async fn listen(&self) -> NodeResult<()> {
        let is_committer = self.group_cache.read().await.is_committer(self.id_address);

        if let Ok(true) = is_committer {
            let current_block_height = self.block_cache.read().await.get_block_height();

            let ready_signatures = self
                .randomness_signature_cache
                .write()
                .await
                .get_ready_to_commit_signatures(current_block_height)
                .await?;

            if !ready_signatures.is_empty() {
                self.publish(ReadyToFulfillRandomnessTask {
                    chain_id: self.chain_id,
                    tasks: ready_signatures,
                })
                .await;
            }
        }

        Ok(())
    }

    async fn handle_interruption(&self) -> NodeResult<()> {
        info!(
            "Handle interruption for RandomnessSignatureAggregationListener, chain_id:{}.",
            self.chain_id
        );

        Ok(())
    }
}
