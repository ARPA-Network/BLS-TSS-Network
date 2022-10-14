use self::adapter::{
    transactions_server::{
        Transactions as AdapterTransactions, TransactionsServer as AdapterTransactionsServer,
    },
    views_server::{Views as AdapterViews, ViewsServer as AdapterViewsServer},
    GetGroupRequest, GroupReply, Member,
};
use crate::contract::{
    adapter::{
        AdapterMockHelper, AdapterTransactions as ModelAdapterTrxs,
        AdapterViews as ModelAdapterViews,
    },
    controller::Controller,
    controller::ControllerMockHelper,
    errors::ControllerError,
    types::{Group, GroupRelayConfirmationTask, Member as ModelMember, SignatureTask},
};
use adapter::{
    CancelInvalidRelayConfirmationTaskRequest, ConfirmRelayRequest, FulfillRandomnessRequest,
    FulfillRelayRequest, GetGroupRelayCacheRequest, GetGroupRelayConfirmationTaskStateReply,
    GetGroupRelayConfirmationTaskStateRequest, GetSignatureTaskCompletionStateReply,
    GetSignatureTaskCompletionStateRequest, GroupRelayConfirmationTaskReply, LastOutputReply,
    MineReply, MineRequest, RequestRandomnessRequest, SetInitialGroupRequest, SignatureTaskReply,
};
use parking_lot::RwLock;
use std::{collections::BTreeMap, sync::Arc};
use tonic::{transport::Server, Request, Response, Status};

pub mod adapter {
    include!("../../stub/adapter.rs");
}

pub struct MockAdapter {
    adapter: Arc<RwLock<Controller>>,
}

impl MockAdapter {
    pub fn new(adapter: Arc<RwLock<Controller>>) -> Self {
        MockAdapter { adapter }
    }
}

