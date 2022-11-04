use super::Subscriber;
use crate::node::{
    error::NodeResult,
    event::{dkg_success::DKGSuccess, types::Topic, Event},
    queue::{event_queue::EventQueue, EventSubscriber},
};
use arpa_node_dal::GroupInfoUpdater;
use async_trait::async_trait;
use log::debug;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PostSuccessGroupingSubscriber<G: GroupInfoUpdater + Sync + Send> {
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoUpdater + Sync + Send> PostSuccessGroupingSubscriber<G> {
    pub fn new(group_cache: Arc<RwLock<G>>, eq: Arc<RwLock<EventQueue>>) -> Self {
        PostSuccessGroupingSubscriber { group_cache, eq }
    }
}

#[async_trait]
impl<G: GroupInfoUpdater + Sync + Send + 'static> Subscriber for PostSuccessGroupingSubscriber<G> {
    async fn notify(&self, topic: Topic, payload: &(dyn Event + Send + Sync)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let DKGSuccess { group } = payload
            .as_any()
            .downcast_ref::<DKGSuccess>()
            .unwrap()
            .clone();

        self.group_cache
            .write()
            .await
            .save_committers(group.index, group.epoch, group.committers)
            .await?;

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write().await.subscribe(Topic::DKGSuccess, subscriber);
    }
}
