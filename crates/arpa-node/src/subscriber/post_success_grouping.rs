use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    error::{NodeError, NodeResult},
    event::{dkg_success::DKGSuccess, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
};
use arpa_core::{
    log::{build_group_related_payload, LogType},
    DKGStatus,
};
use arpa_dal::GroupInfoHandler;
use async_trait::async_trait;
use log::{debug, error, info};
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

        let DKGSuccess {
            chain_id,
            id_address,
            group,
        } = payload
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

            if group.public_key.is_none()
                || *self.group_cache.read().await.get_public_key()? != group.public_key.unwrap()
            {
                error!(
                    "{}",
                    build_group_related_payload(
                        LogType::DKGGroupingTwisted,
                        "Group public key is different from the one saved in DKG process.",
                        chain_id,
                        self.group_cache.read().await.get_group()?
                    )
                );
                return Err(NodeError::DKGGroupingTwisted);
            }

            if !group.members.contains_key(&id_address) {
                error!(
                    "{}",
                    build_group_related_payload(
                        LogType::DKGGroupingTwisted,
                        "This node is not in the group, skip the process.",
                        chain_id,
                        self.group_cache.read().await.get_group()?
                    )
                );
                return Err(NodeError::DKGGroupingTwisted);
            }

            // sync up the members in the group
            if !self
                .group_cache
                .write()
                .await
                .sync_up_members(group.index, group.epoch, group.members)
                .await?
            {
                error!(
                    "{}",
                    build_group_related_payload(
                        LogType::DKGGroupingMemberMisMatch,
                        "Group members are not matched, attempt to run with contract records.",
                        chain_id,
                        self.group_cache.read().await.get_group()?
                    )
                );
            }

            // save the committers and update the state of the group
            self.group_cache
                .write()
                .await
                .save_committers(group.index, group.epoch, group.committers)
                .await?;

            info!(
                "{}",
                build_group_related_payload(
                    LogType::DKGGroupingAvailable,
                    "Group is available, committers saved.",
                    chain_id,
                    self.group_cache.read().await.get_group()?
                )
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
