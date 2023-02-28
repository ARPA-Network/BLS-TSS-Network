use super::{provider::NONCE, WalletSigner};
use crate::{
    adapter::{AdapterClientBuilder, AdapterLogs, AdapterTransactions, AdapterViews},
    contract_stub::{
        adapter::{Adapter, RandomnessRequestFilter},
        shared_types::{Group as ContractGroup, PartialSignature as ContractPartialSignature},
    },
    error::{ContractClientError, ContractClientResult},
    NonceManager, ServiceClient, TransactionCaller,
};
use arpa_node_core::{
    u256_to_vec, ChainIdentity, GeneralChainIdentity, Group, Member, PartialSignature,
    RandomnessTask,
};
use async_trait::async_trait;
use ethers::{
    prelude::{builders::ContractCall, *},
    utils::hex,
};
use log::info;
use std::{
    collections::{BTreeMap, HashMap},
    convert::TryFrom,
    future::Future,
    marker::PhantomData,
    sync::Arc,
    time::Duration,
};
use threshold_bls::group::PairingCurve;

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
        let provider = Provider::<Http>::try_from(identity.get_provider_rpc_endpoint())
            .unwrap()
            .interval(Duration::from_millis(3000));

        // instantiate the client with the wallet
        let signer = Arc::new(SignerMiddleware::new(
            provider,
            identity
                .get_signer()
                .clone()
                .with_chain_id(identity.get_chain_id() as u32),
        ));

        AdapterClient {
            main_id_address,
            adapter_address,
            signer,
        }
    }
}

impl<C: PairingCurve> AdapterClientBuilder<C> for GeneralChainIdentity {
    type Service = AdapterClient;

    fn build_adapter_client(&self, main_id_address: Address) -> AdapterClient {
        AdapterClient::new(main_id_address, self.get_contract_address(), self)
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
impl NonceManager for AdapterClient {
    async fn increment_or_initialize_nonce(&self) -> ContractClientResult<u64> {
        let mut nonce = NONCE.lock().await;
        if *nonce == -1 {
            let tx_count = self
                .signer
                .get_transaction_count(self.signer.address(), None)
                .await?;
            *nonce = tx_count.as_u64() as i64;
        } else {
            *nonce += 1;
        }

        Ok(*nonce as u64)
    }
}

#[async_trait]
impl TransactionCaller for AdapterClient {
    async fn call_contract_function(
        &self,
        info: &str,
        call: ContractCall<WalletSigner, ()>,
    ) -> ContractClientResult<H256> {
        let pending_tx = call.send().await.map_err(|e| {
            let e: ContractClientError = e.into();
            e
        })?;

        info!(
            "Calling contract function {}: {:?}",
            info,
            pending_tx.tx_hash()
        );

        let receipt = pending_tx
            .await
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?
            .ok_or(ContractClientError::NoTransactionReceipt)?;

        if receipt.status == Some(U64::from(0)) {
            return Err(ContractClientError::TransactionFailed);
        } else {
            info!("Transaction successful({}), receipt: {:?}", info, receipt);
        }

        Ok(receipt.transaction_hash)
    }
}

#[allow(unused_variables)]
#[async_trait]
impl AdapterTransactions for AdapterClient {
    async fn request_randomness(&self, seed: U256) -> ContractClientResult<H256> {
        panic!("Not implemented");
    }

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
            .iter()
            .map(|(address, ps)| {
                let sig: U256 = U256::from(ps.signature.as_slice());
                ContractPartialSignature {
                    index: ps.index.into(),
                    partial_signature: sig,
                }
            })
            .collect();

        let nonce = self.increment_or_initialize_nonce().await?;
        let mut call = adapter_contract.fulfill_randomness(group_index.into(), r_id, sig, ps);
        call.tx.set_nonce(nonce);

        self.call_contract_function("fulfill_randomness", call)
            .await
    }
}

#[allow(unused_variables)]
#[async_trait]
impl<C: PairingCurve> AdapterViews<C> for AdapterClient {
    async fn get_group(&self, group_index: usize) -> ContractClientResult<Group<C>> {
        let adapter_contract =
            ServiceClient::<AdapterContract>::prepare_service_client(self).await?;

        let res = adapter_contract
            .get_group(group_index.into())
            .call()
            .await
            .map(parse_contract_group)
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;

        Ok(res)
    }

    async fn get_last_output(&self) -> ContractClientResult<U256> {
        let adapter_contract =
            ServiceClient::<AdapterContract>::prepare_service_client(self).await?;

        let res = adapter_contract.last_output().call().await.map_err(|e| {
            let e: ContractClientError = e.into();
            e
        })?;

        Ok(res)
    }

    async fn is_task_pending(&self, request_id: &[u8]) -> ContractClientResult<bool> {
        let adapter_contract =
            ServiceClient::<AdapterContract>::prepare_service_client(self).await?;

        let r_id = pad_to_bytes32(request_id).unwrap();

        let res = adapter_contract
            .get_pending_request(r_id)
            .call()
            .await
            .map(|r| r.sub_id != 0)
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;

        Ok(res)
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

fn parse_contract_group<C: PairingCurve>(cg: ContractGroup) -> Group<C> {
    let ContractGroup {
        index,
        epoch,
        size,
        threshold,
        public_key,
        members,
        committers,
        commit_cache_list: _,
        is_strictly_majority_consensus_reached,
    } = cg;

    let members: BTreeMap<Address, Member<C>> = members
        .into_iter()
        .enumerate()
        .map(|(index, cm)| {
            let partial_public_key =
                if cm.partial_public_key.is_empty() || cm.partial_public_key[0] == U256::zero() {
                    None
                } else {
                    let bytes = cm
                        .partial_public_key
                        .iter()
                        .map(u256_to_vec)
                        .reduce(|mut acc, mut e| {
                            acc.append(&mut e);
                            acc
                        })
                        .unwrap();
                    Some(bincode::deserialize(&bytes).unwrap())
                };

            let m = Member {
                index,
                id_address: cm.node_id_address,
                rpc_endpoint: None,
                partial_public_key,
            };
            (m.id_address, m)
        })
        .collect();

    let public_key = if public_key.is_empty() || public_key[0] == U256::zero() {
        None
    } else {
        let bytes = public_key
            .iter()
            .map(u256_to_vec)
            .reduce(|mut acc, mut e| {
                acc.append(&mut e);
                acc
            })
            .unwrap();
        Some(bincode::deserialize(&bytes).unwrap())
    };

    Group {
        index: index.as_usize(),
        epoch: epoch.as_usize(),
        size: size.as_usize(),
        threshold: threshold.as_usize(),
        state: is_strictly_majority_consensus_reached,
        public_key,
        members,
        committers,
        c: PhantomData,
    }
}
