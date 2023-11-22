use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    error::NodeResult,
    event::{dkg_success::DKGSuccess, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
};
use arpa_core::DKGStatus;
use arpa_dal::GroupInfoHandler;
use async_trait::async_trait;
use log::{debug, info};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PostSuccessGroupingSubscriber<PC: Curve> {
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    c: PhantomData<PC>,
}

impl<PC: Curve> PostSuccessGroupingSubscriber<PC> {
    pub fn new(
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        PostSuccessGroupingSubscriber {
            group_cache,
            eq,
            c: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> Subscriber
    for PostSuccessGroupingSubscriber<PC>
{
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let DKGSuccess { group } = payload
            .as_any()
            .downcast_ref::<DKGSuccess<PC>>()
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

impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> DebuggableSubscriber
    for PostSuccessGroupingSubscriber<PC>
{
}
