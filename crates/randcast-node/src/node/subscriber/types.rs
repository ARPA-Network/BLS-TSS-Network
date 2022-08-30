use std::sync::Arc;

use parking_lot::RwLock;

use crate::node::{
    committer::committer_client::CommitterClient,
    dal::api::GroupInfoFetcher,
    error::errors::NodeResult,
    event::types::{Event, Topic},
};

pub trait Subscriber {
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()>;

    fn subscribe(self);
}

pub trait CommitterClientHandler<C: CommitterClient, G: GroupInfoFetcher> {
    fn get_id_address(&self) -> &str;

    fn get_group_cache(&self) -> Arc<RwLock<G>>;

    fn prepare_committer_clients(&self) -> NodeResult<Vec<C>> {
        let mut committers = self
            .get_group_cache()
            .read()
            .get_committers()?
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>();

        committers.retain(|c| *c != self.get_id_address());

        let mut committer_clients = vec![];

        for committer in committers {
            let endpoint = self
                .get_group_cache()
                .read()
                .get_member(&committer)?
                .rpc_endpint
                .as_ref()
                .unwrap()
                .to_string();

            let committer_client = C::build(self.get_id_address().to_string(), endpoint.clone());

            committer_clients.push(committer_client);
        }

        Ok(committer_clients)
    }
}
