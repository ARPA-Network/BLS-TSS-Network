use crate::ExponentialBackoffRetryDescriptor;

use super::ChainIdentity;
use ethers_core::types::Address;
use ethers_middleware::{NonceManagerMiddleware, SignerMiddleware};
use ethers_providers::{Http, Provider};
use ethers_signers::{LocalWallet, Signer};
use std::{sync::Arc, time::Duration};

pub type WalletSigner = SignerMiddleware<NonceManagerMiddleware<Arc<Provider<Http>>>, LocalWallet>;

#[derive(Debug, Clone)]
pub struct GeneralChainIdentity {
    chain_id: usize,
    provider: Arc<Provider<Http>>,
    signer: Arc<WalletSigner>,
    controller_address: Address,
    adapter_address: Address,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl GeneralChainIdentity {
    pub fn new(
        chain_id: usize,
        wallet: LocalWallet,
        provider_rpc_endpoint: String,
        provider_polling_interval_millis: u64,
        controller_address: Address,
        adapter_address: Address,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    ) -> Self {
        let provider = Arc::new(
            Provider::<Http>::try_from(provider_rpc_endpoint)
                .unwrap()
                .interval(Duration::from_millis(provider_polling_interval_millis)),
        );

        let wallet = wallet.with_chain_id(chain_id as u32);

        let nonce_manager = NonceManagerMiddleware::new(provider.clone(), wallet.address());

        // instantiate the client with the wallet
        let signer = Arc::new(SignerMiddleware::new(nonce_manager, wallet));

        GeneralChainIdentity {
            chain_id,
            provider,
            signer,
            controller_address,
            adapter_address,
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
        }
    }
}

impl ChainIdentity for GeneralChainIdentity {
    fn get_chain_id(&self) -> usize {
        self.chain_id
    }

    fn get_id_address(&self) -> Address {
        self.signer.address()
    }

    fn get_controller_address(&self) -> Address {
        self.controller_address
    }

    fn get_adapter_address(&self) -> Address {
        self.adapter_address
    }

    fn get_signer(&self) -> Arc<WalletSigner> {
        self.signer.clone()
    }

    fn get_provider(&self) -> Arc<Provider<Http>> {
        self.provider.clone()
    }

    fn get_contract_transaction_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor {
        self.contract_transaction_retry_descriptor
    }

    fn get_contract_view_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor {
        self.contract_view_retry_descriptor
    }
}
