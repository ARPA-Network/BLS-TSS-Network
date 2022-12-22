use self::management_stub::management_service_server::{
    ManagementService, ManagementServiceServer,
};
use self::management_stub::{
    ShutdownListenerReply, ShutdownListenerRequest, StartListenerReply, StartListenerRequest,
};
use crate::node::context::chain::{Chain, MainChainFetcher};
use crate::node::context::types::GeneralContext;
use crate::node::context::ContextFetcher;
use crate::node::scheduler::{FixedTaskScheduler, ListenerType, TaskType};
use arpa_node_contract_client::{
    adapter::AdapterClientBuilder, controller::ControllerClientBuilder,
    coordinator::CoordinatorClientBuilder, provider::ChainProviderBuilder,
};
use arpa_node_core::{ChainIdentity, RandomnessTask, SchedulerError};
use arpa_node_dal::{
    BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater, MdcContextUpdater,
    NodeInfoFetcher, NodeInfoUpdater,
};
use arpa_node_log::debug;
use std::convert::TryInto;
use std::sync::Arc;
use std::{
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::RwLock;
use tonic::transport::Body;
use tonic::{body::BoxBody, transport::Server, Request, Response, Status};
use tower::{Layer, Service};
use uuid::Uuid;
pub mod management_stub {
    include!("../../../rpc_stub/management.rs");
}

pub(crate) struct NodeManagementServiceServer<
    N: NodeInfoFetcher + NodeInfoUpdater + MdcContextUpdater,
    G: GroupInfoFetcher + GroupInfoUpdater + MdcContextUpdater,
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
        N: NodeInfoFetcher + NodeInfoUpdater + MdcContextUpdater,
        G: GroupInfoFetcher + GroupInfoUpdater + MdcContextUpdater,
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
        N: NodeInfoFetcher
            + NodeInfoUpdater
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher
            + GroupInfoUpdater
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
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
    N: NodeInfoFetcher
        + NodeInfoUpdater
        + MdcContextUpdater
        + std::fmt::Debug
        + Clone
        + Sync
        + Send
        + 'static,
    G: GroupInfoFetcher
        + GroupInfoUpdater
        + MdcContextUpdater
        + std::fmt::Debug
        + Clone
        + Sync
        + Send
        + 'static,
    T: BLSTasksFetcher<RandomnessTask>
        + BLSTasksUpdater<RandomnessTask>
        + std::fmt::Debug
        + Clone
        + Sync
        + Send
        + 'static,
    I: ChainIdentity
        + ControllerClientBuilder
        + CoordinatorClientBuilder
        + AdapterClientBuilder
        + ChainProviderBuilder
        + std::fmt::Debug
        + Clone
        + Sync
        + Send
        + 'static,
>(
    endpoint: String,
    context: Arc<RwLock<GeneralContext<N, G, T, I>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = endpoint.parse()?;

    // The stack of middleware that our service will be wrapped in
    let layer = tower::ServiceBuilder::new()
        // Apply middleware from tower
        .timeout(Duration::from_secs(30))
        // Apply our own middleware
        .layer(LogLayer::new(context.clone()))
        // Interceptors can be also be applied as middleware
        .into_inner();

    Server::builder()
        .layer(layer)
        .add_service(ManagementServiceServer::new(
            NodeManagementServiceServer::new(context),
        ))
        .serve(addr)
        .await?;
    Ok(())
}

#[derive(Debug, Clone)]
struct LogLayer<
    N: NodeInfoFetcher + NodeInfoUpdater + MdcContextUpdater + Clone,
    G: GroupInfoFetcher + GroupInfoUpdater + MdcContextUpdater + Clone,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Clone,
    I: ChainIdentity
        + ControllerClientBuilder
        + CoordinatorClientBuilder
        + AdapterClientBuilder
        + ChainProviderBuilder
        + Clone,
> {
    context: Arc<RwLock<GeneralContext<N, G, T, I>>>,
}

impl<
        N: NodeInfoFetcher + NodeInfoUpdater + MdcContextUpdater + Clone,
        G: GroupInfoFetcher + GroupInfoUpdater + MdcContextUpdater + Clone,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Clone,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + Clone,
    > LogLayer<N, G, T, I>
{
    pub fn new(context: Arc<RwLock<GeneralContext<N, G, T, I>>>) -> Self {
        LogLayer { context }
    }
}

impl<
        S,
        N: NodeInfoFetcher + NodeInfoUpdater + MdcContextUpdater + Clone,
        G: GroupInfoFetcher + GroupInfoUpdater + MdcContextUpdater + Clone,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Clone,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + Clone,
    > Layer<S> for LogLayer<N, G, T, I>
{
    type Service = LogService<S, N, G, T, I>;

    fn layer(&self, service: S) -> Self::Service {
        LogService {
            inner: service,
            context: self.context.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct LogService<
    S,
    N: NodeInfoFetcher + NodeInfoUpdater + MdcContextUpdater + Clone,
    G: GroupInfoFetcher + GroupInfoUpdater + MdcContextUpdater + Clone,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Clone,
    I: ChainIdentity
        + ControllerClientBuilder
        + CoordinatorClientBuilder
        + AdapterClientBuilder
        + ChainProviderBuilder
        + Clone,
> {
    inner: S,
    context: Arc<RwLock<GeneralContext<N, G, T, I>>>,
}

impl<
        S,
        N: NodeInfoFetcher
            + NodeInfoUpdater
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher
            + GroupInfoUpdater
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
    > Service<hyper::Request<Body>> for LogService<S, N, G, T, I>
where
    S: Service<hyper::Request<Body>, Response = hyper::Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: hyper::Request<Body>) -> Self::Future {
        // This is necessary because tonic internally uses `tower::buffer::Buffer`.
        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        // for details on why this is necessary
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        let context = self.context.clone();

        Box::pin(async move {
            log_mdc::insert("request_id", Uuid::new_v4().to_string());

            context
                .read()
                .await
                .get_main_chain()
                .get_node_cache()
                .read()
                .await
                .refresh_mdc_entry();

            context
                .read()
                .await
                .get_main_chain()
                .get_group_cache()
                .read()
                .await
                .refresh_mdc_entry();

            debug!("Intercepting management request: {:?}", req);

            let response = inner.call(req).await?;

            log_mdc::remove("request_id");

            Ok(response)
        })
    }
}
