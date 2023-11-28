use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    error::NodeResult,
    event::{new_dkg_task::NewDKGTask, run_dkg::RunDKG, types::Topic},
    queue::{event_queue::EventQueue, EventPublisher, EventSubscriber},
};
use arpa_core::DKGStatus;
use arpa_dal::GroupInfoHandler;
use async_trait::async_trait;
use log::{debug, info};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PreGroupingSubscriber<PC: Curve> {
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    c: PhantomData<PC>,
}

impl<PC: Curve> PreGroupingSubscriber<PC> {
    pub fn new(
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        PreGroupingSubscriber {
            group_cache,
            eq,
            c: PhantomData,
        }
    }
}

#[async_trait]
impl<C: Curve + std::fmt::Debug + Sync + Send> EventPublisher<RunDKG> for PreGroupingSubscriber<C> {
    async fn publish(&self, event: RunDKG) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<C: Curve + std::fmt::Debug + Sync + Send + 'static> Subscriber for PreGroupingSubscriber<C> {
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let NewDKGTask {
            dkg_task,
            self_index,
        } = payload
            .as_any()
            .downcast_ref::<NewDKGTask>()
            .unwrap()
            .clone();

        let cache_index = self.group_cache.read().await.get_index().unwrap_or(0);

        let cache_epoch = self.group_cache.read().await.get_epoch().unwrap_or(0);

        let task_group_index = dkg_task.group_index;

        let task_epoch = dkg_task.epoch;

        if cache_index != task_group_index || cache_epoch != task_epoch {
            self.group_cache
                .write()
                .await
                .save_task_info(self_index, dkg_task.clone())
                .await?;

            let res = self
                .group_cache
                .write()
                .await
                .update_dkg_status(task_group_index, task_epoch, DKGStatus::InPhase)
                .await?;

            if res {
                self.publish(RunDKG { dkg_task }).await;

                info!(
                    "received new dkg_task: index:{} epoch:{}, start handling...",
                    task_group_index, task_epoch
                );
            }
        }

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write().await.subscribe(Topic::NewDKGTask, subscriber);
    }
}

impl<C: Curve + std::fmt::Debug + Sync + Send + 'static> DebuggableSubscriber
    for PreGroupingSubscriber<C>
{
}
