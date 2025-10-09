use crate::{
    adapter::{AdapterClientBuilder, AdapterLogs, AdapterTransactions, AdapterViews},
    error::{ContractClientError, ContractClientResult},
    ethers::adapter::{
        Adapter::{AdapterInstance, RandomnessRequest as ContractRandomnessRequest},
        IAdapter::{PartialSignature as ContractPartialSignature, RequestDetail},
    },
    ServiceClient, TransactionCaller, ViewCaller,
};
use alloy::{
    eips::BlockNumberOrTag,
    hex,
    primitives::{Address, U256},
    rpc::types::TransactionReceipt,
    sol,
};
use arpa_core::{
    pad_to_bytes32, ChainIdentity, ExponentialBackoffRetryDescriptor, GeneralMainChainIdentity,
    GeneralRelayedChainIdentity, PartialSignature, ProviderClientWithSigner, RandomnessRequestType,
    RandomnessTask, DEFAULT_MINIMUM_THRESHOLD, FULFILL_RANDOMNESS_GAS_EXCEPT_CALLBACK,
    RANDOMNESS_REWARD_GAS, VERIFICATION_GAS_OVER_MINIMUM_THRESHOLD,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use log::info;
use std::{collections::BTreeMap, future::Future};
use threshold_bls::poly::Eval;

sol! {
    #[sol(ignore_unlinked)]
    #[sol(rpc)]
    Adapter,
    "abi/Adapter.json"
}

#[allow(dead_code)]
pub struct AdapterClient {
    chain_id: u64,
    main_id_address: Address,
    adapter_address: Address,
    client: ProviderClientWithSigner,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    max_priority_fee_per_gas: Option<u128>,
}

impl AdapterClient {
    pub fn new(
        chain_id: u64,
        main_id_address: Address,
        adapter_address: Address,
        client: ProviderClientWithSigner,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
        max_priority_fee_per_gas: Option<u128>,
    ) -> Self {
        AdapterClient {
            chain_id,
            main_id_address,
            adapter_address,
            client,
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
            max_priority_fee_per_gas,
        }
    }
}

impl AdapterClientBuilder for GeneralMainChainIdentity {
    type AdapterService = AdapterClient;

    fn build_adapter_client(&self, main_id_address: Address) -> AdapterClient {
        AdapterClient::new(
            self.get_chain_id(),
            main_id_address,
            self.get_adapter_address(),
            self.get_client(),
            self.get_contract_transaction_retry_descriptor(),
            self.get_contract_view_retry_descriptor(),
            self.get_max_priority_fee_per_gas(),
        )
    }
}

impl AdapterClientBuilder for GeneralRelayedChainIdentity {
    type AdapterService = AdapterClient;

    fn build_adapter_client(&self, main_id_address: Address) -> AdapterClient {
        AdapterClient::new(
            self.get_chain_id(),
            main_id_address,
            self.get_adapter_address(),
            self.get_client(),
            self.get_contract_transaction_retry_descriptor(),
            self.get_contract_view_retry_descriptor(),
            self.get_max_priority_fee_per_gas(),
        )
    }
}

type AdapterContract = AdapterInstance<ProviderClientWithSigner>;

#[async_trait]
impl ServiceClient<AdapterContract> for AdapterClient {
    async fn prepare_service_client(&self) -> ContractClientResult<AdapterContract> {
        let adapter_contract = Adapter::new(self.adapter_address, self.client.clone());

        Ok(adapter_contract)
    }
}

#[async_trait]
impl TransactionCaller for AdapterClient {}

#[async_trait]
impl ViewCaller for AdapterClient {}

#[async_trait]
impl AdapterTransactions for AdapterClient {
    async fn fulfill_randomness(
        &self,
        group_index: usize,
        task: RandomnessTask,
        signature: Vec<u8>,
        partial_signatures: BTreeMap<Address, PartialSignature>,
    ) -> ContractClientResult<TransactionReceipt> {
        let adapter_contract =
            ServiceClient::<AdapterContract>::prepare_service_client(self).await?;

        let r_id = pad_to_bytes32(&task.request_id).unwrap();

        let sig = U256::from_be_slice(signature.as_slice());

        let ps: Vec<ContractPartialSignature> = partial_signatures
            .values()
            .map(|ps| {
                let eval: Eval<Vec<u8>> =
                    bincode::deserialize(&ps.signed_partial_signature).unwrap();

                let sig: U256 = U256::from_be_slice(eval.value.as_slice());
                ContractPartialSignature {
                    index: U256::from(ps.index),
                    partialSignature: sig,
                }
            })
            .collect();

        let rd = RequestDetail {
            subId: task.subscription_id,
            groupIndex: task.group_index,
            requestType: task.request_type.to_u8(),
            params: task.params.into(),
            callbackContract: task.requester,
            seed: task.seed,
            requestConfirmations: task.request_confirmations,
            callbackGasLimit: task.callback_gas_limit,
            callbackMaxGasPrice: U256::from(task.callback_max_gas_price),
            blockNum: U256::from(task.assignment_block_height),
        };

        let call = adapter_contract.fulfillRandomness(group_index as u32, r_id.into(), sig, rd, ps);

        let partial_signers_count = partial_signatures.len() as u32;

        let extra_verification_gas = if partial_signers_count > DEFAULT_MINIMUM_THRESHOLD {
            VERIFICATION_GAS_OVER_MINIMUM_THRESHOLD
                * (partial_signers_count - DEFAULT_MINIMUM_THRESHOLD)
        } else {
            0
        };

        let extra_add_reward_gas = partial_signers_count * RANDOMNESS_REWARD_GAS;

        let txn_gas_limit = task.callback_gas_limit
            + FULFILL_RANDOMNESS_GAS_EXCEPT_CALLBACK
            + extra_verification_gas
            + extra_add_reward_gas;

        AdapterClient::call_contract_transaction(
            self.chain_id,
            "fulfill_randomness",
            adapter_contract.provider(),
            call.gas(txn_gas_limit as u64),
            self.contract_transaction_retry_descriptor,
            false,
            self.max_priority_fee_per_gas,
        )
        .await
    }
}

#[async_trait]
impl AdapterViews for AdapterClient {
    async fn get_last_randomness(&self) -> ContractClientResult<U256> {
        let adapter_contract =
            ServiceClient::<AdapterContract>::prepare_service_client(self).await?;

        AdapterClient::call_contract_view(
            self.chain_id,
            "get_last_randomness",
            adapter_contract.getLastRandomness(),
            self.contract_view_retry_descriptor,
        )
        .await
    }

    async fn is_task_pending(&self, request_id: &[u8]) -> ContractClientResult<bool> {
        let adapter_contract =
            ServiceClient::<AdapterContract>::prepare_service_client(self).await?;

        let r_id = pad_to_bytes32(request_id).unwrap();
        AdapterClient::call_contract_view(
            self.chain_id,
            "get_pending_request",
            adapter_contract.getPendingRequestCommitment(r_id.into()),
            self.contract_view_retry_descriptor,
        )
        .await
        .map(|r| {
            let r = U256::from_be_slice(r.as_slice());
            !r.is_zero()
        })
    }
}

#[async_trait]
impl AdapterLogs for AdapterClient {
    async fn subscribe_randomness_task<
        C: FnMut(RandomnessTask) -> F + Send,
        F: Future<Output = ContractClientResult<()>> + Send,
    >(
        &self,
        mut cb: C,
    ) -> ContractClientResult<()> {
        let contract = Adapter::new(self.adapter_address, self.client.clone());

        let mut stream = contract
            .RandomnessRequest_filter()
            .from_block(BlockNumberOrTag::Latest)
            .subscribe()
            .await?
            .into_stream();

        while let Some(Ok(evt)) = stream.next().await {
            let (
                ContractRandomnessRequest {
                    requestId,
                    subId,
                    groupIndex,
                    requestType,
                    params,
                    sender,
                    seed,
                    requestConfirmations,
                    callbackGasLimit,
                    callbackMaxGasPrice,
                    estimatedPayment: _,
                },
                meta,
            ) = evt;

            info!( "Received randomness task: chain_id: {}, group_index: {}, request_id: {}, sender: {:?}, sub_id: {}, seed: {}, request_confirmations: {}, callback_gas_limit: {}, callback_max_gas_price: {}, block_number: {}",
                self.chain_id, groupIndex, format!("0x{}", hex::encode(requestId)), sender, subId, seed, requestConfirmations, callbackGasLimit, callbackMaxGasPrice, meta.block_number.unwrap_or(0));

            let task = RandomnessTask {
                request_id: requestId.to_vec(),
                subscription_id: subId,
                group_index: groupIndex,
                request_type: RandomnessRequestType::from(requestType),
                params: params.to_vec(),
                requester: sender,
                seed,
                request_confirmations: requestConfirmations,
                callback_gas_limit: callbackGasLimit,
                callback_max_gas_price: callbackMaxGasPrice.to::<u128>(),
                assignment_block_height: meta.block_number.unwrap_or(0) as usize,
            };
            cb(task).await?;
        }
        Err(ContractClientError::FetchingRandomnessTaskError)
    }
}
