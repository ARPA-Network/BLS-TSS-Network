use crate::{
    error::{ContractClientError, ContractClientResult},
    provider::{BlockFetcher, ChainProviderBuilder},
};
use arpa_core::{ChainIdentity, GeneralMainChainIdentity, GeneralRelayedChainIdentity};
use async_trait::async_trait;
use ethers::prelude::*;
use ethers::providers::Http as HttpProvider;
use std::{future::Future, sync::Arc};

pub struct ChainProvider {
    provider: Arc<Provider<HttpProvider>>,
}

impl ChainProvider {
    pub fn new(provider: Arc<Provider<HttpProvider>>) -> Self {
        ChainProvider { provider }
    }
}

impl ChainProviderBuilder for GeneralMainChainIdentity {
    type ProviderService = ChainProvider;

    fn build_chain_provider(&self) -> ChainProvider {
        ChainProvider::new(self.get_provider())
    }
}

impl ChainProviderBuilder for GeneralRelayedChainIdentity {
    type ProviderService = ChainProvider;

    fn build_chain_provider(&self) -> ChainProvider {
        ChainProvider::new(self.get_provider())
    }
}

#[async_trait]
impl BlockFetcher for ChainProvider {
    async fn subscribe_new_block_height<
        C: FnMut(usize) -> F + Send,
        F: Future<Output = ContractClientResult<()>> + Send,
    >(
        &self,
        mut cb: C,
    ) -> ContractClientResult<()> {
        let mut stream = self.provider.watch_blocks().await?;
        while let Some(block_hash) = stream.next().await {
            let block = self
                .provider
                .get_block(block_hash)
                .await?
                .ok_or(ContractClientError::FetchingBlockError)?;
            cb(block
                .number
                .ok_or(ContractClientError::FetchingBlockError)?
                .as_usize())
            .await?;
        }
        Err(ContractClientError::FetchingBlockError)
    }
}
