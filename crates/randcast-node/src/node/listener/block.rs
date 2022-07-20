use super::types::Listener;
use crate::node::{
    contract_client::adapter_client::{AdapterMockHelper, MockAdapterClient},
    dao::types::ChainIdentity,
    error::errors::NodeResult,
    event::new_block::NewBlock,
    queue::event_queue::{EventPublisher, EventQueue},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct MockBlockListener {
    chain_id: usize,
    id_address: String,
    chain_identity: Arc<RwLock<ChainIdentity>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl MockBlockListener {
    pub fn new(
        chain_id: usize,
        id_address: String,
        chain_identity: Arc<RwLock<ChainIdentity>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockBlockListener {
            chain_id,
            id_address,
            chain_identity,
            eq,
        }
    }
}

impl EventPublisher<NewBlock> for MockBlockListener {
    fn publish(&self, event: NewBlock) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl Listener for MockBlockListener {
    async fn start(mut self) -> NodeResult<()> {
        let rpc_endpoint = self
            .chain_identity
            .read()
            .get_provider_rpc_endpoint()
            .to_string();

        let mut client = MockAdapterClient::new(rpc_endpoint, self.id_address.to_string()).await?;

        loop {
            let block_height = client.mine(1).await?;

            self.publish(NewBlock {
                chain_id: self.chain_id,
                block_height,
            });

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
