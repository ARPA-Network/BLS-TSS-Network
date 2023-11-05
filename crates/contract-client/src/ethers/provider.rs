use crate::{
    error::{ContractClientError, ContractClientResult},
    provider::BlockFetcher,
};
use async_trait::async_trait;
use ethers::prelude::*;
use std::future::Future;

#[async_trait]
impl BlockFetcher for Provider<Ws> {
    async fn subscribe_new_block_height<
        C: FnMut(usize) -> F + Send,
        F: Future<Output = ContractClientResult<()>> + Send,
    >(
        &self,
        mut cb: C,
    ) -> ContractClientResult<()> {
        let mut stream = self.subscribe_blocks().await?;
        while let Some(block) = stream.next().await {
            cb(block
                .number
                .ok_or(ContractClientError::FetchingBlockError)?
                .as_usize())
            .await?;
        }
        Err(ContractClientError::FetchingBlockError)
    }
}
