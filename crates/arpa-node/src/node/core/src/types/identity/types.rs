use ethers_core::types::Address;
use ethers_signers::{LocalWallet, Signer};

use super::ChainIdentity;

#[derive(Debug, Clone)]
pub struct GeneralChainIdentity {
    id: usize,
    chain_id: usize,
    wallet: LocalWallet,
    provider_rpc_endpoint: String,
    controller_address: Address,
    adapter_address: Address,
}

impl GeneralChainIdentity {
    pub fn new(
        id: usize,
        chain_id: usize,
        wallet: LocalWallet,
        provider_rpc_endpoint: String,
        controller_address: Address,
        adapter_address: Address,
    ) -> Self {
        GeneralChainIdentity {
            id,
            chain_id,
            wallet,
            provider_rpc_endpoint,
            controller_address,
            adapter_address,
        }
    }
}

impl ChainIdentity for GeneralChainIdentity {
    fn get_id(&self) -> usize {
        self.id
    }

    fn get_chain_id(&self) -> usize {
        self.chain_id
    }

    fn get_id_address(&self) -> Address {
        self.wallet.address()
    }

    fn get_provider_rpc_endpoint(&self) -> &str {
        &self.provider_rpc_endpoint
    }

    fn get_controller_address(&self) -> Address {
        self.controller_address
    }

    fn get_adapter_address(&self) -> Address {
        self.adapter_address
    }

    fn get_signer(&self) -> &LocalWallet {
        &self.wallet
    }
}
