use crate::ExponentialBackoffRetryDescriptor;
use alloy::{
    primitives::{Address, BlockNumber},
    signers::local::PrivateKeySigner,
    transports::TransportError,
};
use async_trait::async_trait;

mod gas_filler;
pub use gas_filler::*;
mod types;
pub use types::*;

#[async_trait]
pub trait ChainIdentity {
    fn get_chain_id(&self) -> u64;

    fn get_id_address(&self) -> Address;

    fn get_adapter_address(&self) -> Address;

    fn get_signer(&self) -> &PrivateKeySigner;

    fn get_client(&self) -> ProviderClientWithSigner;

    fn get_contract_transaction_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor;

    fn get_contract_view_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor;

    fn get_max_priority_fee_per_gas(&self) -> Option<u128>;

    async fn get_current_gas_price(&self) -> Result<u128, TransportError>;

    async fn get_block_timestamp(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<u64>, TransportError>;
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
    fn get_provider(&self) -> &ProviderClientWithSigner;

    fn get_provider_endpoint(&self) -> &str;

    async fn reset_provider(&mut self) -> Result<(), TransportError>;
}
