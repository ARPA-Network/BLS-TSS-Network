use ethers_core::types::Address;
use ethers_providers::{Http, Provider};
use std::sync::Arc;

mod types;
pub use types::*;

pub trait ChainIdentity {
    fn get_chain_id(&self) -> usize;

    fn get_id_address(&self) -> Address;

    fn get_controller_address(&self) -> Address;

    fn get_adapter_address(&self) -> Address;

    fn get_provider(&self) -> Arc<Provider<Http>>;

    fn get_signer(&self) -> Arc<WalletSigner>;
}
