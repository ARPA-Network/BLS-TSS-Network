use async_trait::async_trait;
use ethers_core::types::{Address, BlockNumber, U256};
use ethers_providers::{Provider, ProviderError, Ws};
use std::sync::Arc;

mod types;
pub use types::*;

use crate::ExponentialBackoffRetryDescriptor;

#[async_trait]
pub trait ChainIdentity {
    fn get_chain_id(&self) -> usize;

    fn get_id_address(&self) -> Address;

    fn get_adapter_address(&self) -> Address;

    fn get_signer(&self) -> Arc<WsWalletSigner>;

    fn get_contract_transaction_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor;

    fn get_contract_view_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor;

    async fn get_current_gas_price(&self) -> Result<U256, ProviderError>;

    async fn get_block_timestamp(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<U256>, ProviderError>;
}

pub trait MainChainIdentity: ChainIdentity {
    fn get_controller_address(&self) -> Address;

    fn get_controller_relayer_address(&self) -> Address;
}

pub trait RelayedChainIdentity: ChainIdentity {
    fn get_controller_oracle_address(&self) -> Address;
}

#[async_trait]
pub trait ChainProviderManager {
    fn get_provider(&self) -> &Provider<Ws>;

    fn get_provider_endpoint(&self) -> &str;

    async fn reset_provider(&mut self) -> Result<(), ProviderError>;
}
