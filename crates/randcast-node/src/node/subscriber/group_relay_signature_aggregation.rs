use super::Subscriber;
use crate::node::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    contract_client::adapter::{AdapterClientBuilder, AdapterTransactions},
    dal::{cache::GroupRelayResultCache, ChainIdentity},
    error::NodeResult,
    event::{ready_to_fulfill_group_relay_task::ReadyToFulfillGroupRelayTask, types::Topic, Event},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use async_trait::async_trait;
use ethers::types::Address;
use log::{error, info};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct GroupRelaySignatureAggregationSubscriber<I: ChainIdentity + AdapterClientBuilder> {
    pub chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl<I: ChainIdentity + AdapterClientBuilder> GroupRelaySignatureAggregationSubscriber<I> {
    pub fn new(
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<I>>,
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

pub struct GeneralFulfillGroupRelayHandler<I: ChainIdentity + AdapterClientBuilder> {
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
}

#[async_trait]
impl<I: ChainIdentity + AdapterClientBuilder + Sync + Send> FulfillGroupRelayHandler
    for GeneralFulfillGroupRelayHandler<I>
{
    async fn handle(
        &self,
        group_index: usize,
        group_relay_task_index: usize,
        signature: Vec<u8>,
        group_as_bytes: Vec<u8>,
    ) -> NodeResult<()> {
        let client = self
            .chain_identity
            .read()
            .build_adapter_client(self.id_address);

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
                info!(
                    "fulfill group_relay successfully! task index: {}, group_index: {}",
                    group_relay_task_index, group_index
                );
            }
            Err(e) => {
                error!("{:?}", e);
            }
        }

        Ok(())
    }
}

impl<I: ChainIdentity + AdapterClientBuilder + Sync + Send + 'static> Subscriber
    for GroupRelaySignatureAggregationSubscriber<I>
{
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        info!("{:?}", topic);

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

                let bls_core = SimpleBLSCore {};

                let signature = bls_core.aggregate(
                    threshold,
                    &partial_signatures.values().cloned().collect::<Vec<_>>(),
                )?;

                let id_address = self.id_address;

                let chain_identity = self.chain_identity.clone();

                let relayed_group_as_bytes = bincode::serialize(&relayed_group)?;

                self.ts.write().add_task(async move {
                    let handler = GeneralFulfillGroupRelayHandler {
                        id_address,
                        chain_identity,
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
                        error!("{:?}", e);
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
