use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::new_block::NewBlock,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::provider::BlockFetcher;
use arpa_core::ListenerDescriptor;
use async_trait::async_trait;
use log::debug;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct BlockListener<PC: Curve> {
    listener_descriptor: ListenerDescriptor,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> std::fmt::Display for BlockListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlockListener")
    }
}

impl<PC: Curve> BlockListener<PC> {
    pub fn new(
        listener_descriptor: ListenerDescriptor,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        BlockListener {
            listener_descriptor,
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
        let chain_id = self.listener_descriptor.chain_id;
        let eq = self.eq.clone();

        let provider = self.chain_identity.read().await.get_provider().clone();

        provider
            .subscribe_new_block_height(move |block_height: usize| {
                debug!("New block height: {} for chain {}", block_height, chain_id);

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
        self.chain_identity.write().await.reset_provider().await?;

        Ok(())
    }

    fn chain_id(&self) -> usize {
        self.listener_descriptor.chain_id
    }

    fn listener_descriptor(&self) -> ListenerDescriptor {
        self.listener_descriptor
    }
}
