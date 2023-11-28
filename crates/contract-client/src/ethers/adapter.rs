use crate::{
    adapter::{AdapterClientBuilder, AdapterLogs, AdapterTransactions, AdapterViews},
    contract_stub::adapter::{
        Adapter, PartialSignature as ContractPartialSignature, RandomnessRequestFilter,
        RequestDetail,
    },
    error::{ContractClientError, ContractClientResult},
    ServiceClient, TransactionCaller, ViewCaller,
};
use arpa_core::{
    pad_to_bytes32, ChainIdentity, ExponentialBackoffRetryDescriptor, GeneralMainChainIdentity,
    GeneralRelayedChainIdentity, PartialSignature, RandomnessRequestType, RandomnessTask,
    WsWalletSigner, DEFAULT_MINIMUM_THRESHOLD, FULFILL_RANDOMNESS_GAS_EXCEPT_CALLBACK,
    RANDOMNESS_REWARD_GAS, VERIFICATION_GAS_OVER_MINIMUM_THRESHOLD,
};
use async_trait::async_trait;
use ethers::{prelude::*, utils::hex};
use log::info;
use std::{collections::HashMap, future::Future, sync::Arc};

#[allow(dead_code)]
pub struct AdapterClient {
    chain_id: usize,
    main_id_address: Address,
    adapter_address: Address,
    signer: Arc<WsWalletSigner>,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl AdapterClient {
    pub fn new(
        chain_id: usize,
        main_id_address: Address,
        adapter_address: Address,
        signer: Arc<WsWalletSigner>,
        contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
        contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    ) -> Self {
        AdapterClient {
            chain_id,
            main_id_address,
            adapter_address,
            signer,
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
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
            self.get_signer(),
            self.get_contract_transaction_retry_descriptor(),
            self.get_contract_view_retry_descriptor(),
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
            self.get_signer(),
            self.get_contract_transaction_retry_descriptor(),
            self.get_contract_view_retry_descriptor(),
        )
    }
}

type AdapterContract = Adapter<WsWalletSigner>;

#[async_trait]
impl ServiceClient<AdapterContract> for AdapterClient {
    async fn prepare_service_client(&self) -> ContractClientResult<AdapterContract> {
        let adapter_contract = Adapter::new(self.adapter_address, self.signer.clone());

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
        partial_signatures: HashMap<Address, PartialSignature>,
    ) -> ContractClientResult<H256> {
        let adapter_contract =
            ServiceClient::<AdapterContract>::prepare_service_client(self).await?;

        let r_id = pad_to_bytes32(&task.request_id).unwrap();

        let sig = U256::from(signature.as_slice());

        let ps: Vec<ContractPartialSignature> = partial_signatures
            .values()
            .map(|ps| {
                let sig: U256 = U256::from(ps.signature.as_slice());
                ContractPartialSignature {
                    index: ps.index.into(),
                    partial_signature: sig,
                }
            })
            .collect();

        let rd = RequestDetail {
            sub_id: task.subscription_id,
            group_index: group_index as u32,
            request_type: task.request_type.to_u8(),
            params: task.params.into(),
            callback_contract: task.requester,
            seed: task.seed,
            request_confirmations: task.request_confirmations,
            callback_gas_limit: task.callback_gas_limit,
            callback_max_gas_price: task.callback_max_gas_price,
            block_num: task.assignment_block_height.into(),
        };

        let call = adapter_contract.fulfill_randomness(group_index as u32, r_id, sig, rd, ps);

        let partial_signers_count = partial_signatures.len() as u32;

        let extra_verification_gas = if partial_signers_count > DEFAULT_MINIMUM_THRESHOLD {
            VERIFICATION_GAS_OVER_MINIMUM_THRESHOLD
                * (partial_signers_count - DEFAULT_MINIMUM_THRESHOLD)
        } else {
            0
        };

        let extra_add_reward_gas = partial_signers_count * RANDOMNESS_REWARD_GAS;

        AdapterClient::call_contract_transaction(
            self.chain_id,
            "fulfill_randomness",
            adapter_contract.client_ref(),
            call.gas(
                task.callback_gas_limit
                    + FULFILL_RANDOMNESS_GAS_EXCEPT_CALLBACK
                    + extra_verification_gas
                    + extra_add_reward_gas,
            ),
            self.contract_transaction_retry_descriptor,
            false,
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
            adapter_contract.get_last_randomness(),
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
            adapter_contract.get_pending_request_commitment(r_id),
            self.contract_view_retry_descriptor,
        )
        .await
        .map(|r| {
            let r = U256::from(r);
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
        let contract = Adapter::new(self.adapter_address, self.signer.clone());

        let events = contract
            .event::<RandomnessRequestFilter>()
            .from_block(BlockNumber::Latest);

        let mut stream = events.subscribe().await?.with_meta();

        while let Some(Ok(evt)) = stream.next().await {
            let (
                RandomnessRequestFilter {
                    request_id,
                    sub_id,
                    group_index,
                    request_type,
                    params,
                    sender,
                    seed,
                    request_confirmations,
                    callback_gas_limit,
                    callback_max_gas_price,
                    estimated_payment: _,
                },
                meta,
            ) = evt;

            info!( "Received randomness task: chain_id: {}, group_index: {}, request_id: {}, sender: {:?}, sub_id: {}, seed: {}, request_confirmations: {}, callback_gas_limit: {}, callback_max_gas_price: {}, block_number: {}",
                self.chain_id, group_index, hex::encode(request_id), sender, sub_id, seed, request_confirmations, callback_gas_limit, callback_max_gas_price, meta.block_number);

            let task = RandomnessTask {
                request_id: request_id.to_vec(),
                subscription_id: sub_id,
                group_index,
                request_type: RandomnessRequestType::from(request_type),
                params: params.to_vec(),
                requester: sender,
                seed,
                request_confirmations,
                callback_gas_limit,
                callback_max_gas_price,
                assignment_block_height: meta.block_number.as_usize(),
            };
            cb(task).await?;
        }
        Err(ContractClientError::FetchingRandomnessTaskError)
    }
}
