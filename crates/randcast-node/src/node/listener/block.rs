use super::Listener;
use crate::node::{
    contract_client::rpc_mock::adapter::{AdapterMockHelper, MockAdapterClient},
    dal::types::ChainIdentity,
    error::{NodeError, NodeResult},
    event::new_block::NewBlock,
    queue::{event_queue::EventQueue, EventPublisher},
};
use async_trait::async_trait;
use log::error;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_retry::{strategy::FixedInterval, RetryIf};

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

        let client = MockAdapterClient::new(rpc_endpoint.clone(), self.id_address.to_string());

        let retry_strategy = FixedInterval::from_millis(1000);

        loop {
            if let Err(err) = RetryIf::spawn(
                retry_strategy.clone(),
                || async {
                    let block_height = client.mine(1).await?;

                    self.publish(NewBlock {
                        chain_id: self.chain_id,
                        block_height,
                    });

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

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
