use super::types::Subscriber;
use crate::node::{
    dao::{api::GroupInfoUpdater, cache::InMemoryGroupInfoCache},
    error::errors::NodeResult,
    event::{
        dkg_success::DKGSuccess,
        types::{Event, Topic},
    },
    queue::event_queue::{EventQueue, EventSubscriber},
};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct PostSuccessGroupingSubscriber {
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl PostSuccessGroupingSubscriber {
    pub fn new(
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        PostSuccessGroupingSubscriber { group_cache, eq }
    }
}

impl Subscriber for PostSuccessGroupingSubscriber {
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        println!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut DKGSuccess;

            let DKGSuccess { group } = *Box::from_raw(struct_ptr);

            self.group_cache
                .write()
                .save_committers(group.index, group.epoch, group.committers)?;
        }

        Ok(())
    }

    fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write().subscribe(Topic::DKGSuccess, subscriber);
    }
}
