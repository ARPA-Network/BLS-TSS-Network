use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::node::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    error::NodeResult,
    event::{ready_to_fulfill_randomness_task::ReadyToFulfillRandomnessTask, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, SubscriberType, TaskScheduler, TaskType},
};
use arpa_node_contract_client::adapter::{AdapterClientBuilder, AdapterTransactions, AdapterViews};
use arpa_node_core::ChainIdentity;
use arpa_node_dal::cache::RandomnessResultCache;
use async_trait::async_trait;
use ethers::types::Address;
use log::{debug, error, info};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct RandomnessSignatureAggregationSubscriber<
    I: ChainIdentity + AdapterClientBuilder<C>,
    C: PairingCurve,
> {
    pub chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<C>,
}

impl<I: ChainIdentity + AdapterClientBuilder<C>, C: PairingCurve>
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
        randomness_task_index: usize,
        signature: Vec<u8>,
        partial_signatures: HashMap<Address, Vec<u8>>,
    ) -> NodeResult<()>;
}

pub struct GeneralFulfillRandomnessHandler<
    I: ChainIdentity + AdapterClientBuilder<C>,
    C: PairingCurve,
> {
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
    c: PhantomData<C>,
}

#[async_trait]
impl<I: ChainIdentity + AdapterClientBuilder<C> + Sync + Send, C: PairingCurve + Sync + Send>
    FulfillRandomnessHandler for GeneralFulfillRandomnessHandler<I, C>
{
    async fn handle(
        &self,
        group_index: usize,
        randomness_task_index: usize,
        signature: Vec<u8>,
        partial_signatures: HashMap<Address, Vec<u8>>,
    ) -> NodeResult<()> {
        let client = self
            .chain_identity
            .read()
            .await
            .build_adapter_client(self.id_address);

        if !client
            .get_signature_task_completion_state(randomness_task_index)
            .await?
        {
            match client
                .fulfill_randomness(
                    group_index,
                    randomness_task_index,
                    signature.clone(),
                    partial_signatures,
                )
                .await
            {
                Ok(()) => {
                    info!("fulfill randomness successfully! signature index: {}, group_index: {}, signature: {}",
                        randomness_task_index, group_index, hex::encode(signature));
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
        I: ChainIdentity + AdapterClientBuilder<C> + std::fmt::Debug + Sync + Send + 'static,
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
                randomness_task_index,
                message: _,
                threshold,
                partial_signatures,
            } = signature.clone();

            let signature = SimpleBLSCore::<C>::aggregate(
                threshold,
                &partial_signatures.values().cloned().collect::<Vec<_>>(),
            )?;

            let id_address = self.id_address;

            let chain_identity = self.chain_identity.clone();

            self.ts.write().await.add_task(
                TaskType::Subscriber(SubscriberType::RandomnessSignatureAggregation),
                async move {
                    let handler = GeneralFulfillRandomnessHandler {
                        id_address,
                        chain_identity,
                        c: PhantomData,
                    };

                    if let Err(e) = handler
                        .handle(
                            group_index,
                            randomness_task_index,
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
        I: ChainIdentity + AdapterClientBuilder<C> + std::fmt::Debug + Sync + Send + 'static,
        C: PairingCurve + std::fmt::Debug + Sync + Send + 'static,
    > DebuggableSubscriber for RandomnessSignatureAggregationSubscriber<I, C>
{
}
