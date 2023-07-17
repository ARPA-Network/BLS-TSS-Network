use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    error::{NodeError, NodeResult},
    event::{ready_to_fulfill_randomness_task::ReadyToFulfillRandomnessTask, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use arpa_contract_client::adapter::{AdapterClientBuilder, AdapterTransactions, AdapterViews};
use arpa_core::{ChainIdentity, PartialSignature, RandomnessTask, SubscriberType, TaskType};
use arpa_dal::{cache::RandomnessResultCache, BLSResultCacheState, SignatureResultCacheUpdater};
use async_trait::async_trait;
use ethers::types::Address;
use log::{debug, error, info};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use threshold_bls::{group::PairingCurve, poly::Eval};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct RandomnessSignatureAggregationSubscriber<
    I: ChainIdentity + AdapterClientBuilder,
    C: SignatureResultCacheUpdater<RandomnessResultCache>,
    PC: PairingCurve,
> {
    pub chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
    randomness_signature_cache: Arc<RwLock<C>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<PC>,
}

impl<
        I: ChainIdentity + AdapterClientBuilder,
        C: SignatureResultCacheUpdater<RandomnessResultCache>,
        PC: PairingCurve,
    > RandomnessSignatureAggregationSubscriber<I, C, PC>
{
    pub fn new(
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<I>>,
        randomness_signature_cache: Arc<RwLock<C>>,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        RandomnessSignatureAggregationSubscriber {
            chain_id,
            id_address,
            chain_identity,
            randomness_signature_cache,
            eq,
            ts,
            c: PhantomData,
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

pub struct GeneralFulfillRandomnessHandler<
    I: ChainIdentity + AdapterClientBuilder,
    C: SignatureResultCacheUpdater<RandomnessResultCache>,
> {
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
    randomness_signature_cache: Arc<RwLock<C>>,
}

#[async_trait]
impl<
        I: ChainIdentity + AdapterClientBuilder + Sync + Send,
        C: SignatureResultCacheUpdater<RandomnessResultCache> + Sync + Send,
    > FulfillRandomnessHandler for GeneralFulfillRandomnessHandler<I, C>
{
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

        let randomness_task_request_id = randomness_task.request_id.clone();

        if client.is_task_pending(&randomness_task_request_id).await? {
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
                Ok(tx_hash) => {
                    self.randomness_signature_cache
                        .write()
                        .await
                        .update_commit_result(
                            &randomness_task_request_id,
                            BLSResultCacheState::Committed,
                        )
                        .await?;

                    info!("fulfill randomness successfully! tx_hash:{:?}, task request id: {}, group_index: {}, signature: {}",
                    tx_hash, format!("{:?}",hex::encode(randomness_task_request_id)), group_index, hex::encode(signature));
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
                    error!("{:?}", e);
                }
            }
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
        I: ChainIdentity + AdapterClientBuilder + std::fmt::Debug + Sync + Send + 'static,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + std::fmt::Debug
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Sync + Send + 'static,
    > Subscriber for RandomnessSignatureAggregationSubscriber<I, C, PC>
{
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let ReadyToFulfillRandomnessTask {
            tasks: ready_signatures,
            ..
        } = payload
            .as_any()
            .downcast_ref::<ReadyToFulfillRandomnessTask>()
            .unwrap();

        for signature in ready_signatures {
            let RandomnessResultCache {
                group_index,
                randomness_task,
                message: _,
                threshold,
                partial_signatures,
            } = signature.clone();

            let partials = partial_signatures
                .values()
                .cloned()
                .collect::<Vec<Vec<u8>>>();

            let signature = SimpleBLSCore::<PC>::aggregate(threshold, &partials)?;

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

            let chain_identity = self.chain_identity.clone();

            let randomness_signature_cache = self.randomness_signature_cache.clone();

            self.ts.write().await.add_task(
                TaskType::Subscriber(SubscriberType::RandomnessSignatureAggregation),
                async move {
                    let handler = GeneralFulfillRandomnessHandler {
                        id_address,
                        chain_identity,
                        randomness_signature_cache,
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
        I: ChainIdentity + AdapterClientBuilder + std::fmt::Debug + Sync + Send + 'static,
        C: SignatureResultCacheUpdater<RandomnessResultCache>
            + std::fmt::Debug
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Sync + Send + 'static,
    > DebuggableSubscriber for RandomnessSignatureAggregationSubscriber<I, C, PC>
{
}
