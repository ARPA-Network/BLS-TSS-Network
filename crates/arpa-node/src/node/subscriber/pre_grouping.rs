use super::Subscriber;
use crate::node::{
    error::NodeResult,
    event::{new_dkg_task::NewDKGTask, run_dkg::RunDKG, types::Topic, Event},
    queue::{event_queue::EventQueue, EventPublisher, EventSubscriber},
};
use arpa_node_core::DKGStatus;
use arpa_node_dal::{GroupInfoFetcher, GroupInfoUpdater};
use async_trait::async_trait;
use log::{debug, info};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PreGroupingSubscriber<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> {
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> PreGroupingSubscriber<G> {
    pub fn new(group_cache: Arc<RwLock<G>>, eq: Arc<RwLock<EventQueue>>) -> Self {
        PreGroupingSubscriber { group_cache, eq }
    }
}

#[async_trait]
impl<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> EventPublisher<RunDKG>
    for PreGroupingSubscriber<G>
{
    async fn publish(&self, event: RunDKG) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static> Subscriber
    for PreGroupingSubscriber<G>
{
    async fn notify(&self, topic: Topic, payload: &(dyn Event + Send + Sync)) -> NodeResult<()> {
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
