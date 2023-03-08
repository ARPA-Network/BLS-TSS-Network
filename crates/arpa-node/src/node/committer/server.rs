use crate::node::context::chain::MainChainFetcher;
use crate::node::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    context::{chain::ChainFetcher, types::GeneralContext, ContextFetcher},
    error::NodeError,
};
use crate::rpc_stub::committer::{
    committer_service_server::{CommitterService, CommitterServiceServer},
    CommitPartialSignatureReply, CommitPartialSignatureRequest,
};
use arpa_node_contract_client::{
    adapter::AdapterClientBuilder, controller::ControllerClientBuilder,
    coordinator::CoordinatorClientBuilder, provider::ChainProviderBuilder,
};
use arpa_node_core::{BLSTaskError, ChainIdentity, RandomnessTask, TaskType};
use arpa_node_dal::{
    BLSTasksFetcher, BLSTasksUpdater, ContextInfoUpdater, GroupInfoFetcher, GroupInfoUpdater,
    NodeInfoFetcher, NodeInfoUpdater, SignatureResultCacheFetcher, SignatureResultCacheUpdater,
};
use ethers::types::Address;
use futures::Future;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;
use tonic::{transport::Server, Request, Response, Status};

type NodeContext<N, G, T, I, PC> = Arc<RwLock<GeneralContext<N, G, T, I, PC>>>;

pub(crate) struct BLSCommitterServiceServer<
    N: NodeInfoFetcher<PC> + NodeInfoUpdater<PC> + ContextInfoUpdater,
    G: GroupInfoFetcher<PC> + GroupInfoUpdater<PC> + ContextInfoUpdater,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
    I: ChainIdentity
        + ControllerClientBuilder
        + CoordinatorClientBuilder
        + AdapterClientBuilder<PC>
        + ChainProviderBuilder,
    PC: PairingCurve,
> {
    id_address: Address,
    group_cache: Arc<RwLock<G>>,
    context: NodeContext<N, G, T, I, PC>,
    c: PhantomData<PC>,
}

impl<
        N: NodeInfoFetcher<PC> + NodeInfoUpdater<PC> + ContextInfoUpdater,
        G: GroupInfoFetcher<PC> + GroupInfoUpdater<PC> + ContextInfoUpdater,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<PC>
            + ChainProviderBuilder,
        PC: PairingCurve,
    > BLSCommitterServiceServer<N, G, T, I, PC>
{
    pub fn new(
        id_address: Address,
        group_cache: Arc<RwLock<G>>,
        context: NodeContext<N, G, T, I, PC>,
    ) -> Self {
        BLSCommitterServiceServer {
            id_address,
            group_cache,
            context,
            c: PhantomData,
        }
    }
}