#[tonic::async_trait]
impl AdapterTransactions for MockAdapter {
    async fn request_randomness(
        &self,
        request: Request<RequestRandomnessRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        self.adapter
            .write()
            .request_randomness(&req.message)
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn fulfill_randomness(
        &self,
        request: Request<FulfillRandomnessRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        self.adapter
            .write()
            .fulfill_randomness(
                &req.id_address,
                req.group_index as usize,
                req.signature_index as usize,
                req.signature,
                req.partial_signatures,
            )
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn mine(&self, request: Request<MineRequest>) -> Result<Response<MineReply>, Status> {
        let req = request.into_inner();

        self.adapter
            .write()
            .mine(req.block_number_increment as usize)
            .map(|block_number| {
                Response::new(MineReply {
                    block_number: block_number as u32,
                })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn fulfill_relay(
        &self,
        request: Request<FulfillRelayRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        self.adapter
            .write()
            .fulfill_relay(
                &req.id_address,
                req.relayer_group_index as usize,
                req.task_index as usize,
                req.signature,
                req.group_as_bytes,
            )
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn set_initial_group(
        &self,
        request: Request<SetInitialGroupRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        self.adapter
            .write()
            .set_initial_group(&req.id_address, req.group)
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn cancel_invalid_relay_confirmation_task(
        &self,
        request: Request<CancelInvalidRelayConfirmationTaskRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        self.adapter
            .write()
            .cancel_invalid_relay_confirmation_task(&req.id_address, req.task_index as usize)
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn confirm_relay(
        &self,
        request: Request<ConfirmRelayRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        self.adapter
            .write()
            .confirm_relay(
                &req.id_address,
                req.task_index as usize,
                req.group_relay_confirmation_as_bytes,
                req.signature,
            )
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }
}

#[tonic::async_trait]
impl AdapterViews for MockAdapter {
    async fn get_group(
        &self,
        request: Request<GetGroupRequest>,
    ) -> Result<Response<GroupReply>, Status> {
        let req = request.into_inner();

        match self.adapter.read().get_group(req.index as usize) {
            Some(group) => {
                let Group {
                    index,
                    epoch,
                    capacity,
                    size,
                    threshold,
                    is_strictly_majority_consensus_reached,
                    public_key,
                    members,
                    committers,
                    ..
                } = group.clone();

                let members: BTreeMap<String, Member> = members
                    .into_iter()
                    .map(|(id_address, m)| (id_address, m.into()))
                    .collect();

                Ok(Response::new(GroupReply {
                    index: index as u32,
                    epoch: epoch as u32,
                    capacity: capacity as u32,
                    size: size as u32,
                    threshold: threshold as u32,
                    state: is_strictly_majority_consensus_reached,
                    public_key,
                    members,
                    committers,
                }))
            }
            None => Err(Status::not_found(
                ControllerError::GroupNotExisted.to_string(),
            )),
        }
    }

    async fn emit_signature_task(
        &self,
        _request: Request<()>,
    ) -> Result<Response<SignatureTaskReply>, Status> {
        self.adapter
            .read()
            .emit_signature_task()
            .map(|signature_task| {
                let SignatureTask {
                    index,
                    message,
                    group_index,
                    assignment_block_height,
                } = signature_task;

                Response::new(SignatureTaskReply {
                    index: index as u32,
                    message,
                    group_index: group_index as u32,
                    assignment_block_height: assignment_block_height as u32,
                })
            })
            .map_err(|e| Status::not_found(e.to_string()))
    }

    async fn get_last_output(
        &self,
        _request: Request<()>,
    ) -> Result<Response<LastOutputReply>, Status> {
        let last_output = self.adapter.read().get_last_output();
        return Ok(Response::new(LastOutputReply { last_output }));
    }

    async fn get_signature_task_completion_state(
        &self,
        request: Request<GetSignatureTaskCompletionStateRequest>,
    ) -> Result<Response<GetSignatureTaskCompletionStateReply>, Status> {
        let req = request.into_inner();

        let state = self
            .adapter
            .read()
            .get_signature_task_completion_state(req.index as usize);

        return Ok(Response::new(GetSignatureTaskCompletionStateReply {
            state,
        }));
    }

    async fn get_group_relay_cache(
        &self,
        request: Request<GetGroupRelayCacheRequest>,
    ) -> Result<Response<GroupReply>, Status> {
        let req = request.into_inner();

        match self
            .adapter
            .read()
            .get_group_relay_cache(req.index as usize)
        {
            Some(group) => {
                let Group {
                    index,
                    epoch,
                    capacity,
                    size,
                    threshold,
                    is_strictly_majority_consensus_reached,
                    public_key,
                    members,
                    committers,
                    ..
                } = group.clone();

                let members: BTreeMap<String, Member> = members
                    .into_iter()
                    .map(|(id_address, m)| (id_address, m.into()))
                    .collect();

                Ok(Response::new(GroupReply {
                    index: index as u32,
                    epoch: epoch as u32,
                    capacity: capacity as u32,
                    size: size as u32,
                    threshold: threshold as u32,
                    state: is_strictly_majority_consensus_reached,
                    public_key,
                    members,
                    committers,
                }))
            }
            None => Err(Status::not_found(
                ControllerError::GroupNotExisted.to_string(),
            )),
        }
    }

    async fn emit_group_relay_confirmation_task(
        &self,
        _request: Request<()>,
    ) -> Result<Response<GroupRelayConfirmationTaskReply>, Status> {
        self.adapter
            .read()
            .emit_group_relay_confirmation_task()
            .map(|group_relay_confirmation_task| {
                let GroupRelayConfirmationTask {
                    index,
                    group_relay_cache_index,
                    relayed_group_index,
                    relayed_group_epoch,
                    relayer_group_index,
                    assignment_block_height,
                } = group_relay_confirmation_task;

                Response::new(GroupRelayConfirmationTaskReply {
                    index: index as u32,
                    group_relay_cache_index: group_relay_cache_index as u32,
                    relayed_group_index: relayed_group_index as u32,
                    relayed_group_epoch: relayed_group_epoch as u32,
                    relayer_group_index: relayer_group_index as u32,
                    assignment_block_height: assignment_block_height as u32,
                })
            })
            .map_err(|e| Status::not_found(e.to_string()))
    }

    async fn get_group_relay_confirmation_task_state(
        &self,
        request: Request<GetGroupRelayConfirmationTaskStateRequest>,
    ) -> Result<Response<GetGroupRelayConfirmationTaskStateReply>, Status> {
        let req = request.into_inner();

        let state = self
            .adapter
            .read()
            .get_group_relay_confirmation_task_state(req.index as usize);

        return Ok(Response::new(GetGroupRelayConfirmationTaskStateReply {
            state,
        }));
    }
}

impl From<ModelMember> for Member {
    fn from(member: ModelMember) -> Self {
        Member {
            index: member.index as u32,
            id_address: member.id_address,
            partial_public_key: member.partial_public_key,
        }
    }
}

pub async fn start_adapter_server(
    endpoint: String,
    adapter: Arc<RwLock<Controller>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = endpoint.parse()?;

    Server::builder()
        .add_service(AdapterTransactionsServer::with_interceptor(
            MockAdapter::new(adapter.clone()),
            intercept,
        ))
        .add_service(AdapterViewsServer::with_interceptor(
            MockAdapter::new(adapter),
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
