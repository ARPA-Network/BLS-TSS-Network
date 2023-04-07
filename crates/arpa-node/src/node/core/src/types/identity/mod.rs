use ethers_core::types::Address;

mod types;
use ethers_signers::LocalWallet;
pub use types::*;

pub trait ChainIdentity {
    fn get_id(&self) -> usize;

    fn get_chain_id(&self) -> usize;

    fn get_id_address(&self) -> Address;

    fn get_provider_rpc_endpoint(&self) -> &str;

    fn get_controller_address(&self) -> Address;

    fn get_adapter_address(&self) -> Address;

    fn get_signer(&self) -> &LocalWallet;
}
