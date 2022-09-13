use crate::node::{
    contract_client::provider::{BlockFetcher, ChainProviderBuilder},
    dal::{types::GeneralChainIdentity, ChainIdentity},
    error::{ContractClientError, NodeResult},
};
use async_trait::async_trait;
use ethers::prelude::*;
use ethers::providers::Http as HttpProvider;
use std::{convert::TryFrom, time::Duration};

pub struct ChainProvider {
    provider: Provider<HttpProvider>,
}

impl ChainProvider {
    pub fn new(identity: &GeneralChainIdentity) -> Self {
        let provider = Provider::<Http>::try_from(identity.get_provider_rpc_endpoint())
            .unwrap()
            .interval(Duration::from_millis(10u64));

        ChainProvider { provider }
    }
}

impl ChainProviderBuilder for GeneralChainIdentity {
    type Service = ChainProvider;

    fn build_chain_provider(&self) -> ChainProvider {
        ChainProvider::new(self)
    }
}

#[async_trait]
impl BlockFetcher for ChainProvider {
    async fn subscribe_new_block_height(
        &self,
        cb: Box<dyn Fn(usize) -> NodeResult<()> + Sync + Send>,
    ) -> NodeResult<()> {
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
                .as_usize())?;
        }
        Err(ContractClientError::FetchingBlockError.into())
    }
}
