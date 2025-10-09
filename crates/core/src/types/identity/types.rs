use crate::{
    eip1559_gas_price_estimator, supports_eip1559, BlobGasFiller, ChainProviderManager,
    ExponentialBackoffRetryDescriptor, GasMiddleware, RelayedChainIdentity, GAS_RAISE_PERCENTAGE,
};

use super::{ChainIdentity, MainChainIdentity};
use alloy::providers::fillers::ChainIdFiller;
use alloy::signers::Signer;
use alloy::{
    eips::{eip1559::Eip1559Estimation, BlockId},
    network::EthereumWallet,
    primitives::{Address, BlockNumber},
    providers::{
        fillers::{FillProvider, JoinFill, NonceFiller, WalletFiller},
        utils::Eip1559Estimator,
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    signers::local::PrivateKeySigner,
    transports::TransportError,
};
use async_trait::async_trait;
use log::debug;

pub type ProviderClientWithSigner = FillProvider<
    JoinFill<
        JoinFill<
            JoinFill<JoinFill<JoinFill<Identity, ChainIdFiller>, NonceFiller>, BlobGasFiller>,
            GasMiddleware,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider,
>;

pub async fn build_client(
    wallet: PrivateKeySigner,
    chain_id: u64,
    ws_connect: WsConnect,
) -> Result<ProviderClientWithSigner, TransportError> {
    let wallet = wallet.with_chain_id(Some(chain_id));

    let client = ProviderBuilder::new()
        .disable_recommended_fillers()
        .filler(ChainIdFiller::new(Some(chain_id)))
        .with_cached_nonce_management()
        .filler(BlobGasFiller)
        .filler(GasMiddleware::new(GAS_RAISE_PERCENTAGE).expect("Failed to create GasMiddleware"))
        .wallet(wallet)
        .connect_ws(ws_connect)
        .await?;

    Ok(client)
}

#[derive(Debug, Clone)]
pub struct GeneralMainChainIdentity {
    chain_id: u64,
    wallet: PrivateKeySigner,
    ws_connect: WsConnect,
    client: ProviderClientWithSigner,
    provider_endpoint: String,
    controller_address: Address,
    controller_relayer_address: Address,
    adapter_address: Address,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    max_priority_fee_per_gas: Option<u128>,
}

impl GeneralMainChainIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chain_id: u64,
        wallet: PrivateKeySigner,
        ws_connect: WsConnect,
        client: ProviderClientWithSigner,
        provider_endpoint: String,
        controller_address: Address,
        controller_relayer_address: Address,
        adapter_address: Address,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
        max_priority_fee_per_gas: Option<u128>,
    ) -> Self {
        GeneralMainChainIdentity {
            chain_id,
            wallet,
            ws_connect,
            client,
            provider_endpoint,
            controller_address,
            controller_relayer_address,
            adapter_address,
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
            max_priority_fee_per_gas,
        }
    }
}

#[async_trait]
impl ChainIdentity for GeneralMainChainIdentity {
    fn get_chain_id(&self) -> u64 {
        self.chain_id
    }

    fn get_id_address(&self) -> Address {
        self.wallet.address()
    }

    fn get_adapter_address(&self) -> Address {
        self.adapter_address
    }

    fn get_signer(&self) -> &PrivateKeySigner {
        &self.wallet
    }

    fn get_client(&self) -> ProviderClientWithSigner {
        self.client.clone()
    }

    fn get_contract_transaction_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor {
        self.contract_transaction_retry_descriptor
    }

    fn get_contract_view_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor {
        self.contract_view_retry_descriptor
    }

    fn get_max_priority_fee_per_gas(&self) -> Option<u128> {
        self.max_priority_fee_per_gas
    }

    async fn get_current_gas_price(&self) -> Result<u128, TransportError> {
        if !supports_eip1559(self.chain_id) {
            return self.client.get_gas_price().await;
        }
        let Eip1559Estimation {
            max_fee_per_gas,
            max_priority_fee_per_gas: _,
        } = self
            .client
            .estimate_eip1559_fees_with(Eip1559Estimator::Custom(Box::new(
                eip1559_gas_price_estimator,
            )))
            .await?;

        Ok(max_fee_per_gas)
    }

