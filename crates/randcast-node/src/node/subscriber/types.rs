use crate::node::{
    error::errors::NodeResult,
    event::types::{Event, Topic},
};

pub trait Subscriber {
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()>;

    fn subscribe(self);
}
