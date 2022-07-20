use super::types::Subscriber;
use crate::node::{
    algorithm::bls::{BLSCore, MockBLSCore},
    contract_client::adapter_client::{AdapterTransactions, AdapterViews, MockAdapterClient},
    dao::{cache::RandomnessResultCache, types::ChainIdentity},
    error::errors::NodeResult,
    event::{
        ready_to_fulfill_randomness_task::ReadyToFulfillRandomnessTask,
        types::{Event, Topic},
    },
    queue::event_queue::{EventQueue, EventSubscriber},
    scheduler::dynamic::{DynamicTaskScheduler, SimpleDynamicTaskScheduler},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};

pub struct RandomnessSignatureAggregationSubscriber {
    pub chain_id: usize,
    id_address: String,
    chain_identity: Arc<RwLock<ChainIdentity>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl RandomnessSignatureAggregationSubscriber {
    pub fn new(
        chain_id: usize,
        id_address: String,
        chain_identity: Arc<RwLock<ChainIdentity>>,
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
        partial_signatures: HashMap<String, Vec<u8>>,
    ) -> NodeResult<()>;
}

pub struct MockFulfillRandomnessHandler {
    adapter_address: String,
    id_address: String,
}

#[async_trait]
impl FulfillRandomnessHandler for MockFulfillRandomnessHandler {
    async fn handle(
        &self,
        group_index: usize,
        randomness_task_index: usize,
        signature: Vec<u8>,
        partial_signatures: HashMap<String, Vec<u8>>,
    ) -> NodeResult<()> {
        let mut client =
            MockAdapterClient::new(self.adapter_address.clone(), self.id_address.clone()).await?;

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
                    println!("fulfill randomness successfully! signature index: {}, group_index: {}, signature: {}",
                        randomness_task_index, group_index, hex::encode(signature));
                }
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        }

        Ok(())
    }
}

impl Subscriber for RandomnessSignatureAggregationSubscriber {
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        println!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut ReadyToFulfillRandomnessTask;

            let ReadyToFulfillRandomnessTask {
                chain_id: _,
                tasks: ready_signatures,
            } = *Box::from_raw(struct_ptr);

            for signature in ready_signatures {
                let RandomnessResultCache {
                    group_index,
                    randomness_task_index,
                    message: _,
                    threshold,
                    partial_signatures,
                } = signature;

                let bls_core = MockBLSCore {};

                let signature = bls_core.aggregate(
                    threshold,
                    &partial_signatures.values().cloned().collect::<Vec<_>>(),
                )?;

                let adapter_address = self
                    .chain_identity
                    .read()
                    .get_provider_rpc_endpoint()
                    .to_string();

                let id_address = self.id_address.clone();

                self.ts.write().add_task(async move {
                    let handler = MockFulfillRandomnessHandler {
                        adapter_address,
                        id_address,
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
                        println!("{:?}", e);
                    }
                });
            }
        }

        Ok(())
    }

    fn subscribe(self) {
        let eq = self.eq.clone();

        let chain_id = self.chain_id;

        let subscriber = Box::new(self);

        eq.write()
            .subscribe(Topic::ReadyToFulfillRandomnessTask(chain_id), subscriber);
    }
}
