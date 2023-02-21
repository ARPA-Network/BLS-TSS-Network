use crate::rpc_stub::adapter::{
    transactions_server::{
        Transactions as AdapterTransactions, TransactionsServer as AdapterTransactionsServer,
    },
    views_server::{Views as AdapterViews, ViewsServer as AdapterViewsServer},
    GetGroupRequest, GroupReply, IsTaskPendingReply, Member,
};
use crate::rpc_stub::adapter::{
    FulfillRandomnessRequest, LastOutputReply, MineReply, MineRequest, RequestRandomnessRequest,
    SignatureTaskReply,
};
use crate::{
    contract::{
        adapter::{
            AdapterMockHelper, AdapterTransactions as ModelAdapterTrxs,
            AdapterViews as ModelAdapterViews,
        },
        controller::Controller,
        controller::ControllerMockHelper,
        errors::ControllerError,
        types::{Group, Member as ModelMember, SignatureTask},
        utils::address_to_string,
    },
    rpc_stub::adapter::IsTaskPendingRequest,
};
use ethers_core::types::{Address, U256};
use parking_lot::RwLock;
use std::{collections::BTreeMap, sync::Arc};
use tonic::{transport::Server, Request, Response, Status};

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
            .request_randomness(U256::from_big_endian(&req.seed))
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn fulfill_randomness(
        &self,
        request: Request<FulfillRandomnessRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let partial_signatures = req
            .partial_signatures
            .into_iter()
            .map(|(id_address, signature)| (id_address.parse::<Address>().unwrap(), signature))
            .collect();

        self.adapter
            .write()
            .fulfill_randomness(
                &req.id_address.parse::<Address>().unwrap(),
                req.group_index as usize,
                req.request_id,
                req.signature,
                partial_signatures,
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
                    .map(|(id_address, m)| (address_to_string(id_address), m.into()))
                    .collect();

                let committers = committers.into_iter().map(address_to_string).collect();

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
                    request_id,
                    seed,
                    group_index,
                    assignment_block_height,
                } = signature_task;

                let mut seed_bytes = vec![0u8; 32];
                seed.to_big_endian(&mut seed_bytes);

                Response::new(SignatureTaskReply {
                    request_id,
                    seed: seed_bytes,
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
        let mut b1 = vec![0u8; 32];
        last_output.to_big_endian(&mut b1);
        return Ok(Response::new(LastOutputReply { last_output: b1 }));
    }

    async fn is_task_pending(
        &self,
        request: Request<IsTaskPendingRequest>,
    ) -> Result<Response<IsTaskPendingReply>, Status> {
        let req = request.into_inner();

        let state = self.adapter.read().is_task_pending(&req.request_id);

        return Ok(Response::new(IsTaskPendingReply { state }));
    }
}

impl From<ModelMember> for Member {
    fn from(member: ModelMember) -> Self {
        Member {
            index: member.index as u32,
            id_address: address_to_string(member.id_address),
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
