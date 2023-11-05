use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::new_block::NewBlock,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::provider::BlockFetcher;
use async_trait::async_trait;
use log::info;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

pub struct BlockListener<PC: Curve> {
    chain_id: usize,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> BlockListener<PC> {
    pub fn new(
        chain_id: usize,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        BlockListener {
            chain_id,
            chain_identity,
            eq,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> EventPublisher<NewBlock> for BlockListener<PC> {
    async fn publish(&self, event: NewBlock) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for BlockListener<PC> {
    async fn listen(&self) -> NodeResult<()> {
        let chain_id = self.chain_id;
        let eq = self.eq.clone();

        self.chain_identity
            .read()
            .await
            .get_provider()
            .subscribe_new_block_height(move |block_height: usize| {
                let eq = eq.clone();
                async move {
                    eq.read()
                        .await
                        .publish(NewBlock {
                            chain_id,
                            block_height,
                        })
                        .await;

                    Ok(())
                }
            })
            .await?;

        Ok(())
    }

    async fn handle_interruption(&self) -> NodeResult<()> {
        info!(
            "Handle interruption for BlockListener, chain_id:{}.",
            self.chain_id
        );
        self.chain_identity.write().await.reset_provider().await?;

        Ok(())
    }
}
