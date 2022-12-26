use self::management_stub::management_service_server::{
    ManagementService, ManagementServiceServer,
};
use self::management_stub::{
    AggregatePartialSigsReply, AggregatePartialSigsRequest, FulfillRandomnessReply,
    FulfillRandomnessRequest, GetGroupInfoReply, GetGroupInfoRequest, GetNodeInfoReply,
    GetNodeInfoRequest, Group, ListFixedTasksReply, ListFixedTasksRequest, Member,
    NodeActivateReply, NodeActivateRequest, NodeQuitReply, NodeQuitRequest, NodeRegisterReply,
    NodeRegisterRequest, PartialSignReply, PartialSignRequest, PostProcessDkgReply,
    PostProcessDkgRequest, SendPartialSigReply, SendPartialSigRequest, ShutdownListenerReply,
    ShutdownListenerRequest, ShutdownNodeReply, ShutdownNodeRequest, StartListenerReply,
    StartListenerRequest, VerifyPartialSigsReply, VerifyPartialSigsRequest, VerifySigReply,
    VerifySigRequest,
};
use crate::node::context::chain::MainChainFetcher;
use crate::node::context::types::GeneralContext;
use crate::node::context::ContextFetcher;
use crate::node::error::NodeError;
use crate::node::management::ComponentService;
use crate::node::scheduler::ListenerType;
use arpa_node_contract_client::{
    adapter::AdapterClientBuilder, controller::ControllerClientBuilder,
    coordinator::CoordinatorClientBuilder, provider::ChainProviderBuilder,
};
use arpa_node_core::{
    address_to_string, ChainIdentity, Group as ModelGroup, Member as ModelMember, RandomnessTask,
    SchedulerError,
};
use arpa_node_dal::error::DataAccessError;
use arpa_node_dal::{
    BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater, MdcContextUpdater,
    NodeInfoFetcher, NodeInfoUpdater,
};
use arpa_node_log::debug;
use hyper::http::HeaderValue;
use rustc_hex::FromHexError;
use std::convert::TryInto;
use std::sync::Arc;
use std::{
    task::{Context, Poll},
    time::Duration,
};
use threshold_bls::curve::bls12381::G1;
use tokio::sync::RwLock;
use tonic::transport::Body;
use tonic::{body::BoxBody, transport::Server, Request, Response, Status};
use tower::{Layer, Service};
use uuid::Uuid;

