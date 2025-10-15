use crate::controller_oracle::ControllerOracleTransactions;
use crate::ethers::controller_oracle::ControllerOracle::ControllerOracleInstance;
use crate::ethers::parse_controller_oracle_contract_group;
use crate::{
    // contract_stub::controller_oracle::{ControllerOracle, Group as ContractGroup},
    controller_oracle::{ControllerOracleClientBuilder, ControllerOracleViews},
    error::ContractClientResult,
    ServiceClient,
};
use crate::{TransactionCaller, ViewCaller};
use alloy::{
    primitives::{Address, U256},
    rpc::types::TransactionReceipt,
    sol,
};
use arpa_core::{
    ChainIdentity, ExponentialBackoffRetryDescriptor, GeneralMainChainIdentity,
    GeneralRelayedChainIdentity, Group, RelayedChainIdentity, ProviderClientWithSigner,
};
use async_trait::async_trait;
use threshold_bls::group::Curve;

sol! {
    #[sol(ignore_unlinked)]
    #[sol(rpc)]
    ControllerOracle,
    "abi/ControllerOracle.json"
}

pub struct ControllerOracleClient {
    chain_id: u64,
    controller_oracle_address: Address,
    client: ProviderClientWithSigner,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    max_priority_fee_per_gas: Option<u128>,
}

impl ControllerOracleClient {
    pub fn new(
        chain_id: u64,
        controller_oracle_address: Address,
        identity: &GeneralRelayedChainIdentity,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
        max_priority_fee_per_gas: Option<u128>,
    ) -> Self {
        ControllerOracleClient {
            chain_id,
            controller_oracle_address,
            client: identity.get_client(),
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
            max_priority_fee_per_gas,
        }
    }
}

impl<C: Curve> ControllerOracleClientBuilder<C> for GeneralMainChainIdentity {
    type ControllerOracleService = ControllerOracleClient;

    fn build_controller_oracle_client(&self) -> ControllerOracleClient {
        panic!("not implemented")
    }
}

impl<C: Curve> ControllerOracleClientBuilder<C> for GeneralRelayedChainIdentity {
    type ControllerOracleService = ControllerOracleClient;

    fn build_controller_oracle_client(&self) -> ControllerOracleClient {
        ControllerOracleClient::new(
            self.get_chain_id(),
            self.get_controller_oracle_address(),
            self,
            self.get_contract_transaction_retry_descriptor(),
            self.get_contract_view_retry_descriptor(),
            self.get_max_priority_fee_per_gas(),
        )
    }
}

type ControllerOracleContract = ControllerOracleInstance<ProviderClientWithSigner>;

#[async_trait]
impl ServiceClient<ControllerOracleContract> for ControllerOracleClient {
    async fn prepare_service_client(&self) -> ContractClientResult<ControllerOracleContract> {
        let controller_oracle_contract =
            ControllerOracle::new(self.controller_oracle_address, self.client.clone());

        Ok(controller_oracle_contract)
    }
}

#[async_trait]
impl TransactionCaller for ControllerOracleClient {}

#[async_trait]
impl ViewCaller for ControllerOracleClient {}

#[async_trait]
impl ControllerOracleTransactions for ControllerOracleClient {
    async fn node_withdraw(&self, recipient: Address) -> ContractClientResult<TransactionReceipt> {
        let controller_oracle_contract =
            ServiceClient::<ControllerOracleContract>::prepare_service_client(self).await?;

        let call = controller_oracle_contract.nodeWithdraw(recipient);

        ControllerOracleClient::call_contract_transaction(
            self.chain_id,
            "node_withdraw",
            controller_oracle_contract.provider(),
            call,
            self.contract_transaction_retry_descriptor,
            true,
            self.max_priority_fee_per_gas,
        )
        .await
    }
}

#[async_trait]
impl<C: Curve> ControllerOracleViews<C> for ControllerOracleClient {
    async fn get_group(&self, group_index: usize) -> ContractClientResult<Group<C>> {
        let controller_oracle_contract =
            ServiceClient::<ControllerOracleContract>::prepare_service_client(self).await?;

        ControllerOracleClient::call_contract_view(
            self.chain_id,
            "get_group",
            controller_oracle_contract.getGroup(U256::from(group_index)),
            self.contract_view_retry_descriptor,
        )
        .await
        .map(parse_controller_oracle_contract_group)
    }
}
