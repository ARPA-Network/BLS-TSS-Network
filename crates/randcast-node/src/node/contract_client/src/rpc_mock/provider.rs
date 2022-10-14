use std::future::Future;

use super::adapter::{AdapterMockHelper, MockAdapterClient};
use crate::{
    error::ContractClientResult,
    provider::{BlockFetcher, ChainProviderBuilder},
};
use arpa_node_core::{ChainIdentity, MockChainIdentity, PALCEHOLDER_ADDRESS};
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
    async fn subscribe_new_block_height<
        C: FnMut(usize) -> F + Send,
        F: Future<Output = ContractClientResult<()>> + Send,
    >(
        &self,
        mut cb: C,
    ) -> ContractClientResult<()> {
        loop {
            let block_height = self.mine(1).await?;
            cb(block_height).await?;
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}
