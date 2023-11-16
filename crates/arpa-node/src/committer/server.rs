use crate::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    context::{types::GeneralContext, Context},
    error::NodeError,
};
use crate::{
    context::chain::Chain,
    rpc_stub::committer::{
        committer_service_server::{CommitterService, CommitterServiceServer},
        CommitPartialSignatureReply, CommitPartialSignatureRequest,
    },
};
use arpa_core::{BLSTaskError, BLSTaskType, SchedulerError};
use arpa_dal::GroupInfoHandler;
use ethers::types::Address;
use futures::Future;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::{
    group::Curve,
    sig::{SignatureScheme, ThresholdScheme},
};
use tokio::sync::RwLock;
use tonic::{transport::Server, Request, Response, Status};

type NodeContext<PC, S> = Arc<RwLock<GeneralContext<PC, S>>>;

pub(crate) struct BLSCommitterServiceServer<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
> {
    id_address: Address,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    context: NodeContext<PC, S>,
    c: PhantomData<PC>,
    s: PhantomData<S>,
}

impl<PC: Curve, S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>>
    BLSCommitterServiceServer<PC, S>
{
    pub fn new(
        id_address: Address,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        context: NodeContext<PC, S>,
    ) -> Self {
        BLSCommitterServiceServer {
            id_address,
            group_cache,
            context,
            c: PhantomData,
            s: PhantomData,
        }
    }
}

#[tonic::async_trait]
impl<
        PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
    > CommitterService for BLSCommitterServiceServer<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
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

        let chain_id = req.chain_id as usize;

        let req_id_address: Address = req
            .id_address
            .parse()
            .map_err(|_| Status::invalid_argument(NodeError::AddressFormatError.to_string()))?;

        if let Ok(member) = self.group_cache.read().await.get_member(req_id_address) {
            let partial_public_key = member.partial_public_key.clone().unwrap();

            SimpleBLSCore::<PC, S>::partial_verify(
                &partial_public_key,
                &req.message,
                &req.partial_signature,
            )
            .map_err(|e| Status::internal(e.to_string()))?;

            match BLSTaskType::from(req.task_type) {
                BLSTaskType::Randomness => {
                    let main_chain_id = self
                        .context
                        .read()
                        .await
                        .get_main_chain()
                        .get_chain_identity()
                        .read()
                        .await
                        .get_chain_id();

                    let randomness_result_cache = if chain_id == main_chain_id {
                        self.context
                            .read()
                            .await
                            .get_main_chain()
                            .get_randomness_result_cache()
                    } else {
                        if !self.context.read().await.contains_relayed_chain(chain_id) {
                            return Err(Status::invalid_argument(
                                SchedulerError::InvalidChainId(chain_id).to_string(),
                            ));
                        }
                        self.context
                            .read()
                            .await
                            .get_relayed_chain(req.chain_id as usize)
                            .unwrap()
                            .get_randomness_result_cache()
                    };

                    if !randomness_result_cache
                        .read()
                        .await
                        .contains(&req.request_id)
                        .await
                        .map_err(|e| Status::internal(e.to_string()))?
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
                        .await
                        .unwrap()
                        .result_cache
                        .message
                        .clone();

                    if req.message != committer_cache_message {
                        return Err(Status::invalid_argument(
                            NodeError::InvalidTaskMessage.to_string(),
                        ));
                    }

                    if !randomness_result_cache
                        .write()
                        .await
                        .add_partial_signature(
                            req.request_id,
                            req_id_address,
                            req.partial_signature,
                        )
                        .await
                        .map_err(|e| Status::internal(e.to_string()))?
                    {
                        return Err(Status::invalid_argument(
                            BLSTaskError::AlreadyCommittedPartialSignature.to_string(),
                        ));
                    }
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
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    S: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + Clone
        + Send
        + Sync
        + 'static,
>(
    endpoint: String,
    context: NodeContext<PC, S>,
    shutdown_signal: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
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
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    S: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + std::fmt::Debug
        + Clone
        + Sync
        + Send
        + 'static,
>(
    endpoint: String,
    context: NodeContext<PC, S>,
) -> Result<(), Box<dyn std::error::Error>>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
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
