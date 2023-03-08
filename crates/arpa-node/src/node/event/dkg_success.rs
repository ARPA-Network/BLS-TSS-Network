use crate::node::subscriber::DebuggableEvent;
use arpa_node_core::Group;
use threshold_bls::group::PairingCurve;

use super::{types::Topic, Event};

#[derive(Clone, Debug)]
pub struct DKGSuccess<C: PairingCurve> {
    pub group: Group<C>,
}

impl<C: PairingCurve> DKGSuccess<C> {
    pub fn new(group: Group<C>) -> Self {
        DKGSuccess { group }
    }
}

impl<C: PairingCurve + 'static> Event for DKGSuccess<C> {
    fn topic(&self) -> Topic {
        Topic::DKGSuccess
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl<C: PairingCurve + Send + Sync + 'static> DebuggableEvent for DKGSuccess<C> {}
