use crate::context::types::GeneralContext;
use crate::context::ContextFetcher;
use crate::error::NodeError;
use crate::management::ComponentService;
use crate::rpc_stub::management::management_service_server::{
    ManagementService, ManagementServiceServer,
};
use crate::rpc_stub::management::{
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
use arpa_core::{
    address_to_string, Group as ModelGroup, ListenerType, Member as ModelMember, SchedulerError,
};
use arpa_dal::error::DataAccessError;
use arpa_log::debug;
use hyper::http::HeaderValue;
use rustc_hex::FromHexError;
use std::convert::TryInto;
use std::sync::Arc;
use std::{
    task::{Context, Poll},
    time::Duration,
};
use threshold_bls::group::Curve;
use threshold_bls::sig::{SignatureScheme, ThresholdScheme};
use tokio::sync::RwLock;
use tonic::transport::Body;
use tonic::{body::BoxBody, transport::Server, Request, Response, Status};
use tower::{Layer, Service};
use uuid::Uuid;

use super::{BLSRandomnessService, DBService, DKGService, GroupInfo, NodeInfo, NodeService};

type NodeContext<PC, S> = Arc<RwLock<GeneralContext<PC, S>>>;

pub(crate) struct NodeManagementServiceServer<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
> {
    context: NodeContext<PC, S>,
}

impl<PC: Curve, S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>>
    NodeManagementServiceServer<PC, S>
{
    pub fn new(context: NodeContext<PC, S>) -> Self {
        NodeManagementServiceServer { context }
    }
}

#[tonic::async_trait]
impl<
        PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Send
            + Sync
            + 'static,
    > ManagementService for NodeManagementServiceServer<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
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
            .start_listener(req.chain_id as usize, task_type)
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
            .shutdown_listener(req.chain_id as usize, task_type)
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
        let request_id = req.request_id;
        let threshold = req.threshold as usize;
        let msg = req.msg;
        let partial_sig = self
            .context
            .write()
            .await
            .partial_sign(request_id, threshold, &msg)
            .await
            .map_err(|e: anyhow::Error| Status::failed_precondition(e.to_string()))?;
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
            .map_err(|e: anyhow::Error| Status::failed_precondition(e.to_string()))?;
        return Ok(Response::new(AggregatePartialSigsReply { sig }));
    }

    async fn verify_sig(
        &self,
        request: Request<VerifySigRequest>,
    ) -> Result<tonic::Response<VerifySigReply>, tonic::Status> {
        let req = request.into_inner();
        let public = bincode::deserialize(&req.public)
            .map_err(|e: bincode::Error| Status::invalid_argument(e.to_string()))?;
        let msg = req.msg;
        let sig = req.sig;
        self.context
            .read()
            .await
            .verify_sig(&public, &msg, &sig)
            .map_err(|e: anyhow::Error| Status::failed_precondition(e.to_string()))?;
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
            .map(|k| bincode::deserialize(k).unwrap())
            .collect::<Vec<_>>();
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
            .map_err(|e: anyhow::Error| Status::failed_precondition(e.to_string()))?;
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
        let request_id = req.request_id;
        let partial = req.partial_sig;

        self.context
            .write()
            .await
            .send_partial_sig(
                req.chain_id as usize,
                member_id_address,
                msg,
                request_id,
                partial,
            )
            .await
            .map_err(|e: anyhow::Error| Status::unavailable(e.to_string()))?;
        return Ok(Response::new(SendPartialSigReply { res: true }));
    }

    async fn fulfill_randomness(
        &self,
        request: Request<FulfillRandomnessRequest>,
    ) -> Result<tonic::Response<FulfillRandomnessReply>, tonic::Status> {
        let req = request.into_inner();
        let group_index = req.group_index as usize;
        let request_id = req.request_id;
        let sig = req.sig;
        let partial_sigs = req
            .partial_sigs
            .into_iter()
            .map(|(k, v)| (k.parse().unwrap(), v))
            .collect();
        self.context
            .write()
            .await
            .fulfill_randomness(
                req.chain_id as usize,
                group_index,
                request_id,
                sig,
                partial_sigs,
            )
            .await
            .map_err(|e: anyhow::Error| Status::failed_precondition(e.to_string()))?;
        return Ok(Response::new(FulfillRandomnessReply { res: true }));
    }
}

impl<PC: Curve> From<NodeInfo<PC>> for GetNodeInfoReply {
    fn from(n: NodeInfo<PC>) -> Self {
        GetNodeInfoReply {
            id_address: address_to_string(n.id_address),
            node_rpc_endpoint: n.node_rpc_endpoint,
            dkg_private_key: bincode::serialize(&n.dkg_private_key).unwrap(),
            dkg_public_key: bincode::serialize(&n.dkg_public_key).unwrap(),
        }
    }
}

impl<PC: Curve> From<GroupInfo<PC>> for GetGroupInfoReply {
    fn from(g: GroupInfo<PC>) -> Self {
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

impl<PC: Curve> From<ModelGroup<PC>> for Group {
    fn from(g: ModelGroup<PC>) -> Self {
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

impl<PC: Curve> From<ModelMember<PC>> for Member {
    fn from(member: ModelMember<PC>) -> Self {
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
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    SS: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + Clone
        + Send
        + Sync
        + 'static,
>(
    endpoint: String,
    context: NodeContext<PC, SS>,
) -> Result<(), Box<dyn std::error::Error>>
where
    <SS as ThresholdScheme>::Error: Sync + Send,
    <SS as SignatureScheme>::Error: Sync + Send,
{
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
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
> {
    context: NodeContext<PC, S>,
}

impl<PC: Curve, S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>>
    LogLayer<PC, S>
{
    pub fn new(context: NodeContext<PC, S>) -> Self {
        LogLayer { context }
    }
}

impl<
        S,
        PC: Curve,
        SS: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
    > Layer<S> for LogLayer<PC, SS>
{
    type Service = LogService<S, PC, SS>;

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
    PC: Curve,
    SS: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
> {
    inner: S,
    context: NodeContext<PC, SS>,
}

impl<
        S,
        PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
        SS: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Send
            + Sync
            + 'static,
    > Service<hyper::Request<Body>> for LogService<S, PC, SS>
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
            log_mdc::insert("management_request_id", Uuid::new_v4().to_string());

            debug!("Intercepting management request: {:?}", req);

            let token = HeaderValue::from_str(
                context
                    .read()
                    .await
                    .get_config()
                    .get_node_management_rpc_token(),
            )
            .unwrap();

            match req.headers().get("authorization") {
                Some(t) if token == t => {}
                _ => return Ok(Status::unauthenticated("No valid auth token").to_http()),
            };

            let response = inner.call(req).await?;

            log_mdc::remove("management_request_id");

            Ok(response)
        })
    }
}
