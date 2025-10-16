use crate::{
    error::{ContractClientError, ContractClientResult},
    provider::BlockFetcher,
};
use alloy::providers::Provider;
use arpa_core::ProviderClientWithSigner;
use async_trait::async_trait;
use futures_util::StreamExt;
use std::future::Future;

#[async_trait]
impl BlockFetcher for ProviderClientWithSigner {
    async fn subscribe_new_block_height<
        C: FnMut(usize) -> F + Send,
        F: Future<Output = ContractClientResult<()>> + Send,
    >(
        &self,
        mut cb: C,
    ) -> ContractClientResult<()> {
        let mut stream = self.subscribe_blocks().await?.into_stream();

        while let Some(block) = stream.next().await {
            cb(block.number as usize).await?;
        }
        Err(ContractClientError::FetchingBlockError)
    }
}