#[tonic::async_trait]
impl<
        N: NodeInfoFetcher<PC>
            + NodeInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<PC>
            + GroupInfoUpdater<PC>
            + ContextInfoUpdater
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
            + AdapterClientBuilder<PC>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > CommitterService for BLSCommitterServiceServer<N, G, T, I, PC>
{
    async fn commit_partial_signature(
        &self,
        request: Request<CommitPartialSignatureRequest>,
    ) -> Result<Response<CommitPartialSignatureReply>, Status> {
        let req = request.into_inner();

        if let Err(_) | Ok(false) = self.group_cache.read().await.get_state() {
            return Err(Status::not_found(NodeError::GroupNotReady.to_string()));
        }

        if let Err(_) | Ok(false) = self.group_cache.read().await.is_committer(self.id_address) {
            return Err(Status::not_found(NodeError::NotCommitter.to_string()));
        }

        let req_id_address: Address = req
            .id_address
            .parse()
            .map_err(|_| Status::invalid_argument(NodeError::AddressFormatError.to_string()))?;

        if let Ok(member) = self.group_cache.read().await.get_member(req_id_address) {
            let partial_public_key = member.partial_public_key.clone().unwrap();

            SimpleBLSCore::<PC>::partial_verify(
                &partial_public_key,
                &req.message,
                &req.partial_signature,
            )
            .map_err(|e| Status::internal(e.to_string()))?;

            let chain_id = req.chain_id as usize;

            match TaskType::from(req.task_type) {
                TaskType::Randomness => {
                    let randomness_result_cache = match chain_id {
                        0 => self
                            .context
                            .read()
                            .await
                            .get_main_chain()
                            .get_randomness_result_cache(),
                        _ => {
                            return Err(Status::invalid_argument(
                                NodeError::InvalidChainId(chain_id).to_string(),
                            ));
                        }
                    };

                    if !randomness_result_cache
                        .read()
                        .await
                        .contains(&req.request_id)
                    {
                        return Err(Status::invalid_argument(
                            BLSTaskError::CommitterCacheNotExisted.to_string(),
                        ));
                        // because we can't assure reliability of requested partial signature to original message,
                        // we refuse to accept other node's request if the committer has not build this committer cache first.
                    }

                    let committer_cache_message = randomness_result_cache
                        .read()
                        .await
                        .get(&req.request_id)
                        .unwrap()
                        .result_cache
                        .message
                        .clone();

                    if req.message != committer_cache_message {
                        return Err(Status::invalid_argument(
                            NodeError::InvalidTaskMessage.to_string(),
                        ));
                    }

                    randomness_result_cache
                        .write()
                        .await
                        .add_partial_signature(
                            req.request_id,
                            req_id_address,
                            req.partial_signature,
                        )
                        .map_err(|_| {
                            Status::internal(BLSTaskError::CommitterCacheNotExisted.to_string())
                        })?;
                }

                _ => {
                    return Err(Status::invalid_argument(
                        NodeError::InvalidTaskType.to_string(),
                    ));
                }
            }

            return Ok(Response::new(CommitPartialSignatureReply { result: true }));
        }

        Err(Status::not_found(NodeError::MemberNotExisted.to_string()))
    }
}

pub async fn start_committer_server_with_shutdown<
    F: Future<Output = ()>,
    N: NodeInfoFetcher<PC>
        + NodeInfoUpdater<PC>
        + ContextInfoUpdater
        + std::fmt::Debug
        + Clone
        + Sync
        + Send
        + 'static,
    G: GroupInfoFetcher<PC>
        + GroupInfoUpdater<PC>
        + ContextInfoUpdater
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
        + AdapterClientBuilder<PC>
        + ChainProviderBuilder
        + std::fmt::Debug
        + Clone
        + Sync
        + Send
        + 'static,
    PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
>(
    endpoint: String,
    context: NodeContext<N, G, T, I, PC>,
    shutdown_signal: F,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = endpoint.parse()?;

    let id_address = context
        .read()
        .await
        .get_main_chain()
        .get_chain_identity()
        .read()
        .await
        .get_id_address();

    let group_cache = context.read().await.get_main_chain().get_group_cache();

    Server::builder()
        .add_service(CommitterServiceServer::with_interceptor(
            BLSCommitterServiceServer::new(id_address, group_cache, context),
            intercept,
        ))
        .serve_with_shutdown(addr, shutdown_signal)
        .await?;
    Ok(())
}

pub async fn start_committer_server<
    N: NodeInfoFetcher<PC>
        + NodeInfoUpdater<PC>
        + ContextInfoUpdater
        + std::fmt::Debug
        + Clone
        + Sync
        + Send
        + 'static,
    G: GroupInfoFetcher<PC>
        + GroupInfoUpdater<PC>
        + ContextInfoUpdater
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
        + AdapterClientBuilder<PC>
        + ChainProviderBuilder
        + std::fmt::Debug
        + Clone
        + Sync
        + Send
        + 'static,
    PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
>(
    endpoint: String,
    context: NodeContext<N, G, T, I, PC>,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = endpoint.parse()?;

    let id_address = context
        .read()
        .await
        .get_main_chain()
        .get_chain_identity()
        .read()
        .await
        .get_id_address();

    let group_cache = context.read().await.get_main_chain().get_group_cache();

    Server::builder()
        .add_service(CommitterServiceServer::with_interceptor(
            BLSCommitterServiceServer::new(id_address, group_cache, context),
            intercept,
        ))
        .serve(addr)
        .await?;
    Ok(())
}

fn intercept(req: Request<()>) -> Result<Request<()>, Status> {
    // println!("Intercepting request: {:?}", req);

    Ok(req)
}
