use self::committer::committer_service_client::CommitterServiceClient;
use self::committer::CommitPartialSignatureRequest;
use crate::node::dao::types::TaskType;
use crate::node::error::errors::NodeResult;
use async_trait::async_trait;
use tonic::Request;

pub mod committer {
    include!("../../../stub/committer.rs");
}

#[async_trait]
pub trait CommitterService {
    async fn commit_partial_signature(
        &mut self,
        chain_id: usize,
        task_type: TaskType,
        message: Vec<u8>,
        signature_index: usize,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool>;
}

pub struct MockCommitterClient {
    id_address: String,
    committer_service_client: CommitterServiceClient<tonic::transport::Channel>,
}

impl MockCommitterClient {
    pub async fn new(
        id_address: String,
        committer_endpoint: String,
    ) -> NodeResult<MockCommitterClient> {
        let committer_service_client: CommitterServiceClient<tonic::transport::Channel> =
            CommitterServiceClient::connect(format!("{}{}", "http://", committer_endpoint.clone()))
                .await?;

        Ok(MockCommitterClient {
            id_address,
            committer_service_client,
        })
    }
}

#[async_trait]
impl CommitterService for MockCommitterClient {
    async fn commit_partial_signature(
        &mut self,
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

        self.committer_service_client
            .commit_partial_signature(request)
            .await
            .map(|r| r.into_inner().result)
            .map_err(|status| status.into())
    }
}
