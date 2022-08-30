use self::committer::committer_service_client::CommitterServiceClient;
use self::committer::CommitPartialSignatureRequest;
use crate::node::dal::types::TaskType;
use crate::node::error::errors::NodeResult;
use crate::node::ServiceClient;
use async_trait::async_trait;
use tonic::Request;

pub mod committer {
    include!("../../../stub/committer.rs");
}

#[async_trait]
pub trait CommitterService {
    async fn commit_partial_signature(
        self,
        chain_id: usize,
        task_type: TaskType,
        message: Vec<u8>,
        signature_index: usize,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool>;
}

pub trait CommitterClient {
    fn get_id_address(&self) -> &str;

    fn get_committer_endpoint(&self) -> &str;

    fn build(id_address: String, committer_endpoint: String) -> Self;
}

#[derive(Clone, Debug)]
pub struct MockCommitterClient {
    id_address: String,
    committer_endpoint: String,
}

impl MockCommitterClient {
    pub fn new(id_address: String, committer_endpoint: String) -> Self {
        MockCommitterClient {
            id_address,
            committer_endpoint,
        }
    }
}

impl CommitterClient for MockCommitterClient {
    fn get_id_address(&self) -> &str {
        &self.id_address
    }

    fn get_committer_endpoint(&self) -> &str {
        &self.committer_endpoint
    }

    fn build(id_address: String, committer_endpoint: String) -> Self {
        Self::new(id_address, committer_endpoint)
    }
}

#[async_trait]
impl ServiceClient<CommitterServiceClient<tonic::transport::Channel>> for MockCommitterClient {
    async fn prepare_service_client(
        &self,
    ) -> NodeResult<CommitterServiceClient<tonic::transport::Channel>> {
        CommitterServiceClient::connect(format!("{}{}", "http://", self.committer_endpoint.clone()))
            .await
            .map_err(|err| err.into())
    }
}

#[async_trait]
impl CommitterService for MockCommitterClient {
    async fn commit_partial_signature(
        self,
        chain_id: usize,
        task_type: TaskType,
        message: Vec<u8>,
        signature_index: usize,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool> {
        let request = Request::new(CommitPartialSignatureRequest {
            id_address: self.id_address.to_string(),
            chain_id: chain_id as u32,
            signature_index: signature_index as u32,
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
