use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::node::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    error::{NodeError, NodeResult},
    event::{ready_to_fulfill_randomness_task::ReadyToFulfillRandomnessTask, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, SubscriberType, TaskScheduler, TaskType},
};
use arpa_node_contract_client::adapter::{AdapterClientBuilder, AdapterTransactions, AdapterViews};
use arpa_node_core::{ChainIdentity, PartialSignature};
use arpa_node_dal::cache::RandomnessResultCache;
use async_trait::async_trait;
use ethers::types::Address;
use log::{debug, error, info};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use threshold_bls::{group::PairingCurve, poly::Eval};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct RandomnessSignatureAggregationSubscriber<
    I: ChainIdentity + AdapterClientBuilder,
    C: PairingCurve,
> {
    pub chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<C>,
}

impl<I: ChainIdentity + AdapterClientBuilder, C: PairingCurve>
    RandomnessSignatureAggregationSubscriber<I, C>
{
    pub fn new(
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<I>>,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        RandomnessSignatureAggregationSubscriber {
            chain_id,
            id_address,
            chain_identity,
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
        randomness_task_request_id: Vec<u8>,
        signature: Vec<u8>,
        partial_signatures: HashMap<Address, PartialSignature>,
    ) -> NodeResult<()>;
}

pub struct GeneralFulfillRandomnessHandler<I: ChainIdentity + AdapterClientBuilder> {
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
}

#[async_trait]
impl<I: ChainIdentity + AdapterClientBuilder + Sync + Send> FulfillRandomnessHandler
    for GeneralFulfillRandomnessHandler<I>
{
    async fn handle(
        &self,
        group_index: usize,
        randomness_task_request_id: Vec<u8>,
        signature: Vec<u8>,
        partial_signatures: HashMap<Address, PartialSignature>,
    ) -> NodeResult<()> {
        let client = self
            .chain_identity
            .read()
            .await
            .build_adapter_client(self.id_address);

        if client.is_task_pending(&randomness_task_request_id).await? {
            match client
                .fulfill_randomness(
                    group_index,
                    randomness_task_request_id.clone(),
                    signature.clone(),
                    partial_signatures,
                )
                .await
            {
                Ok(tx_hash) => {
                    info!("fulfill randomness successfully! tx_hash:{:?}, task request id: {}, group_index: {}, signature: {}",
                    tx_hash, format!("{:?}",hex::encode(randomness_task_request_id)), group_index, hex::encode(signature));
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<
        I: ChainIdentity + AdapterClientBuilder + std::fmt::Debug + Sync + Send + 'static,
        C: PairingCurve + std::fmt::Debug + Sync + Send + 'static,
    > Subscriber for RandomnessSignatureAggregationSubscriber<I, C>
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
                randomness_task_request_id,
                message: _,
                threshold,
                partial_signatures,
            } = signature.clone();

            let partials = partial_signatures
                .values()
                .cloned()
                .collect::<Vec<Vec<u8>>>();

            let signature = SimpleBLSCore::<C>::aggregate(threshold, &partials)?;

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

            self.ts.write().await.add_task(
                TaskType::Subscriber(SubscriberType::RandomnessSignatureAggregation),
                async move {
                    let handler = GeneralFulfillRandomnessHandler {
                        id_address,
                        chain_identity,
                    };

                    if let Err(e) = handler
                        .handle(
                            group_index,
                            randomness_task_request_id,
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
        C: PairingCurve + std::fmt::Debug + Sync + Send + 'static,
    > DebuggableSubscriber for RandomnessSignatureAggregationSubscriber<I, C>
{
}
