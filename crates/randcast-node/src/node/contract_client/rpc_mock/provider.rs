use super::adapter::{AdapterMockHelper, MockAdapterClient};
use crate::node::{
    contract_client::provider::{BlockFetcher, ChainProviderBuilder},
    dal::{types::MockChainIdentity, ChainIdentity},
    error::NodeResult,
    PALCEHOLDER_ADDRESS,
};
use async_trait::async_trait;

impl ChainProviderBuilder for MockChainIdentity {
    type Service = MockAdapterClient;

    fn build_chain_provider(&self) -> MockAdapterClient {
        MockAdapterClient::new(
            self.get_provider_rpc_endpoint().to_string(),
            PALCEHOLDER_ADDRESS,
        )
    }
}

#[async_trait]
impl BlockFetcher for MockAdapterClient {
    async fn subscribe_new_block_height(
        &self,
        cb: Box<dyn Fn(usize) -> NodeResult<()> + Sync + Send>,
    ) -> NodeResult<()> {
        loop {
            let block_height = self.mine(1).await?;
            cb(block_height)?;
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
