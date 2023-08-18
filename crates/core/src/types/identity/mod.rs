use async_trait::async_trait;
use ethers_core::types::{Address, U256};
use ethers_providers::{Http, Provider, ProviderError};
use std::sync::Arc;

mod types;
pub use types::*;

use crate::ExponentialBackoffRetryDescriptor;

#[async_trait]
pub trait ChainIdentity {
    fn get_chain_id(&self) -> usize;

    fn get_id_address(&self) -> Address;

    fn get_adapter_address(&self) -> Address;

    fn get_provider(&self) -> Arc<Provider<Http>>;

    fn get_signer(&self) -> Arc<WalletSigner>;

    fn get_contract_transaction_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor;

    fn get_contract_view_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor;

    async fn get_current_gas_price(&self) -> Result<U256, ProviderError>;
}

pub trait MainChainIdentity: ChainIdentity {
    fn get_controller_address(&self) -> Address;

    fn get_controller_relayer_address(&self) -> Address;
}

pub trait RelayedChainIdentity: ChainIdentity {
    fn get_controller_oracle_address(&self) -> Address;
}