use super::{BLSRandomnessService, DBService, DKGService, GroupInfo, NodeInfo, NodeService};

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
    async fn list_fixed_tasks(
        &self,
        request: Request<ListFixedTasksRequest>,
    ) -> Result<Response<ListFixedTasksReply>, Status> {
        let _req = request.into_inner();

        let fixed_tasks = self
            .context
            .read()
            .await
            .list_fixed_tasks()
            .await
            .map(|ts| ts.iter().map(|t| t.to_string()).collect())
            .map_err(|e: SchedulerError| Status::internal(e.to_string()))?;

        return Ok(Response::new(ListFixedTasksReply { fixed_tasks }));
    }

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
            .start_listener(task_type)
            .await
            .map_err(|e: SchedulerError| Status::already_exists(e.to_string()))?;

        return Ok(Response::new(StartListenerReply { res: true }));
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
            .shutdown_listener(task_type)
            .await
            .map_err(|e: SchedulerError| Status::not_found(e.to_string()))?;

        return Ok(Response::new(ShutdownListenerReply { res: true }));
    }

    async fn node_register(
        &self,
        request: Request<NodeRegisterRequest>,
    ) -> Result<tonic::Response<NodeRegisterReply>, tonic::Status> {
        let _req = request.into_inner();
        self.context
            .read()
            .await
            .node_register()
            .await
            .map_err(|e: NodeError| Status::failed_precondition(e.to_string()))?;
        return Ok(Response::new(NodeRegisterReply { res: true }));
    }

    async fn node_activate(
        &self,
        request: Request<NodeActivateRequest>,
    ) -> Result<tonic::Response<NodeActivateReply>, tonic::Status> {
        let _req = request.into_inner();
        //TODO
        // return Ok(Response::new(NodeActivateReply { res: true }));
        return Err(Status::unimplemented("unimplemented"));
    }

    async fn node_quit(
        &self,
        request: Request<NodeQuitRequest>,
    ) -> Result<tonic::Response<NodeQuitReply>, tonic::Status> {
        let _req = request.into_inner();
        //TODO
        // return Ok(Response::new(NodeQuitReply { res: true }));
        return Err(Status::unimplemented("unimplemented"));
    }

    async fn shutdown_node(
        &self,
        request: Request<ShutdownNodeRequest>,
    ) -> Result<tonic::Response<ShutdownNodeReply>, tonic::Status> {
        let _req = request.into_inner();
        self.context
            .read()
            .await
            .shutdown_node()
            .await
            .map_err(|e: NodeError| Status::internal(e.to_string()))?;
        return Ok(Response::new(ShutdownNodeReply { res: true }));
    }

    async fn get_node_info(
        &self,
        request: Request<GetNodeInfoRequest>,
    ) -> Result<tonic::Response<GetNodeInfoReply>, tonic::Status> {
        let _req = request.into_inner();
        let node_info = self
            .context
            .read()
            .await
            .get_node_info()
            .await
            .map_err(|e: DataAccessError| Status::unavailable(e.to_string()))?;
        return Ok(Response::new(node_info.into()));
    }

    async fn get_group_info(
        &self,
        request: Request<GetGroupInfoRequest>,
    ) -> Result<tonic::Response<GetGroupInfoReply>, tonic::Status> {
        let _req = request.into_inner();
        let group_info = self
            .context
            .read()
            .await
            .get_group_info()
            .await
            .map_err(|e: DataAccessError| Status::unavailable(e.to_string()))?;
        return Ok(Response::new(group_info.into()));
    }

    async fn post_process_dkg(
        &self,
        request: Request<PostProcessDkgRequest>,
    ) -> Result<tonic::Response<PostProcessDkgReply>, tonic::Status> {
        let _req = request.into_inner();
        self.context
            .write()
            .await
            .post_process_dkg()
            .await
            .map_err(|e: NodeError| Status::failed_precondition(e.to_string()))?;
        return Ok(Response::new(PostProcessDkgReply { res: true }));
    }

    async fn partial_sign(
        &self,
        request: Request<PartialSignRequest>,
    ) -> Result<tonic::Response<PartialSignReply>, tonic::Status> {
        let req = request.into_inner();
        let sig_index = req.sig_index as usize;
        let threshold = req.threshold as usize;
        let msg = req.msg;
        let partial_sig = self
            .context
            .write()
            .await
            .partial_sign(sig_index, threshold, &msg)
            .await
            .map_err(|e: NodeError| Status::failed_precondition(e.to_string()))?;
        return Ok(Response::new(PartialSignReply { partial_sig }));
    }

    async fn aggregate_partial_sigs(
        &self,
        request: Request<AggregatePartialSigsRequest>,
    ) -> Result<tonic::Response<AggregatePartialSigsReply>, tonic::Status> {
        let req = request.into_inner();
        let threshold = req.threshold as usize;
        let partial_sigs = req.partial_sigs;
        let sig = self
            .context
            .write()
            .await
            .aggregate_partial_sigs(threshold, &partial_sigs)
            .map_err(|e: NodeError| Status::failed_precondition(e.to_string()))?;
        return Ok(Response::new(AggregatePartialSigsReply { sig }));
    }

    async fn verify_sig(
        &self,
        request: Request<VerifySigRequest>,
    ) -> Result<tonic::Response<VerifySigReply>, tonic::Status> {
        let req = request.into_inner();
        let public: G1 = bincode::deserialize(&req.public)
            .map_err(|e: bincode::Error| Status::invalid_argument(e.to_string()))?;
        let msg = req.msg;
        let sig = req.sig;
        self.context
            .read()
            .await
            .verify_sig(&public, &msg, &sig)
            .map_err(|e: NodeError| Status::failed_precondition(e.to_string()))?;
        return Ok(Response::new(VerifySigReply { res: true }));
    }

    async fn verify_partial_sigs(
        &self,
        request: Request<VerifyPartialSigsRequest>,
    ) -> Result<tonic::Response<VerifyPartialSigsReply>, tonic::Status> {
        let req = request.into_inner();
        let publics = req
            .publics
            .iter()
            .map(|k| {
                let public: G1 = bincode::deserialize(k).unwrap();
                public
            })
            .collect::<Vec<G1>>();
        let msg = req.msg;
        let partial_sigs = req
            .partial_sigs
            .iter()
            .map(|sig| sig as &[u8])
            .collect::<Vec<&[u8]>>();
        self.context
            .read()
            .await
            .verify_partial_sigs(&publics, &msg, &partial_sigs)
            .map_err(|e: NodeError| Status::failed_precondition(e.to_string()))?;
        return Ok(Response::new(VerifyPartialSigsReply { res: true }));
    }

    async fn send_partial_sig(
        &self,
        request: Request<SendPartialSigRequest>,
    ) -> Result<tonic::Response<SendPartialSigReply>, tonic::Status> {
        let req = request.into_inner();
        let member_id_address = req
            .member_id_address
            .parse()
            .map_err(|e: FromHexError| Status::invalid_argument(e.to_string()))?;
        let msg = req.msg;
        let sig_index = req.sig_index as usize;
        let partial = req.partial_sig;

        self.context
            .write()
            .await
            .send_partial_sig(member_id_address, msg, sig_index, partial)
            .await
            .map_err(|e: NodeError| Status::unavailable(e.to_string()))?;
        return Ok(Response::new(SendPartialSigReply { res: true }));
    }

    async fn fulfill_randomness(
        &self,
        request: Request<FulfillRandomnessRequest>,
    ) -> Result<tonic::Response<FulfillRandomnessReply>, tonic::Status> {
        let req = request.into_inner();
        let group_index = req.group_index as usize;
        let sig_index = req.sig_index as usize;
        let sig = req.sig;
        let partial_sigs = req
            .partial_sigs
            .into_iter()
            .map(|(k, v)| (k.parse().unwrap(), v))
            .collect();
        self.context
            .write()
            .await
            .fulfill_randomness(group_index, sig_index, sig, partial_sigs)
            .await
            .map_err(|e: NodeError| Status::failed_precondition(e.to_string()))?;
        return Ok(Response::new(FulfillRandomnessReply { res: true }));
    }
}

