use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::node::{
    error::NodeResult,
    event::{dkg_post_process::DKGPostProcess, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use arpa_node_contract_client::controller::{
    ControllerClientBuilder, ControllerTransactions, ControllerViews,
};
use arpa_node_core::{ChainIdentity, DKGStatus, SubscriberType, TaskType, PALCEHOLDER_ADDRESS};
use arpa_node_dal::GroupInfoUpdater;
use arpa_node_log::*;
use async_trait::async_trait;
use log::{debug, error, info};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PostGroupingSubscriber<
    I: ChainIdentity + ControllerClientBuilder<C>,
    G: GroupInfoUpdater<C>,
    C: PairingCurve,
> {
    main_chain_identity: Arc<RwLock<I>>,
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<C>,
}

impl<I: ChainIdentity + ControllerClientBuilder<C>, G: GroupInfoUpdater<C>, C: PairingCurve>
    PostGroupingSubscriber<I, G, C>
{
    pub fn new(
        main_chain_identity: Arc<RwLock<I>>,
        group_cache: Arc<RwLock<G>>,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        PostGroupingSubscriber {
            main_chain_identity,
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

pub struct GeneralDKGPostProcessHandler<
    I: ChainIdentity + ControllerClientBuilder<C>,
    G: GroupInfoUpdater<C>,
    C: PairingCurve,
> {
    main_chain_identity: Arc<RwLock<I>>,
    group_cache: Arc<RwLock<G>>,
    c: PhantomData<C>,
}

#[async_trait]
impl<
        I: ChainIdentity + ControllerClientBuilder<C> + Sync + Send,
        G: GroupInfoUpdater<C> + Sync + Send,
        C: PairingCurve + Sync + Send,
    > DKGPostProcessHandler for GeneralDKGPostProcessHandler<I, G, C>
{
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

            let client = self
                .main_chain_identity
                .read()
                .await
                .build_controller_client();

            if PALCEHOLDER_ADDRESS != client.get_coordinator(group_index).await? {
                client.post_process_dkg(group_index, group_epoch).await?;
            };
        }

        Ok(())
    }
}

#[async_trait]
impl<
        I: ChainIdentity + ControllerClientBuilder<C> + std::fmt::Debug + Sync + Send + 'static,
        G: GroupInfoUpdater<C> + std::fmt::Debug + Sync + Send + 'static,
        C: PairingCurve + std::fmt::Debug + Sync + Send + 'static,
    > Subscriber for PostGroupingSubscriber<I, G, C>
{
    #[log_function]
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let &DKGPostProcess {
            group_index,
            group_epoch,
        } = payload.as_any().downcast_ref::<DKGPostProcess>().unwrap();

        let main_chain_identity = self.main_chain_identity.clone();
        let group_cache = self.group_cache.clone();

        self.ts.write().await.add_task(TaskType::Subscriber(SubscriberType::PostGrouping),async move {
                let handler = GeneralDKGPostProcessHandler {
                    main_chain_identity,
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

impl<
        I: ChainIdentity + ControllerClientBuilder<C> + std::fmt::Debug + Sync + Send + 'static,
        G: GroupInfoUpdater<C> + std::fmt::Debug + Sync + Send + 'static,
        C: PairingCurve + std::fmt::Debug + Sync + Send + 'static,
    > DebuggableSubscriber for PostGroupingSubscriber<I, G, C>
{
}
