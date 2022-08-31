pub mod client;
pub mod server;

use crate::node::dal::types::TaskType;
use crate::node::dal::GroupInfoFetcher;
use crate::node::error::NodeResult;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

#[async_trait]
pub(crate) trait CommitterService {
    async fn commit_partial_signature(
        self,
        chain_id: usize,
        task_type: TaskType,
        message: Vec<u8>,
        signature_index: usize,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool>;
}

pub(crate) trait CommitterClient {
    fn get_id_address(&self) -> &str;

    fn get_committer_endpoint(&self) -> &str;

    fn build(id_address: String, committer_endpoint: String) -> Self;
}

pub(crate) trait CommitterClientHandler<C: CommitterClient, G: GroupInfoFetcher> {
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
