use self::committer_stub::{
    committer_service_server::{CommitterService, CommitterServiceServer},
    CommitPartialSignatureReply, CommitPartialSignatureRequest,
};
use crate::node::context::chain::MainChainFetcher;
use crate::node::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    context::{
        chain::{AdapterChainFetcher, ChainFetcher},
        types::GeneralContext,
        ContextFetcher,
    },
    error::NodeError,
};
use arpa_node_contract_client::{
    adapter::AdapterClientBuilder, controller::ControllerClientBuilder,
    coordinator::CoordinatorClientBuilder, provider::ChainProviderBuilder,
};
use arpa_node_core::{ChainIdentity, RandomnessTask, TaskError, TaskType};
use arpa_node_dal::{
    BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater, NodeInfoFetcher,
    SignatureResultCacheFetcher, SignatureResultCacheUpdater,
};
use ethers::types::Address;
use futures::Future;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{transport::Server, Request, Response, Status};

pub mod committer_stub {
    include!("../../../rpc_stub/committer.rs");
}

pub(crate) struct BLSCommitterServiceServer<
    N: NodeInfoFetcher,
    G: GroupInfoFetcher + GroupInfoUpdater,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
    I: ChainIdentity
        + ControllerClientBuilder
        + CoordinatorClientBuilder
        + AdapterClientBuilder
        + ChainProviderBuilder,
> {
    id_address: Address,
    group_cache: Arc<RwLock<G>>,
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
    > BLSCommitterServiceServer<N, G, T, I>
{
    pub fn new(
        id_address: Address,
        group_cache: Arc<RwLock<G>>,
        context: Arc<RwLock<GeneralContext<N, G, T, I>>>,
    ) -> Self {
        BLSCommitterServiceServer {
            id_address,
            group_cache,
            context,
        }
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
    > CommitterService for BLSCommitterServiceServer<N, G, T, I>
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
            let partial_public_key = member.partial_public_key.unwrap();

            let bls_core = SimpleBLSCore {};

            bls_core
                .partial_verify(&partial_public_key, &req.message, &req.partial_signature)
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
                            if !self.context.read().await.contains_chain(chain_id) {
                                return Err(Status::invalid_argument(
                                    NodeError::InvalidChainId(chain_id).to_string(),
                                ));
                            }
                            self.context
                                .read()
                                .await
                                .get_adapter_chain(req.chain_id as usize)
                                .unwrap()
                                .get_randomness_result_cache()
                        }
                    };

                    if !randomness_result_cache
                        .read()
                        .await
                        .contains(req.signature_index as usize)
                    {
                        return Err(Status::invalid_argument(
                            TaskError::CommitterCacheNotExisted.to_string(),
                        ));
                        // because we can't assure reliability of requested partial signature to original message,
                        // we refuse to accept other node's request if the committer has not build this committer cache first.
                    }

                    let committer_cache_message = randomness_result_cache
                        .read()
                        .await
                        .get(req.signature_index as usize)
                        .unwrap()
                        .result_cache
                        .message
                        .to_string();

                    let req_message = String::from_utf8(req.message)
                        .map_err(|e| Status::internal(e.to_string()))?;

                    if req_message != committer_cache_message {
                        return Err(Status::invalid_argument(
                            NodeError::InvalidTaskMessage.to_string(),
                        ));
                    }

                    randomness_result_cache
                        .write()
                        .await
                        .add_partial_signature(
                            req.signature_index as usize,
                            req_id_address,
                            req.partial_signature,
                        )
                        .map_err(|_| {
                            Status::internal(TaskError::CommitterCacheNotExisted.to_string())
                        })?;
                }

                TaskType::GroupRelay => {
                    let group_relay_result_cache = self
                        .context
                        .read()
                        .await
                        .get_main_chain()
                        .get_group_relay_result_cache();

                    if !group_relay_result_cache
                        .read()
                        .await
                        .contains(req.signature_index as usize)
                    {
                        return Err(Status::invalid_argument(
                            TaskError::CommitterCacheNotExisted.to_string(),
                        ));
                        // because we can't assure reliability of requested partial signature to original message,
                        // we refuse to accept other node's request if the committer has not build this committer cache first.
                    }

                    let relayed_group = group_relay_result_cache
                        .read()
                        .await
                        .get(req.signature_index as usize)
                        .unwrap()
                        .result_cache
                        .relayed_group
                        .clone();

                    let relayed_group_as_bytes = bincode::serialize(&relayed_group).unwrap();

                    if req.message != relayed_group_as_bytes {
                        return Err(Status::invalid_argument(
                            NodeError::InvalidTaskMessage.to_string(),
                        ));
                    }

                    group_relay_result_cache
                        .write()
                        .await
                        .add_partial_signature(
                            req.signature_index as usize,
                            req_id_address,
                            req.partial_signature,
                        )
                        .map_err(|_| {
                            Status::internal(TaskError::CommitterCacheNotExisted.to_string())
                        })?;
                }
                TaskType::GroupRelayConfirmation => {
                    if chain_id == 0 || !self.context.read().await.contains_chain(chain_id) {
                        return Err(Status::invalid_argument(
                            NodeError::InvalidChainId(chain_id).to_string(),
                        ));
                    }

                    let group_relay_confirmation_result_cache = self
                        .context
                        .read()
                        .await
                        .get_adapter_chain(req.chain_id as usize)
                        .unwrap()
                        .get_group_relay_confirmation_result_cache();

                    if !group_relay_confirmation_result_cache
                        .read()
                        .await
                        .contains(req.signature_index as usize)
                    {
                        return Err(Status::invalid_argument(
                            TaskError::CommitterCacheNotExisted.to_string(),
                        ));
                        // because we can't assure reliability of requested partial signature to original message,
                        // we refuse to accept other node's request if the committer has not build this committer cache first.
                    }

                    let group_relay_confirmation = group_relay_confirmation_result_cache
                        .read()
                        .await
                        .get(req.signature_index as usize)
                        .unwrap()
                        .result_cache
                        .group_relay_confirmation
                        .clone();

                    let group_relay_confirmation_as_bytes =
                        bincode::serialize(&group_relay_confirmation).unwrap();

                    if req.message != group_relay_confirmation_as_bytes {
                        return Err(Status::invalid_argument(
                            NodeError::InvalidTaskMessage.to_string(),
                        ));
                    }

                    group_relay_confirmation_result_cache
                        .write()
                        .await
                        .add_partial_signature(
                            req.signature_index as usize,
                            req_id_address,
                            req.partial_signature,
                        )
                        .map_err(|_| {
                            Status::internal(TaskError::CommitterCacheNotExisted.to_string())
                        })?;
                }
            }

            return Ok(Response::new(CommitPartialSignatureReply { result: true }));
        }

        Err(Status::not_found(NodeError::MemberNotExisted.to_string()))
    }
}

pub async fn start_committer_server_with_shutdown<
    F: Future<Output = ()>,
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
