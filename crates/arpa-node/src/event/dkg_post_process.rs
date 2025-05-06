use super::{types::Topic, Event};
use crate::subscriber::DebuggableEvent;
use arpa_core::Group;
use threshold_bls::group::Curve;

#[derive(Clone, Debug)]
pub struct DKGPostProcess<C: Curve> {
    pub group_index: usize,
    pub group_epoch: usize,
    pub group: Group<C>,
}

impl<C: Curve> DKGPostProcess<C> {
    pub fn new(group_index: usize, group_epoch: usize, group: Group<C>) -> Self {
        DKGPostProcess {
            group_index,
            group_epoch,
            group,
        }
    }
}

impl<C: Curve + Send + Sync + 'static> Event for DKGPostProcess<C> {
    fn topic(&self) -> Topic {
        Topic::DKGPostProcess
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl<C: Curve + Send + Sync + 'static> DebuggableEvent for DKGPostProcess<C> {}
