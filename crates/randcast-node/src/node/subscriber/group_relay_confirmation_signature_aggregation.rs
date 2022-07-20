use super::types::Subscriber;
use crate::node::{
    algorithm::bls::{BLSCore, MockBLSCore},
    contract_client::adapter_client::{AdapterTransactions, MockAdapterClient},
    dao::{cache::GroupRelayConfirmationResultCache, types::ChainIdentity},
    error::errors::NodeResult,
    event::{
        ready_to_fulfill_group_relay_confirmation_task::ReadyToFulfillGroupRelayConfirmationTask,
        types::{Event, Topic},
    },
    queue::event_queue::{EventQueue, EventSubscriber},
    scheduler::dynamic::{DynamicTaskScheduler, SimpleDynamicTaskScheduler},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct GroupRelayConfirmationSignatureAggregationSubscriber {
    pub chain_id: usize,
    id_address: String,
    chain_identity: Arc<RwLock<ChainIdentity>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl GroupRelayConfirmationSignatureAggregationSubscriber {
    pub fn new(
        chain_id: usize,
        id_address: String,
        chain_identity: Arc<RwLock<ChainIdentity>>,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        GroupRelayConfirmationSignatureAggregationSubscriber {
            chain_id,
            id_address,
            chain_identity,
            eq,
            ts,
        }
    }
}

#[async_trait]
pub trait FulfillGroupRelayConfirmationHandler {
    async fn handle(
        &self,
        group_index: usize,
        group_relay_task_index: usize,
        signature: Vec<u8>,
        group_as_bytes: Vec<u8>,
    ) -> NodeResult<()>;
}

pub struct MockFulfillGroupRelayConfirmationHandler {
    adapter_address: String,
    id_address: String,
}

#[async_trait]
impl FulfillGroupRelayConfirmationHandler for MockFulfillGroupRelayConfirmationHandler {
    async fn handle(
        &self,
        group_index: usize,
        group_relay_confirmation_task_index: usize,
        signature: Vec<u8>,
        group_relay_confirmation_as_bytes: Vec<u8>,
    ) -> NodeResult<()> {
        let mut client =
            MockAdapterClient::new(self.adapter_address.clone(), self.id_address.clone()).await?;

        match client
            .confirm_relay(
                group_relay_confirmation_task_index,
                group_relay_confirmation_as_bytes,
                signature,
            )
            .await
        {
            Ok(()) => {
                println!("fulfill group_relay_confirmation successfully! task index: {}, group_index: {}",
                        group_relay_confirmation_task_index, group_index);
            }
            Err(e) => {
                println!("{:?}", e);
            }
        }

        Ok(())
    }
}

impl Subscriber for GroupRelayConfirmationSignatureAggregationSubscriber {
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        println!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut ReadyToFulfillGroupRelayConfirmationTask;

            let ReadyToFulfillGroupRelayConfirmationTask {
                chain_id: _,
                tasks: ready_signatures,
            } = *Box::from_raw(struct_ptr);

            for signature in ready_signatures {
                let GroupRelayConfirmationResultCache {
                    group_index,
                    group_relay_confirmation_task_index,
                    group_relay_confirmation,
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

                let group_relay_confirmation_as_bytes =
                    bincode::serialize(&group_relay_confirmation)?;

                self.ts.write().add_task(async move {
                    let handler = MockFulfillGroupRelayConfirmationHandler {
                        adapter_address,
                        id_address,
                    };

                    if let Err(e) = handler
                        .handle(
                            group_index,
                            group_relay_confirmation_task_index,
                            signature.clone(),
                            group_relay_confirmation_as_bytes,
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

        eq.write().subscribe(
            Topic::ReadyToFulfillGroupRelayConfirmationTask(chain_id),
            subscriber,
        );
    }
}
