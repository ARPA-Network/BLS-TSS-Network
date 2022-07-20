use super::types::Subscriber;
use crate::node::{
    algorithm::bls::{BLSCore, MockBLSCore},
    contract_client::adapter_client::{AdapterTransactions, MockAdapterClient},
    dao::{cache::GroupRelayResultCache, types::ChainIdentity},
    error::errors::NodeResult,
    event::{
        ready_to_fulfill_group_relay_task::ReadyToFulfillGroupRelayTask,
        types::{Event, Topic},
    },
    queue::event_queue::{EventQueue, EventSubscriber},
    scheduler::dynamic::{DynamicTaskScheduler, SimpleDynamicTaskScheduler},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct GroupRelaySignatureAggregationSubscriber {
    pub chain_id: usize,
    id_address: String,
    chain_identity: Arc<RwLock<ChainIdentity>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl GroupRelaySignatureAggregationSubscriber {
    pub fn new(
        chain_id: usize,
        id_address: String,
        chain_identity: Arc<RwLock<ChainIdentity>>,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        GroupRelaySignatureAggregationSubscriber {
            chain_id,
            id_address,
            chain_identity,
            eq,
            ts,
        }
    }
}

#[async_trait]
pub trait FulfillGroupRelayHandler {
    async fn handle(
        &self,
        group_index: usize,
        group_relay_task_index: usize,
        signature: Vec<u8>,
        group_as_bytes: Vec<u8>,
    ) -> NodeResult<()>;
}

pub struct MockFulfillGroupRelayHandler {
    adapter_address: String,
    id_address: String,
}

#[async_trait]
impl FulfillGroupRelayHandler for MockFulfillGroupRelayHandler {
    async fn handle(
        &self,
        group_index: usize,
        group_relay_task_index: usize,
        signature: Vec<u8>,
        group_as_bytes: Vec<u8>,
    ) -> NodeResult<()> {
        let mut client =
            MockAdapterClient::new(self.adapter_address.clone(), self.id_address.clone()).await?;

        match client
            .fulfill_relay(
                group_index,
                group_relay_task_index,
                signature,
                group_as_bytes,
            )
            .await
        {
            Ok(()) => {
                println!(
                    "fulfill group_relay successfully! task index: {}, group_index: {}",
                    group_relay_task_index, group_index
                );
            }
            Err(e) => {
                println!("{:?}", e);
            }
        }

        Ok(())
    }
}

impl Subscriber for GroupRelaySignatureAggregationSubscriber {
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        println!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut ReadyToFulfillGroupRelayTask;

            let ReadyToFulfillGroupRelayTask {
                tasks: ready_signatures,
            } = *Box::from_raw(struct_ptr);

            for signature in ready_signatures {
                let GroupRelayResultCache {
                    group_index,
                    group_relay_task_index,
                    relayed_group,
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

                let relayed_group_as_bytes = bincode::serialize(&relayed_group)?;

                self.ts.write().add_task(async move {
                    let handler = MockFulfillGroupRelayHandler {
                        adapter_address,
                        id_address,
                    };

                    if let Err(e) = handler
                        .handle(
                            group_index,
                            group_relay_task_index,
                            signature.clone(),
                            relayed_group_as_bytes,
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

        let subscriber = Box::new(self);

        eq.write()
            .subscribe(Topic::ReadyToFulfillGroupRelayTask, subscriber);
    }
}
