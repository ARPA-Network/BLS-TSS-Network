use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    context::ChainIdentityHandlerType,
    error::{NodeError, NodeResult},
    event::{ready_to_fulfill_randomness_task::ReadyToFulfillRandomnessTask, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use arpa_contract_client::{
    adapter::{AdapterTransactions, AdapterViews},
    error::ContractClientError,
};
use arpa_core::{
    log::{build_request_related_payload, build_transaction_receipt_log_payload, LogType},
    BLSTaskType, PartialSignature, RandomnessTask, SubscriberType, TaskType,
    DEFAULT_MAX_RANDOMNESS_FULFILLMENT_ATTEMPTS,
};
use arpa_dal::{cache::RandomnessResultCache, BLSResultCacheState};
use arpa_dal::{BlockInfoHandler, SignatureResultCacheHandler};
use async_trait::async_trait;
use ethers::types::{Address, U256};
use log::{debug, error, info};
use serde_json::json;
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use threshold_bls::{
    group::Curve,
    poly::Eval,
    sig::{SignatureScheme, ThresholdScheme},
};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct RandomnessSignatureAggregationSubscriber<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
> {
    chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    randomness_signature_cache:
        Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<PC>,
    s: PhantomData<S>,
}

impl<PC: Curve, S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>>
    RandomnessSignatureAggregationSubscriber<PC, S>
{
    pub fn new(
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
        randomness_signature_cache: Arc<
            RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>,
        >,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        RandomnessSignatureAggregationSubscriber {
            chain_id,
            id_address,
            chain_identity,
            block_cache,
            randomness_signature_cache,
            eq,
            ts,
            c: PhantomData,
            s: PhantomData,
        }
    }
}

#[async_trait]
pub trait FulfillRandomnessHandler {
    async fn handle(
        &self,
        group_index: usize,
        randomness_task: RandomnessTask,
        signature: Vec<u8>,
        partial_signatures: HashMap<Address, PartialSignature>,
    ) -> NodeResult<()>;
}

pub struct GeneralFulfillRandomnessHandler<PC: Curve> {
    id_address: Address,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    randomness_signature_cache:
        Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>>,
    pc: PhantomData<PC>,
}

#[async_trait]
impl<PC: Curve> FulfillRandomnessHandler for GeneralFulfillRandomnessHandler<PC> {
    async fn handle(
        &self,
        group_index: usize,
        randomness_task: RandomnessTask,
        signature: Vec<u8>,
        partial_signatures: HashMap<Address, PartialSignature>,
    ) -> NodeResult<()> {
        let client = self
            .chain_identity
            .read()
            .await
            .build_adapter_client(self.id_address);

        let chain_id = self.chain_identity.read().await.get_chain_id();

        let randomness_task_request_id = randomness_task.request_id.clone();

        let randomness_task_json = json!(randomness_task);

        if client.is_task_pending(&randomness_task_request_id).await? {
            if self.block_cache.read().await.get_block_height()
                - randomness_task.assignment_block_height
                > 86400 / self.block_cache.read().await.get_block_time()
            {
                self.randomness_signature_cache
                    .write()
                    .await
                    .update_commit_result(&randomness_task_request_id, BLSResultCacheState::Expired)
                    .await?;

                info!("mark randomness task as expired. task request id: {}, assignment_block_height:{:?}",
                    format!("{:?}",hex::encode(randomness_task_request_id)), randomness_task.assignment_block_height);

                return Ok(());
            }

            let wei_per_gas = self
                .chain_identity
                .read()
                .await
                .get_current_gas_price()
                .await?;

            if wei_per_gas > randomness_task.callback_max_gas_price {
                self.randomness_signature_cache
                    .write()
                    .await
                    .update_commit_result(
                        &randomness_task_request_id,
                        BLSResultCacheState::NotCommitted,
                    )
                    .await?;

                info!("cancel fulfilling randomness as gas price is too high! task request id: {}, current_gas_price:{:?}, max_gas_price: {:?}",
                    format!("{:?}",hex::encode(randomness_task_request_id)), wei_per_gas, randomness_task.callback_max_gas_price);

                return Ok(());
            }

            match client
                .fulfill_randomness(
                    group_index,
                    randomness_task,
                    signature.clone(),
                    partial_signatures,
                )
                .await
            {
                Ok(receipt) => {
                    self.randomness_signature_cache
                        .write()
                        .await
                        .update_commit_result(
                            &randomness_task_request_id,
                            BLSResultCacheState::Committed,
                        )
                        .await?;

                    info!(
                        "{}",
                        build_transaction_receipt_log_payload(
                            LogType::FulfillmentFinished,
                            "Randomness fulfilled successfully.",
                            chain_id,
                            &randomness_task_request_id,
                            BLSTaskType::Randomness,
                            randomness_task_json,
                            receipt.transaction_hash,
                            receipt.gas_used.unwrap_or(U256::zero()),
                            receipt.effective_gas_price.unwrap_or(U256::zero()),
                        )
                    );
                }
                Err(e) => {
                    self.randomness_signature_cache
                        .write()
                        .await
                        .update_commit_result(
                            &randomness_task_request_id,
                            BLSResultCacheState::NotCommitted,
                        )
                        .await?;

                    match e {
                        ContractClientError::TransactionFailed(receipt) => {
                            error!(
                                "{}",
                                build_transaction_receipt_log_payload(
                                    LogType::FulfillmentFailed,
                                    "Randomness fulfillment reverted.",
                                    chain_id,
                                    &randomness_task_request_id,
                                    BLSTaskType::Randomness,
                                    randomness_task_json,
                                    receipt.transaction_hash,
                                    receipt.gas_used.unwrap_or(U256::zero()),
                                    receipt.effective_gas_price.unwrap_or(U256::zero()),
                                )
                            );
                        }
                        _ => {
                            error!(
                                "{}",
                                build_transaction_receipt_log_payload(
                                    LogType::FulfillmentFailed,
                                    &format!("Randomness fulfillment failed with error: {:?}", e),
                                    chain_id,
                                    &randomness_task_request_id,
                                    BLSTaskType::Randomness,
                                    randomness_task_json,
                                    Default::default(),
                                    U256::zero(),
                                    U256::zero(),
                                )
                            );
                        }
                    }
                }
            }

            self.randomness_signature_cache
                .write()
                .await
                .incr_committed_times(&randomness_task_request_id)
                .await?;
        } else {
            self.randomness_signature_cache
                .write()
                .await
                .update_commit_result(
                    &randomness_task_request_id,
                    BLSResultCacheState::CommittedByOthers,
                )
                .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl<
        PC: Curve + std::fmt::Debug + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Sync
            + Send
            + 'static,
    > Subscriber for RandomnessSignatureAggregationSubscriber<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let ReadyToFulfillRandomnessTask {
            tasks: ready_signature_caches,
            ..
        } = payload
            .as_any()
            .downcast_ref::<ReadyToFulfillRandomnessTask>()
            .unwrap();

        for ready_signature_cache in ready_signature_caches {
            let RandomnessResultCache {
                group_index,
                randomness_task,
                message: _,
                threshold,
                partial_signatures,
                committed_times,
            } = ready_signature_cache.clone();

            if committed_times >= DEFAULT_MAX_RANDOMNESS_FULFILLMENT_ATTEMPTS {
                self.randomness_signature_cache
                    .write()
                    .await
                    .update_commit_result(&randomness_task.request_id, BLSResultCacheState::FAULTY)
                    .await?;

                error!("mark randomness task as faulty for too many failed fulfillment attempts. task request id: {}",
                    format!("{:?}",hex::encode(&randomness_task.request_id)));

                continue;
            }

            let partials = partial_signatures
                .values()
                .cloned()
                .collect::<Vec<Vec<u8>>>();

            if let Ok(signature) = SimpleBLSCore::<PC, S>::aggregate(threshold, &partials) {
                info!(
                    "{}",
                    build_request_related_payload(
                        LogType::AggregatedSignatureFinished,
                        "Randomness signature aggregated successfully.",
                        self.chain_id,
                        &randomness_task.request_id,
                        BLSTaskType::Randomness,
                        json!(randomness_task),
                        None
                    )
                );

                let partial_signatures = partial_signatures
                    .iter()
                    .map(|(addr, partial)| {
                        let eval: Eval<Vec<u8>> = bincode::deserialize(partial)?;
                        let partial = PartialSignature {
                            index: eval.index as usize,
                            signature: eval.value,
                        };
                        Ok((*addr, partial))
                    })
                    .collect::<Result<_, NodeError>>()?;

                let id_address = self.id_address;

                let block_cache = self.block_cache.clone();

                let chain_identity = self.chain_identity.clone();

                let randomness_signature_cache = self.randomness_signature_cache.clone();

                self.ts.write().await.add_task(
                    TaskType::Subscriber(
                        self.chain_identity.read().await.get_chain_id(),
                        SubscriberType::RandomnessSignatureAggregation,
                    ),
                    async move {
                        let handler = GeneralFulfillRandomnessHandler {
                            id_address,
                            chain_identity,
                            block_cache,
                            randomness_signature_cache,
                            pc: PhantomData,
                        };

                        if let Err(e) = handler
                            .handle(
                                group_index,
                                randomness_task,
                                signature.clone(),
                                partial_signatures,
                            )
                            .await
                        {
                            error!("{:?}", e);
                        }
                    },
                )?;
            } else {
                error!(
                    "{}",
                    build_request_related_payload(
                        LogType::AggregatedSignatureFailed,
                        "Randomness signature aggregation failed.",
                        self.chain_id,
                        &randomness_task.request_id,
                        BLSTaskType::Randomness,
                        json!(randomness_task),
                        None
                    )
                );
                continue;
            }
        }

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let chain_id = self.chain_id;

        let subscriber = Box::new(self);

        eq.write()
            .await
            .subscribe(Topic::ReadyToFulfillRandomnessTask(chain_id), subscriber);
    }
}

impl<
        PC: Curve + std::fmt::Debug + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Sync
            + Send
            + 'static,
    > DebuggableSubscriber for RandomnessSignatureAggregationSubscriber<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
}
