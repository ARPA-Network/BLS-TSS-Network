use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::{dkg_post_process::DKGPostProcess, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use arpa_contract_client::{
    controller::{ControllerTransactions, ControllerViews},
    controller_relayer::ControllerRelayerTransactions,
};
use arpa_core::{DKGStatus, SubscriberType, TaskType, PLACEHOLDER_ADDRESS};
use arpa_dal::GroupInfoHandler;
use arpa_log::*;
use async_trait::async_trait;
use log::{debug, error, info};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PostGroupingSubscriber<PC: Curve> {
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    supported_relayed_chains: Vec<usize>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<PC>,
}

impl<PC: Curve> PostGroupingSubscriber<PC> {
    pub fn new(
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        supported_relayed_chains: Vec<usize>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        PostGroupingSubscriber {
            chain_identity,
            supported_relayed_chains,
            group_cache,
            eq,
            ts,
            c: PhantomData,
        }
    }
}

#[async_trait]
pub trait DKGPostProcessHandler {
    async fn handle(&self, group_index: usize, group_epoch: usize) -> NodeResult<()>;
}

pub struct GeneralDKGPostProcessHandler<PC: Curve> {
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    supported_relayed_chains: Vec<usize>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    c: PhantomData<PC>,
}

#[async_trait]
impl<PC: Curve + Sync + Send + 'static> DKGPostProcessHandler for GeneralDKGPostProcessHandler<PC> {
    #[log_function]
    async fn handle(&self, group_index: usize, group_epoch: usize) -> NodeResult<()> {
        if self
            .group_cache
            .write()
            .await
            .update_dkg_status(group_index, group_epoch, DKGStatus::None)
            .await?
        {
            info!(
                "DKG status updated to None for group {} epoch {}",
                group_index, group_epoch
            );

            let controller_client = self.chain_identity.read().await.build_controller_client();

            let controller_relayer_client = self
                .chain_identity
                .read()
                .await
                .build_controller_relayer_client();

            if PLACEHOLDER_ADDRESS
                != ControllerViews::<PC>::get_coordinator(&controller_client, group_index).await?
            {
                controller_client
                    .post_process_dkg(group_index, group_epoch)
                    .await?;

                for relayed_chain_id in self.supported_relayed_chains.iter() {
                    controller_relayer_client
                        .relay_group(*relayed_chain_id, group_index)
                        .await?;
                }
            };
        }

        Ok(())
    }
}

#[async_trait]
impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> Subscriber
    for PostGroupingSubscriber<PC>
{
    #[log_function]
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let &DKGPostProcess {
            group_index,
            group_epoch,
        } = payload.as_any().downcast_ref::<DKGPostProcess>().unwrap();

        let chain_identity = self.chain_identity.clone();
        let supported_relayed_chains = self.supported_relayed_chains.clone();
        let group_cache = self.group_cache.clone();

        self.ts.write().await.add_task(TaskType::Subscriber(self.chain_identity.read().await.get_chain_id(), SubscriberType::PostGrouping),async move {
                let handler = GeneralDKGPostProcessHandler {
                    chain_identity,
                    supported_relayed_chains,
                    group_cache,
                    c: PhantomData,
                };

                if let Err(e) = handler.handle(group_index, group_epoch).await {
                    error!("{:?}", e);
                } else {
                    info!("-------------------------call post process successfully-------------------------");
                }
            })?;

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write()
            .await
            .subscribe(Topic::DKGPostProcess, subscriber);
    }
}

impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> DebuggableSubscriber
    for PostGroupingSubscriber<PC>
{
}
