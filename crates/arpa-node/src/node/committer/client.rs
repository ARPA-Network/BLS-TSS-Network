use super::{CommitterClient, CommitterService, ServiceClient};
use crate::node::error::{NodeError, NodeResult};
use crate::rpc_stub::committer::committer_service_client::CommitterServiceClient;
use crate::rpc_stub::committer::CommitPartialSignatureRequest;
use arpa_node_core::{address_to_string, jitter, BLSTaskType, CONFIG};
use async_trait::async_trait;
use ethers::types::Address;
use log::error;
use tokio_retry::{strategy::ExponentialBackoff, RetryIf};
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
        task_type: BLSTaskType,
        request_id: Vec<u8>,
        message: Vec<u8>,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool> {
        let commit_partial_signature_retry_descriptor = CONFIG
            .get()
            .unwrap()
            .time_limits
            .unwrap()
            .commit_partial_signature_retry_descriptor;
        let retry_strategy =
            ExponentialBackoff::from_millis(commit_partial_signature_retry_descriptor.base)
                .factor(commit_partial_signature_retry_descriptor.factor)
                .map(|e| {
                    if commit_partial_signature_retry_descriptor.use_jitter {
                        jitter(e)
                    } else {
                        e
                    }
                })
                .take(commit_partial_signature_retry_descriptor.max_attempts);

        RetryIf::spawn(
            retry_strategy,
            || async {
                let chain_id = chain_id;
                let request_id = request_id.clone();
                let message = message.clone();
                let partial_signature = partial_signature.clone();

                let request = Request::new(CommitPartialSignatureRequest {
                    id_address: address_to_string(self.id_address),
                    chain_id: chain_id as u32,
                    task_type: task_type.to_i32(),
                    request_id,
                    message,
                    partial_signature,
                });

                let mut committer_client = self.prepare_service_client().await?;

                committer_client
                    .commit_partial_signature(request)
                    .await
                    .map(|r| r.into_inner().result)
                    .map_err(|status| status.into())
            },
            |e: &NodeError| {
                error!(
                    "send partial signature to committer {0} failed. Retry... Error: {1:?}",
                    address_to_string(self.get_id_address()),
                    e
                );
                true
            },
        )
        .await
    }
}
