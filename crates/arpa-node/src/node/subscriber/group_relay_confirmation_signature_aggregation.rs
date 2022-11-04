use super::Subscriber;
use crate::node::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    error::NodeResult,
    event::{
        ready_to_fulfill_group_relay_confirmation_task::ReadyToFulfillGroupRelayConfirmationTask,
        types::Topic, Event,
    },
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use arpa_node_contract_client::adapter::{AdapterClientBuilder, AdapterTransactions};
use arpa_node_core::ChainIdentity;
use arpa_node_dal::cache::GroupRelayConfirmationResultCache;
use async_trait::async_trait;
use ethers::types::Address;
use log::{debug, error, info};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct GroupRelayConfirmationSignatureAggregationSubscriber<
    I: ChainIdentity + AdapterClientBuilder,
> {
    pub chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl<I: ChainIdentity + AdapterClientBuilder>
    GroupRelayConfirmationSignatureAggregationSubscriber<I>
{
    pub fn new(
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<I>>,
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

pub struct GeneralFulfillGroupRelayConfirmationHandler<I: ChainIdentity + AdapterClientBuilder> {
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
}

#[async_trait]
impl<I: ChainIdentity + AdapterClientBuilder + Sync + Send> FulfillGroupRelayConfirmationHandler
    for GeneralFulfillGroupRelayConfirmationHandler<I>
{
    async fn handle(
        &self,
        group_index: usize,
        group_relay_confirmation_task_index: usize,
        signature: Vec<u8>,
        group_relay_confirmation_as_bytes: Vec<u8>,
    ) -> NodeResult<()> {
        let client = self
            .chain_identity
            .read()
            .await
            .build_adapter_client(self.id_address);

        match client
            .confirm_relay(
                group_relay_confirmation_task_index,
                group_relay_confirmation_as_bytes,
                signature,
            )
            .await
        {
            Ok(()) => {
                info!("fulfill group_relay_confirmation successfully! task index: {}, group_index: {}",
                        group_relay_confirmation_task_index, group_index);
            }
            Err(e) => {
                error!("{:?}", e);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<I: ChainIdentity + AdapterClientBuilder + Sync + Send + 'static> Subscriber
    for GroupRelayConfirmationSignatureAggregationSubscriber<I>
{
    async fn notify(&self, topic: Topic, payload: &(dyn Event + Send + Sync)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let ReadyToFulfillGroupRelayConfirmationTask {
            tasks: ready_signatures,
            ..
        } = payload
            .as_any()
            .downcast_ref::<ReadyToFulfillGroupRelayConfirmationTask>()
            .unwrap();

        for signature in ready_signatures {
            let GroupRelayConfirmationResultCache {
                group_index,
                group_relay_confirmation_task_index,
                group_relay_confirmation,
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

            let group_relay_confirmation_as_bytes = bincode::serialize(&group_relay_confirmation)?;

            self.ts.write().await.add_task(async move {
                let handler = GeneralFulfillGroupRelayConfirmationHandler {
                    id_address,
                    chain_identity,
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

        eq.write().await.subscribe(
            Topic::ReadyToFulfillGroupRelayConfirmationTask(chain_id),
            subscriber,
        );
    }
}
