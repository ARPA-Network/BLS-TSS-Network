use crate::{
    // contract_stub::controller_relayer::ControllerRelayer,
    controller_relayer::{ControllerRelayerClientBuilder, ControllerRelayerTransactions},
    error::ContractClientResult,
    ethers::controller_relayer::ControllerRelayer::ControllerRelayerInstance,
    ServiceClient,
    TransactionCaller,
};
use alloy::{
    primitives::{Address, U256},
    rpc::types::TransactionReceipt,
    sol,
};
use arpa_core::{
    ChainIdentity, ExponentialBackoffRetryDescriptor, GeneralMainChainIdentity,
    GeneralRelayedChainIdentity, MainChainIdentity, ProviderClientWithSigner,
};
use async_trait::async_trait;

sol! {
    #[sol(ignore_unlinked)]
    #[sol(rpc)]
    ControllerRelayer,
    "abi/ControllerRelayer.json"
}

pub struct ControllerRelayerClient {
    chain_id: u64,
    controller_relayer_address: Address,
    client: ProviderClientWithSigner,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    max_priority_fee_per_gas: Option<u128>,
}

impl ControllerRelayerClient {
    pub fn new(
        chain_id: u64,
        controller_relayer_address: Address,
        identity: &GeneralMainChainIdentity,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        max_priority_fee_per_gas: Option<u128>,
    ) -> Self {
        ControllerRelayerClient {
            chain_id,
            controller_relayer_address,
            client: identity.get_client(),
            contract_transaction_retry_descriptor,
            max_priority_fee_per_gas,
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
            self.get_max_priority_fee_per_gas(),
        )
    }
}

impl ControllerRelayerClientBuilder for GeneralRelayedChainIdentity {
    type ControllerRelayerService = ControllerRelayerClient;

    fn build_controller_relayer_client(&self) -> ControllerRelayerClient {
        panic!("not implemented")
    }
}

type ControllerRelayerContract = ControllerRelayerInstance<ProviderClientWithSigner>;

#[async_trait]
impl ServiceClient<ControllerRelayerContract> for ControllerRelayerClient {
    async fn prepare_service_client(&self) -> ContractClientResult<ControllerRelayerContract> {
        let controller_relayer_contract =
            ControllerRelayer::new(self.controller_relayer_address, self.client.clone());

        Ok(controller_relayer_contract)
    }
}

#[async_trait]
impl TransactionCaller for ControllerRelayerClient {}

#[async_trait]
impl ControllerRelayerTransactions for ControllerRelayerClient {
    async fn relay_group(
        &self,
        chain_id: u64,
        group_index: usize,
    ) -> ContractClientResult<TransactionReceipt> {
        let controller_relayer_contract =
            ServiceClient::<ControllerRelayerContract>::prepare_service_client(self).await?;

        let call =
            controller_relayer_contract.relayGroup(U256::from(chain_id), U256::from(group_index));

        ControllerRelayerClient::call_contract_transaction(
            self.chain_id,
            "relay_group",
            controller_relayer_contract.provider(),
            call,
            self.contract_transaction_retry_descriptor,
            false,
            self.max_priority_fee_per_gas,
        )
        .await
    }
}
