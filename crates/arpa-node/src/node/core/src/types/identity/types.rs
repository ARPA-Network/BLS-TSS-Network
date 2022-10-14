use ethers_core::{rand, types::Address};
use ethers_signers::{LocalWallet, Signer};

use super::ChainIdentity;

#[derive(Clone)]
pub struct MockChainIdentity {
    id: usize,
    chain_id: usize,
    id_address: Address,
    provider_rpc_endpoint: String,
    mock_signer: LocalWallet,
}

impl MockChainIdentity {
    pub fn new(
        id: usize,
        chain_id: usize,
        id_address: Address,
        provider_rpc_endpoint: String,
    ) -> Self {
        let mut rng = rand::thread_rng();
        let mock_signer = LocalWallet::new(&mut rng);

        MockChainIdentity {
            id,
            chain_id,
            id_address,
            provider_rpc_endpoint,
            mock_signer,
        }
    }
}

impl ChainIdentity for MockChainIdentity {
    fn get_id(&self) -> usize {
        self.id
    }

    fn get_chain_id(&self) -> usize {
        self.chain_id
    }

    fn get_id_address(&self) -> Address {
        self.id_address
    }

    fn get_provider_rpc_endpoint(&self) -> &str {
        &self.provider_rpc_endpoint
    }

    fn get_contract_address(&self) -> Address {
        Address::random()
    }

    fn get_signer(&self) -> &LocalWallet {
        &self.mock_signer
    }
}

#[derive(Clone)]
pub struct GeneralChainIdentity {
    id: usize,
    chain_id: usize,
    wallet: LocalWallet,
    provider_rpc_endpoint: String,
    contract_address: Address,
}

impl GeneralChainIdentity {
    pub fn new(
        id: usize,
        chain_id: usize,
        wallet: LocalWallet,
        provider_rpc_endpoint: String,
        contract_address: Address,
    ) -> Self {
        GeneralChainIdentity {
            id,
            chain_id,
            wallet,
            provider_rpc_endpoint,
            contract_address,
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

    fn get_contract_address(&self) -> Address {
        self.contract_address
    }

    fn get_signer(&self) -> &LocalWallet {
        &self.wallet
    }
}
