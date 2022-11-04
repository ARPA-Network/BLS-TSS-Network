use super::Subscriber;
use crate::node::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    error::NodeResult,
    event::{ready_to_fulfill_randomness_task::ReadyToFulfillRandomnessTask, types::Topic, Event},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use arpa_node_contract_client::adapter::{AdapterClientBuilder, AdapterTransactions, AdapterViews};
use arpa_node_core::ChainIdentity;
use arpa_node_dal::cache::RandomnessResultCache;
use async_trait::async_trait;
use ethers::types::Address;
use log::{debug, error, info};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

pub struct RandomnessSignatureAggregationSubscriber<I: ChainIdentity + AdapterClientBuilder> {
    pub chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl<I: ChainIdentity + AdapterClientBuilder> RandomnessSignatureAggregationSubscriber<I> {
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
impl<I: ChainIdentity + AdapterClientBuilder + Sync + Send + 'static> Subscriber
    for RandomnessSignatureAggregationSubscriber<I>
{
    async fn notify(&self, topic: Topic, payload: &(dyn Event + Send + Sync)) -> NodeResult<()> {
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

            let bls_core = SimpleBLSCore {};

            let signature = bls_core.aggregate(
                threshold,
                &partial_signatures.values().cloned().collect::<Vec<_>>(),
            )?;

            let id_address = self.id_address;

            let chain_identity = self.chain_identity.clone();

            self.ts.write().await.add_task(async move {
                let handler = GeneralFulfillRandomnessHandler {
                    id_address,
                    chain_identity,
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
            });
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
