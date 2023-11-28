use crate::{
    eip1559_gas_price_estimator, ChainProviderManager, ExponentialBackoffRetryDescriptor,
    RelayedChainIdentity, DEFAULT_WEBSOCKET_PROVIDER_RECONNECT_TIMES,
};

use super::{ChainIdentity, MainChainIdentity};
use async_trait::async_trait;
use ethers_core::types::{Address, BlockNumber, U256};
use ethers_middleware::{NonceManagerMiddleware, SignerMiddleware};
use ethers_providers::{Http, Middleware, Provider, ProviderError, Ws};
use ethers_signers::{LocalWallet, Signer};
use std::sync::Arc;

pub type WsWalletSigner = SignerMiddleware<NonceManagerMiddleware<Arc<Provider<Ws>>>, LocalWallet>;
pub type HttpWalletSigner =
    SignerMiddleware<NonceManagerMiddleware<Arc<Provider<Http>>>, LocalWallet>;

#[derive(Debug, Clone)]
pub struct GeneralMainChainIdentity {
    chain_id: usize,
    signer: Arc<WsWalletSigner>,
    provider_endpoint: String,
    controller_address: Address,
    controller_relayer_address: Address,
    adapter_address: Address,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl GeneralMainChainIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chain_id: usize,
        wallet: LocalWallet,
        provider: Arc<Provider<Ws>>,
        provider_endpoint: String,
        controller_address: Address,
        controller_relayer_address: Address,
        adapter_address: Address,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    ) -> Self {
        let wallet = wallet.with_chain_id(chain_id as u32);

        let nonce_manager = NonceManagerMiddleware::new(provider, wallet.address());

        // instantiate the client with the wallet
        let signer = Arc::new(SignerMiddleware::new(nonce_manager, wallet));

        GeneralMainChainIdentity {
            chain_id,
            signer,
            provider_endpoint,
            controller_address,
            controller_relayer_address,
            adapter_address,
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
        }
    }
}

#[async_trait]
impl ChainIdentity for GeneralMainChainIdentity {
    fn get_chain_id(&self) -> usize {
        self.chain_id
    }

    fn get_id_address(&self) -> Address {
        self.signer.address()
    }

    fn get_adapter_address(&self) -> Address {
        self.adapter_address
    }

    fn get_signer(&self) -> Arc<WsWalletSigner> {
        self.signer.clone()
    }

    fn get_contract_transaction_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor {
        self.contract_transaction_retry_descriptor
    }

    fn get_contract_view_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor {
        self.contract_view_retry_descriptor
    }

    async fn get_current_gas_price(&self) -> Result<U256, ProviderError> {
        let (max_fee, _) = self
            .signer
            .provider()
            .estimate_eip1559_fees(Some(eip1559_gas_price_estimator))
            .await?;

        Ok(max_fee)
    }

    async fn get_block_timestamp(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<U256>, ProviderError> {
        self.signer
            .provider()
            .get_block(block_number)
            .await
            .map(|o| o.map(|b| b.timestamp))
    }
}

#[async_trait]
impl MainChainIdentity for GeneralMainChainIdentity {
    fn get_controller_address(&self) -> Address {
        self.controller_address
    }

    fn get_controller_relayer_address(&self) -> Address {
        self.controller_relayer_address
    }
}

#[async_trait]
impl ChainProviderManager for GeneralMainChainIdentity {
    fn get_provider(&self) -> &Provider<Ws> {
        self.signer.provider()
    }

    fn get_provider_endpoint(&self) -> &str {
        &self.provider_endpoint
    }

    async fn reset_provider(&mut self) -> Result<(), ProviderError> {
        let provider = Arc::new(
            Provider::<Ws>::connect_with_reconnects(
                &self.provider_endpoint,
                DEFAULT_WEBSOCKET_PROVIDER_RECONNECT_TIMES,
            )
            .await?
            .interval(self.get_provider().get_interval()),
        );

        let nonce_manager = NonceManagerMiddleware::new(provider, self.get_id_address());

        let signer = Arc::new(SignerMiddleware::new(
            nonce_manager,
            self.signer.signer().clone(),
        ));

        self.signer = signer;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GeneralRelayedChainIdentity {
    chain_id: usize,
    signer: Arc<WsWalletSigner>,
    provider_endpoint: String,
    controller_oracle_address: Address,
    adapter_address: Address,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl GeneralRelayedChainIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chain_id: usize,
        wallet: LocalWallet,
        provider: Arc<Provider<Ws>>,
        provider_endpoint: String,
        controller_oracle_address: Address,
        adapter_address: Address,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    ) -> Self {
        let wallet = wallet.with_chain_id(chain_id as u32);

        let nonce_manager = NonceManagerMiddleware::new(provider, wallet.address());

        // instantiate the client with the wallet
        let signer = Arc::new(SignerMiddleware::new(nonce_manager, wallet));

        GeneralRelayedChainIdentity {
            chain_id,
            signer,
            provider_endpoint,
            controller_oracle_address,
            adapter_address,
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
        }
    }
}

#[async_trait]
impl ChainIdentity for GeneralRelayedChainIdentity {
    fn get_chain_id(&self) -> usize {
        self.chain_id
    }

    fn get_id_address(&self) -> Address {
        self.signer.address()
    }

    fn get_adapter_address(&self) -> Address {
        self.adapter_address
    }

    fn get_signer(&self) -> Arc<WsWalletSigner> {
        self.signer.clone()
    }

    fn get_contract_transaction_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor {
        self.contract_transaction_retry_descriptor
    }

    fn get_contract_view_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor {
        self.contract_view_retry_descriptor
    }

    async fn get_current_gas_price(&self) -> Result<U256, ProviderError> {
        let (max_fee, _) = self
            .signer
            .provider()
            .estimate_eip1559_fees(Some(eip1559_gas_price_estimator))
            .await?;

        Ok(max_fee)
    }

    async fn get_block_timestamp(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<U256>, ProviderError> {
        self.signer
            .provider()
            .get_block(block_number)
            .await
            .map(|o| o.map(|b| b.timestamp))
    }
}

impl RelayedChainIdentity for GeneralRelayedChainIdentity {
    fn get_controller_oracle_address(&self) -> Address {
        self.controller_oracle_address
    }
}

#[async_trait]
impl ChainProviderManager for GeneralRelayedChainIdentity {
    fn get_provider(&self) -> &Provider<Ws> {
        self.signer.provider()
    }

    fn get_provider_endpoint(&self) -> &str {
        &self.provider_endpoint
    }

    async fn reset_provider(&mut self) -> Result<(), ProviderError> {
        let provider = Arc::new(
            Provider::<Ws>::connect_with_reconnects(
                &self.provider_endpoint,
                DEFAULT_WEBSOCKET_PROVIDER_RECONNECT_TIMES,
            )
            .await?
            .interval(self.get_provider().get_interval()),
        );

        let nonce_manager = NonceManagerMiddleware::new(provider, self.get_id_address());

        let signer = Arc::new(SignerMiddleware::new(
            nonce_manager,
            self.signer.signer().clone(),
        ));

        self.signer = signer;

        Ok(())
    }
}
