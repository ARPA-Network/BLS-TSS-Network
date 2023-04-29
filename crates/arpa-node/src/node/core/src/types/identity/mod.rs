use ethers_core::types::Address;
use ethers_providers::{Http, Provider};
use std::sync::Arc;

mod types;
pub use types::*;

use crate::ExponentialBackoffRetryDescriptor;

pub trait ChainIdentity {
    fn get_chain_id(&self) -> usize;

    fn get_id_address(&self) -> Address;

    fn get_controller_address(&self) -> Address;

    fn get_adapter_address(&self) -> Address;

    fn get_provider(&self) -> Arc<Provider<Http>>;

    fn get_signer(&self) -> Arc<WalletSigner>;

    fn get_contract_transaction_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor;

    fn get_contract_view_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor;
}
