use self::management_stub::management_service_server::{
    ManagementService, ManagementServiceServer,
};
use self::management_stub::{
    ShutdownListenerReply, ShutdownListenerRequest, StartListenerReply, StartListenerRequest,
};
use crate::node::context::chain::Chain;
use crate::node::context::types::GeneralContext;
use crate::node::context::ContextFetcher;
use crate::node::scheduler::{FixedTaskScheduler, ListenerType, TaskType};
use arpa_node_contract_client::{
    adapter::AdapterClientBuilder, controller::ControllerClientBuilder,
    coordinator::CoordinatorClientBuilder, provider::ChainProviderBuilder,
};
use arpa_node_core::{ChainIdentity, RandomnessTask, SchedulerError};
use arpa_node_dal::{
    BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater, NodeInfoFetcher,
};
use std::convert::TryInto;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;

pub mod management_stub {
    include!("../../../rpc_stub/management.rs");
}

pub(crate) struct NodeManagementServiceServer<
    N: NodeInfoFetcher,
    G: GroupInfoFetcher + GroupInfoUpdater,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
    I: ChainIdentity
        + ControllerClientBuilder
        + CoordinatorClientBuilder
        + AdapterClientBuilder
        + ChainProviderBuilder,
> {
    context: Arc<RwLock<GeneralContext<N, G, T, I>>>,
}

impl<
        N: NodeInfoFetcher,
        G: GroupInfoFetcher + GroupInfoUpdater,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder,
    > NodeManagementServiceServer<N, G, T, I>
{
    pub fn new(context: Arc<RwLock<GeneralContext<N, G, T, I>>>) -> Self {
        NodeManagementServiceServer { context }
    }
}

#[tonic::async_trait]
impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + Sync
            + Send
            + 'static,
    > ManagementService for NodeManagementServiceServer<N, G, T, I>
{
    async fn start_listener(
        &self,
        request: Request<StartListenerRequest>,
    ) -> Result<Response<StartListenerReply>, Status> {
        let req = request.into_inner();

        let task_type: ListenerType = (req.task_type() as i32)
            .try_into()
            .map_err(|e: SchedulerError| Status::invalid_argument(e.to_string()))?;

        self.context
            .write()
            .await
            .get_main_chain()
            .init_listener(
                self.context.read().await.get_event_queue(),
                self.context.read().await.get_fixed_task_handler(),
                TaskType::Listener(task_type),
            )
            .await
            .map_err(|e: SchedulerError| Status::already_exists(e.to_string()))?;

        return Ok(Response::new(StartListenerReply { result: true }));
    }

    async fn shutdown_listener(
        &self,
        request: Request<ShutdownListenerRequest>,
    ) -> Result<Response<ShutdownListenerReply>, Status> {
        let req = request.into_inner();

        let task_type: ListenerType = (req.task_type() as i32)
            .try_into()
            .map_err(|e: SchedulerError| Status::invalid_argument(e.to_string()))?;

        self.context
            .write()
            .await
            .get_fixed_task_handler()
            .write()
            .await
            .abort(TaskType::Listener(task_type))
            .await
            .map_err(|e: SchedulerError| Status::not_found(e.to_string()))?;

        return Ok(Response::new(ShutdownListenerReply { result: true }));
    }
}

pub async fn start_management_server<
    N: NodeInfoFetcher + Sync + Send + 'static,
    G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
    I: ChainIdentity
        + ControllerClientBuilder
        + CoordinatorClientBuilder
        + AdapterClientBuilder
        + ChainProviderBuilder
        + Sync
        + Send
        + 'static,
>(
    endpoint: String,
    context: Arc<RwLock<GeneralContext<N, G, T, I>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = endpoint.parse()?;

    Server::builder()
        .add_service(ManagementServiceServer::with_interceptor(
            NodeManagementServiceServer::new(context),
            intercept,
        ))
        .serve(addr)
        .await?;
    Ok(())
}

fn intercept(req: Request<()>) -> Result<Request<()>, Status> {
    println!("Intercepting request: {:?}", req);
    log_mdc::insert("request_id", Uuid::new_v4().to_string());

    Ok(req)
}
