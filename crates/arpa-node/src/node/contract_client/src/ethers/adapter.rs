use crate::{
    adapter::{AdapterClientBuilder, AdapterLogs, AdapterTransactions, AdapterViews},
    contract_stub::{
        adapter::{Adapter, RandomnessRequestFilter},
        shared_types::PartialSignature as ContractPartialSignature,
    },
    error::{ContractClientError, ContractClientResult},
    ServiceClient, TransactionCaller, ViewCaller,
};
use arpa_node_core::{
    ChainIdentity, GeneralChainIdentity, PartialSignature, RandomnessTask, WalletSigner,
};
use async_trait::async_trait;
use ethers::{prelude::*, utils::hex};
use log::info;
use std::{collections::HashMap, future::Future, sync::Arc};

#[allow(dead_code)]
pub struct AdapterClient {
    main_id_address: Address,
    adapter_address: Address,
    signer: Arc<WalletSigner>,
}

impl AdapterClient {
    pub fn new(
        main_id_address: Address,
        adapter_address: Address,
        identity: &GeneralChainIdentity,
    ) -> Self {
        AdapterClient {
            main_id_address,
            adapter_address,
            signer: identity.get_signer(),
        }
    }
}

impl AdapterClientBuilder for GeneralChainIdentity {
    type Service = AdapterClient;

    fn build_adapter_client(&self, main_id_address: Address) -> AdapterClient {
        AdapterClient::new(main_id_address, self.get_adapter_address(), self)
    }
}

type AdapterContract = Adapter<WalletSigner>;

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
        request_id: Vec<u8>,
        signature: Vec<u8>,
        partial_signatures: HashMap<Address, PartialSignature>,
    ) -> ContractClientResult<H256> {
        let adapter_contract =
            ServiceClient::<AdapterContract>::prepare_service_client(self).await?;

        let r_id = pad_to_bytes32(&request_id).unwrap();

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

        let call = adapter_contract.fulfill_randomness(group_index.into(), r_id, sig, ps);

        AdapterClient::call_contract_transaction("fulfill_randomness", call).await
    }
}

#[async_trait]
impl AdapterViews for AdapterClient {
    async fn get_last_randomness(&self) -> ContractClientResult<U256> {
        let adapter_contract =
            ServiceClient::<AdapterContract>::prepare_service_client(self).await?;

        AdapterClient::call_contract_view(
            "get_last_randomness",
            adapter_contract.get_last_randomness(),
        )
        .await
    }

    async fn is_task_pending(&self, request_id: &[u8]) -> ContractClientResult<bool> {
        let adapter_contract =
            ServiceClient::<AdapterContract>::prepare_service_client(self).await?;

        let r_id = pad_to_bytes32(request_id).unwrap();

        AdapterClient::call_contract_view(
            "get_pending_request",
            adapter_contract.get_pending_request(r_id),
        )
        .await
        .map(|r| r.sub_id != 0)
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

        let events: Event<WalletSigner, RandomnessRequestFilter> = contract
            .event::<RandomnessRequestFilter>()
            .from_block(BlockNumber::Latest);

        // turn the stream into a stream of events
        let mut stream = events.stream().await?.with_meta();

        while let Some(Ok(evt)) = stream.next().await {
            let (
                RandomnessRequestFilter {
                    group_index,
                    request_id,
                    sender,
                    sub_id,
                    seed,
                    request_confirmations,
                    callback_gas_limit,
                    callback_max_gas_price,
                },
                meta,
            ) = evt;

            info!( "Received randomness task: group_index: {}, request_id: {}, sender: {:?}, sub_id: {}, seed: {}, request_confirmations: {}, callback_gas_limit: {}, callback_max_gas_price: {}, block_number: {}",
                group_index, hex::encode(request_id), sender, sub_id, seed, request_confirmations, callback_gas_limit, callback_max_gas_price, meta.block_number);

            let task = RandomnessTask {
                request_id: request_id.to_vec(),
                seed,
                group_index: group_index.as_usize(),
                request_confirmations: request_confirmations as usize,
                assignment_block_height: meta.block_number.as_usize(),
            };
            cb(task).await?;
        }
        Err(ContractClientError::FetchingRandomnessTaskError)
    }
}

fn pad_to_bytes32(s: &[u8]) -> Option<[u8; 32]> {
    let s_len = s.len();

    if s_len > 32 {
        return None;
    }

    let mut result: [u8; 32] = Default::default();

    result[..s_len].clone_from_slice(s);

    Some(result)
}