impl From<NodeInfo> for GetNodeInfoReply {
    fn from(n: NodeInfo) -> Self {
        GetNodeInfoReply {
            id_address: address_to_string(n.id_address),
            node_rpc_endpoint: n.node_rpc_endpoint,
            dkg_private_key: bincode::serialize(&n.dkg_private_key).unwrap(),
            dkg_public_key: bincode::serialize(&n.dkg_public_key).unwrap(),
        }
    }
}

impl From<GroupInfo> for GetGroupInfoReply {
    fn from(g: GroupInfo) -> Self {
        let share = if let Some(s) = g.share {
            bincode::serialize(&s).unwrap()
        } else {
            vec![]
        };
        GetGroupInfoReply {
            share,
            group: Some(g.group.into()),
            dkg_status: g.dkg_status.to_usize() as i32,
            self_index: g.self_index as u32,
            dkg_start_block_height: g.dkg_start_block_height as u32,
        }
    }
}

impl From<ModelGroup> for Group {
    fn from(g: ModelGroup) -> Self {
        let public_key = if let Some(k) = g.public_key {
            bincode::serialize(&k).unwrap()
        } else {
            vec![]
        };

        let committers = g.committers.into_iter().map(address_to_string).collect();

        let members = g
            .members
            .into_iter()
            .map(|(id_address, m)| (address_to_string(id_address), m.into()))
            .collect();

        Group {
            index: g.index as u32,
            epoch: g.epoch as u32,
            size: g.size as u32,
            threshold: g.threshold as u32,
            state: g.state,
            public_key,
            members,
            committers,
        }
    }
}

impl From<ModelMember> for Member {
    fn from(member: ModelMember) -> Self {
        let partial_public_key = if let Some(k) = member.partial_public_key {
            bincode::serialize(&k).unwrap()
        } else {
            vec![]
        };

        Member {
            index: member.index as u32,
            id_address: address_to_string(member.id_address),
            rpc_endpoint: member.rpc_endpoint.unwrap_or_default(),
            partial_public_key,
        }
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

            let token_str = match context
                .read()
                .await
                .get_config()
                .get_node_management_rpc_token()
            {
                Ok(t) => t,
                Err(_) => {
                    return Ok(
                        Status::unauthenticated("Invalid management server token setup").to_http(),
                    )
                }
            };

            let token = HeaderValue::from_str(&token_str).unwrap();

            match req.headers().get("authorization") {
                Some(t) if token == t => {}
                _ => return Ok(Status::unauthenticated("No valid auth token").to_http()),
            };

            let response = inner.call(req).await?;

            log_mdc::remove("request_id");

            Ok(response)
        })
    }
}
