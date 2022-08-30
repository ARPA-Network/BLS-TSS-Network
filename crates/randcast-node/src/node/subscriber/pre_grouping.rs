use super::types::Subscriber;
use crate::node::{
    dal::{
        api::{GroupInfoFetcher, GroupInfoUpdater},
        types::DKGStatus,
    },
    error::errors::NodeResult,
    event::{
        new_dkg_task::NewDKGTask,
        run_dkg::RunDKG,
        types::{Event, Topic},
    },
    queue::event_queue::{EventPublisher, EventQueue, EventSubscriber},
};
use log::info;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct PreGroupingSubscriber<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> {
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> PreGroupingSubscriber<G> {
    pub fn new(group_cache: Arc<RwLock<G>>, eq: Arc<RwLock<EventQueue>>) -> Self {
        PreGroupingSubscriber { group_cache, eq }
    }
}

impl<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send> EventPublisher<RunDKG>
    for PreGroupingSubscriber<G>
{
    fn publish(&self, event: RunDKG) {
        self.eq.read().publish(event);
    }
}

impl<G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static> Subscriber
    for PreGroupingSubscriber<G>
{
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        info!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut NewDKGTask;

            let NewDKGTask {
                dkg_task,
                self_index,
            } = *Box::from_raw(struct_ptr);

            let cache_index = self.group_cache.read().get_index().unwrap_or(0);

            let cache_epoch = self.group_cache.read().get_epoch().unwrap_or(0);

            let task_group_index = dkg_task.group_index;

            let task_epoch = dkg_task.epoch;

            if cache_index != task_group_index || cache_epoch != task_epoch {
                self.group_cache
                    .write()
                    .save_task_info(self_index, dkg_task.clone())?;

                let res = self.group_cache.write().update_dkg_status(
                    task_group_index,
                    task_epoch,
                    DKGStatus::InPhase,
                )?;

                if res {
                    self.publish(RunDKG { dkg_task });

                    info!(
                        "received new dkg_task: index:{} epoch:{}, start handling...",
                        task_group_index, task_epoch
                    );
                }
            }
        }

        Ok(())
    }

    fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write().subscribe(Topic::NewDKGTask, subscriber);
    }
}
