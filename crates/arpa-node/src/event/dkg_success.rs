use crate::subscriber::DebuggableEvent;
use arpa_core::Group;
use threshold_bls::group::Curve;

use super::{types::Topic, Event};

#[derive(Clone, Debug)]
pub struct DKGSuccess<C: Curve> {
    pub group: Group<C>,
}

impl<C: Curve> DKGSuccess<C> {
    pub fn new(group: Group<C>) -> Self {
        DKGSuccess { group }
    }
}

impl<C: Curve + 'static> Event for DKGSuccess<C> {
    fn topic(&self) -> Topic {
        Topic::DKGSuccess
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl<C: Curve + Send + Sync + 'static> DebuggableEvent for DKGSuccess<C> {}
