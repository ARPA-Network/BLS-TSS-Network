use super::{CommitterClient, CommitterService, ServiceClient};
use crate::node::error::NodeResult;
use crate::rpc_stub::committer::committer_service_client::CommitterServiceClient;
use crate::rpc_stub::committer::CommitPartialSignatureRequest;
use arpa_node_core::{address_to_string, TaskType};
use async_trait::async_trait;
use ethers::types::Address;
use tonic::Request;

#[derive(Clone, Debug)]
pub(crate) struct GeneralCommitterClient {
    id_address: Address,
    committer_endpoint: String,
}

impl GeneralCommitterClient {
    pub fn new(id_address: Address, committer_endpoint: String) -> Self {
        GeneralCommitterClient {
            id_address,
            committer_endpoint,
        }
    }
}

impl CommitterClient for GeneralCommitterClient {
    fn get_id_address(&self) -> Address {
        self.id_address
    }

    fn get_committer_endpoint(&self) -> &str {
        &self.committer_endpoint
    }

    fn build(id_address: Address, committer_endpoint: String) -> Self {
        Self::new(id_address, committer_endpoint)
    }
}

#[async_trait]
impl ServiceClient<CommitterServiceClient<tonic::transport::Channel>> for GeneralCommitterClient {
    async fn prepare_service_client(
        &self,
    ) -> NodeResult<CommitterServiceClient<tonic::transport::Channel>> {
        CommitterServiceClient::connect(format!("{}{}", "http://", self.committer_endpoint.clone()))
            .await
            .map_err(|err| err.into())
    }
}

#[async_trait]
impl CommitterService for GeneralCommitterClient {
    async fn commit_partial_signature(
        self,
        chain_id: usize,
        task_type: TaskType,
        message: Vec<u8>,
        request_id: Vec<u8>,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool> {
        let request = Request::new(CommitPartialSignatureRequest {
            id_address: address_to_string(self.id_address),
            chain_id: chain_id as u32,
            request_id,
            partial_signature,
            task_type: task_type.to_i32(),
            message,
        });

        let mut committer_client = self.prepare_service_client().await?;

        committer_client
            .commit_partial_signature(request)
            .await
            .map(|r| r.into_inner().result)
            .map_err(|status| status.into())
    }
}