    async fn get_block_timestamp(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<u64>, TransportError> {
        self.client
            .get_block(BlockId::Number(block_number.into()))
            .await
            .map(|o| o.map(|b| b.header.timestamp))
    }
}

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
    fn get_provider(&self) -> &ProviderClientWithSigner {
        &self.client
    }

    fn get_provider_endpoint(&self) -> &str {
        &self.provider_endpoint
    }

    async fn reset_provider(&mut self) -> Result<(), TransportError> {
        debug!("Resetting provider for chain {}", self.chain_id);

        self.client =
            build_client(self.wallet.clone(), self.chain_id, self.ws_connect.clone()).await?;

        debug!("Provider reset for chain {}", self.chain_id);

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GeneralRelayedChainIdentity {
    chain_id: u64,
    wallet: PrivateKeySigner,
    ws_connect: WsConnect,
    client: ProviderClientWithSigner,
    provider_endpoint: String,
    controller_oracle_address: Address,
    adapter_address: Address,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    max_priority_fee_per_gas: Option<u128>,
}

impl GeneralRelayedChainIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chain_id: u64,
        wallet: PrivateKeySigner,
        ws_connect: WsConnect,
        client: ProviderClientWithSigner,
        provider_endpoint: String,
        controller_oracle_address: Address,
        adapter_address: Address,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
        max_priority_fee_per_gas: Option<u128>,
    ) -> Self {
        GeneralRelayedChainIdentity {
            chain_id,
            wallet,
            ws_connect,
            client,
            provider_endpoint,
            controller_oracle_address,
            adapter_address,
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
            max_priority_fee_per_gas,
        }
    }
}

#[async_trait]
impl ChainIdentity for GeneralRelayedChainIdentity {
    fn get_chain_id(&self) -> u64 {
        self.chain_id
    }

    fn get_id_address(&self) -> Address {
        self.wallet.address()
    }

    fn get_adapter_address(&self) -> Address {
        self.adapter_address
    }

    fn get_signer(&self) -> &PrivateKeySigner {
        &self.wallet
    }

    fn get_client(&self) -> ProviderClientWithSigner {
        self.client.clone()
    }

    fn get_contract_transaction_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor {
        self.contract_transaction_retry_descriptor
    }

    fn get_contract_view_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor {
        self.contract_view_retry_descriptor
    }

    fn get_max_priority_fee_per_gas(&self) -> Option<u128> {
        self.max_priority_fee_per_gas
    }

    async fn get_current_gas_price(&self) -> Result<u128, TransportError> {
        if !supports_eip1559(self.chain_id) {
            return self.client.get_gas_price().await;
        }
        let Eip1559Estimation {
            max_fee_per_gas,
            max_priority_fee_per_gas: _,
        } = self
            .client
            .estimate_eip1559_fees_with(Eip1559Estimator::Custom(Box::new(
                eip1559_gas_price_estimator,
            )))
            .await?;

        Ok(max_fee_per_gas)
    }

    async fn get_block_timestamp(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<u64>, TransportError> {
        self.client
            .get_block(BlockId::Number(block_number.into()))
            .await
            .map(|o| o.map(|b| b.header.timestamp))
    }
}

impl RelayedChainIdentity for GeneralRelayedChainIdentity {
    fn get_controller_oracle_address(&self) -> Address {
        self.controller_oracle_address
    }
}

#[async_trait]
impl ChainProviderManager for GeneralRelayedChainIdentity {
    fn get_provider(&self) -> &ProviderClientWithSigner {
        &self.client
    }

    fn get_provider_endpoint(&self) -> &str {
        &self.provider_endpoint
    }

    async fn reset_provider(&mut self) -> Result<(), TransportError> {
        debug!("Resetting provider for chain {}", self.chain_id);

        self.client =
            build_client(self.wallet.clone(), self.chain_id, self.ws_connect.clone()).await?;

        debug!("Provider reset for chain {}", self.chain_id);

        Ok(())
    }
}
