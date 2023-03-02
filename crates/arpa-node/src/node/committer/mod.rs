pub mod client;
pub mod server;

use crate::node::error::NodeResult;
use arpa_node_core::TaskType;
use arpa_node_dal::GroupInfoFetcher;
use async_trait::async_trait;
use ethers::types::Address;
use std::sync::Arc;
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;

#[async_trait]
pub trait ServiceClient<C> {
    async fn prepare_service_client(&self) -> NodeResult<C>;
}

#[async_trait]
pub(crate) trait CommitterService {
    async fn commit_partial_signature(
        self,
        chain_id: usize,
        task_type: TaskType,
        message: Vec<u8>,
        request_id: Vec<u8>,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool>;
}

pub(crate) trait CommitterClient {
    fn get_id_address(&self) -> Address;

    fn get_committer_endpoint(&self) -> &str;

    fn build(id_address: Address, committer_endpoint: String) -> Self;
}

#[async_trait]
pub(crate) trait CommitterClientHandler<
    C: CommitterClient + Sync + Send,
    G: GroupInfoFetcher<PC> + Sync + Send,
    PC: PairingCurve,
>
{
    async fn get_id_address(&self) -> Address;

    fn get_group_cache(&self) -> Arc<RwLock<G>>;

    async fn prepare_committer_clients(&self) -> NodeResult<Vec<C>> {
        let mut committers = self.get_group_cache().read().await.get_committers()?;

        let id_address = self.get_id_address().await;

        committers.retain(|c| *c != id_address);

        let mut committer_clients = vec![];

        for committer in committers {
            let endpoint = self
                .get_group_cache()
                .read()
                .await
                .get_member(committer)?
                .rpc_endpoint
                .as_ref()
                .unwrap()
                .to_string();

            let committer_client = C::build(id_address, endpoint.clone());

            committer_clients.push(committer_client);
        }

        Ok(committer_clients)
    }
}
