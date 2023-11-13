use crate::{
    contract_stub::controller_relayer::ControllerRelayer,
    controller_relayer::{ControllerRelayerClientBuilder, ControllerRelayerTransactions},
    error::ContractClientResult,
    ServiceClient, TransactionCaller,
};
use arpa_core::{
    ChainIdentity, ExponentialBackoffRetryDescriptor, GeneralMainChainIdentity,
    GeneralRelayedChainIdentity, MainChainIdentity, WsWalletSigner,
};
use async_trait::async_trait;
use ethers::prelude::*;
use std::sync::Arc;

pub struct ControllerRelayerClient {
    chain_id: usize,
    controller_relayer_address: Address,
    signer: Arc<WsWalletSigner>,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl ControllerRelayerClient {
    pub fn new(
        chain_id: usize,
        controller_relayer_address: Address,
        identity: &GeneralMainChainIdentity,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    ) -> Self {
        ControllerRelayerClient {
            chain_id,
            controller_relayer_address,
            signer: identity.get_signer(),
            contract_transaction_retry_descriptor,
        }
    }
}

impl ControllerRelayerClientBuilder for GeneralMainChainIdentity {
    type ControllerRelayerService = ControllerRelayerClient;

    fn build_controller_relayer_client(&self) -> ControllerRelayerClient {
        ControllerRelayerClient::new(
            self.get_chain_id(),
            self.get_controller_relayer_address(),
            self,
            self.get_contract_transaction_retry_descriptor(),
        )
    }
}

impl ControllerRelayerClientBuilder for GeneralRelayedChainIdentity {
    type ControllerRelayerService = ControllerRelayerClient;

    fn build_controller_relayer_client(&self) -> ControllerRelayerClient {
        panic!("not implemented")
    }
}

type ControllerRelayerContract = ControllerRelayer<WsWalletSigner>;

#[async_trait]
impl ServiceClient<ControllerRelayerContract> for ControllerRelayerClient {
    async fn prepare_service_client(&self) -> ContractClientResult<ControllerRelayerContract> {
        let controller_relayer_contract =
            ControllerRelayer::new(self.controller_relayer_address, self.signer.clone());

        Ok(controller_relayer_contract)
    }
}

#[async_trait]
impl TransactionCaller for ControllerRelayerClient {}

#[async_trait]
impl ControllerRelayerTransactions for ControllerRelayerClient {
    async fn relay_group(&self, chain_id: usize, group_index: usize) -> ContractClientResult<H256> {
        let controller_relayer_contract =
            ServiceClient::<ControllerRelayerContract>::prepare_service_client(self).await?;

        let call = controller_relayer_contract.relay_group(chain_id.into(), group_index.into());

        ControllerRelayerClient::call_contract_transaction(
            self.chain_id,
            "relay_group",
            controller_relayer_contract.client_ref(),
            call,
            self.contract_transaction_retry_descriptor,
            false,
        )
        .await
    }
}
