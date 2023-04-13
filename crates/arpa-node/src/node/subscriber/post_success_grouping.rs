use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::node::{
    error::NodeResult,
    event::{dkg_success::DKGSuccess, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
};
use arpa_node_core::DKGStatus;
use arpa_node_dal::GroupInfoUpdater;
use async_trait::async_trait;
use log::{debug, info};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PostSuccessGroupingSubscriber<G: GroupInfoUpdater<C> + Sync + Send, C: PairingCurve> {
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
    c: PhantomData<C>,
}

impl<G: GroupInfoUpdater<C> + Sync + Send, C: PairingCurve> PostSuccessGroupingSubscriber<G, C> {
    pub fn new(group_cache: Arc<RwLock<G>>, eq: Arc<RwLock<EventQueue>>) -> Self {
        PostSuccessGroupingSubscriber {
            group_cache,
            eq,
            c: PhantomData,
        }
    }
}

#[async_trait]
impl<
        G: GroupInfoUpdater<C> + std::fmt::Debug + Sync + Send + 'static,
        C: PairingCurve + std::fmt::Debug + Sync + Send + 'static,
    > Subscriber for PostSuccessGroupingSubscriber<G, C>
{
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let DKGSuccess { group } = payload
            .as_any()
            .downcast_ref::<DKGSuccess<C>>()
            .unwrap()
            .clone();

        if self
            .group_cache
            .write()
            .await
            .update_dkg_status(group.index, group.epoch, DKGStatus::WaitForPostProcess)
            .await?
        {
            info!(
                "DKG status updated to WaitForPostProcess for group {} epoch {}",
                group.index, group.epoch
            );

            self.group_cache
                .write()
                .await
                .save_committers(group.index, group.epoch, group.committers)
                .await?;

            info!(
                "Group index:{} epoch:{} is available, committers saved.",
                group.index, group.epoch
            );
        }

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write().await.subscribe(Topic::DKGSuccess, subscriber);
    }
}

impl<
        G: GroupInfoUpdater<C> + std::fmt::Debug + Sync + Send + 'static,
        C: PairingCurve + std::fmt::Debug + Sync + Send + 'static,
    > DebuggableSubscriber for PostSuccessGroupingSubscriber<G, C>
{
}
