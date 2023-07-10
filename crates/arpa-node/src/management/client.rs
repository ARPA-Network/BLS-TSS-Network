use crate::error::{NodeError, NodeResult};
use crate::rpc_stub::management::management_service_client::ManagementServiceClient;
use crate::rpc_stub::management::ListFixedTasksRequest;
use tonic::codegen::InterceptedService;
use tonic::service::Interceptor;
use tonic::transport::Channel;
use tonic::Request;

#[derive(Clone, Debug)]
pub struct GeneralManagementClient {
    management_endpoint: String,
    authorization_token: String,
}

impl GeneralManagementClient {
    pub fn new(management_endpoint: String, authorization_token: String) -> Self {
        GeneralManagementClient {
            management_endpoint,
            authorization_token,
        }
    }

    async fn prepare_service_client(
        &self,
    ) -> NodeResult<ManagementServiceClient<InterceptedService<Channel, impl Interceptor + '_>>>
    {
        let channel = tonic::transport::Endpoint::new(format!(
            "{}{}",
            "http://",
            self.management_endpoint.clone()
        ))?
        .connect()
        .await
        .map_err(NodeError::RpcNotAvailableError)?;

        let client =
            ManagementServiceClient::with_interceptor(channel, move |mut req: Request<()>| {
                req.metadata_mut().insert(
                    "authorization",
                    self.authorization_token.clone().parse().unwrap(),
                );
                Ok(req)
            });

        Ok(client)
    }

    pub async fn list_fixed_tasks(&self) -> NodeResult<Vec<String>> {
        let mut management_client = self.prepare_service_client().await?;

        let request = Request::new(ListFixedTasksRequest {});
        management_client
            .list_fixed_tasks(request)
            .await
            .map(|r| r.into_inner().fixed_tasks)
            .map_err(|status| status.into())
    }
}
