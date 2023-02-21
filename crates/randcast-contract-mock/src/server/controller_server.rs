use super::adapter_server::MockAdapter;
use crate::contract::{
    controller::{
        Controller, ControllerMockHelper, ControllerTransactions as ModelControllerTrxs,
        ControllerViews as ModelControllerViews, COORDINATOR_ADDRESS_PREFIX,
    },
    coordinator::{Transactions, Views},
    errors::ControllerError,
    types::{DKGTask, Member as ModelMember, Node},
    utils::address_to_string,
};
use crate::rpc_stub::adapter::{
    transactions_server::TransactionsServer as AdapterTransactionsServer,
    views_server::ViewsServer as AdapterViewsServer,
};
use crate::rpc_stub::controller::{
    transactions_server::{
        Transactions as ControllerTransactions, TransactionsServer as ControllerTransactionsServer,
    },
    views_server::{Views as ControllerViews, ViewsServer as ControllerViewsServer},
    CommitDkgRequest, GetNodeRequest, Member, NodeRegisterRequest, NodeReply,
    PostProcessDkgRequest,
};
use crate::rpc_stub::controller::{DkgTaskReply, MineReply, MineRequest};
use crate::rpc_stub::coordinator::{
    transactions_server::{
        Transactions as CoordinatorTransactions,
        TransactionsServer as CoordinatorTransactionsServer,
    },
    views_server::{Views as CoordinatorViews, ViewsServer as CoordinatorViewsServer},
    BlsKeysReply, InPhaseReply, JustificationsReply, ParticipantsReply, PublishRequest,
    ResponsesReply, SharesReply,
};
use ethers_core::types::Address;
use parking_lot::RwLock;
use std::sync::Arc;
use tonic::{transport::Server, Request, Response, Status};

pub struct MockController {
    controller: Arc<RwLock<Controller>>,
}

impl MockController {
    pub fn new(controller: Arc<RwLock<Controller>>) -> Self {
        MockController { controller }
    }
}

pub struct MockCoordinator {
    controller: Arc<RwLock<Controller>>,
}

impl MockCoordinator {
    pub fn new(controller: Arc<RwLock<Controller>>) -> Self {
        MockCoordinator { controller }
    }

    fn check_and_fetch_coordinator_group_index_from_request<T>(
        &self,
        req: &Request<T>,
    ) -> Result<usize, Status> {
        let address = req
            .metadata()
            .get("address")
            .ok_or_else(|| Status::invalid_argument("coordinator address is empty"))?
            .to_str()
            .map(|i| i.to_string())
            .map_err(|_| Status::invalid_argument("coordinator address is invalid"))?;

        let req_index = address[COORDINATOR_ADDRESS_PREFIX.len()..]
            .parse()
            .map_err(|_| Status::invalid_argument("invalid coordinator address format"))?;

        let controller = self.controller.read();

        controller.coordinators.get(&req_index).ok_or_else(|| {
            Status::not_found(ControllerError::CoordinatorNotExisted(req_index).to_string())
        })?;

        Ok(req_index)
    }
}

#[tonic::async_trait]
impl ControllerTransactions for MockController {
    async fn node_register(
        &self,
        request: Request<NodeRegisterRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let id_address = req.id_address.parse::<Address>().unwrap();

        self.controller
            .write()
            .node_register(id_address, req.id_public_key)
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn commit_dkg(&self, request: Request<CommitDkgRequest>) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let id_address = req.id_address.parse::<Address>().unwrap();

        let disqualified_nodes = req
            .disqualified_nodes
            .into_iter()
            .map(|i| i.parse::<Address>().unwrap())
            .collect();

        self.controller
            .write()
            .commit_dkg(
                &id_address,
                req.group_index as usize,
                req.group_epoch as usize,
                req.public_key,
                req.partial_public_key,
                disqualified_nodes,
            )
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn post_process_dkg(
        &self,
        request: Request<PostProcessDkgRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let id_address = req.id_address.parse::<Address>().unwrap();

        self.controller
            .write()
            .post_process_dkg(
                &id_address,
                req.group_index as usize,
                req.group_epoch as usize,
            )
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn mine(&self, request: Request<MineRequest>) -> Result<Response<MineReply>, Status> {
        let req = request.into_inner();

        self.controller
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
impl ControllerViews for MockController {
    async fn get_node(
        &self,
        request: Request<GetNodeRequest>,
    ) -> Result<Response<NodeReply>, Status> {
        let req = request.into_inner();

        let id_address = req.id_address.parse::<Address>().unwrap();

        match self.controller.read().get_node(&id_address) {
            Some(node) => Ok(Response::new(node.clone().into())),
            None => Err(Status::not_found(
                ControllerError::NodeNotExisted.to_string(),
            )),
        }
    }

    async fn emit_dkg_task(&self, _request: Request<()>) -> Result<Response<DkgTaskReply>, Status> {
        self.controller
            .read()
            .emit_dkg_task()
            .map(|dkg_task| {
                let DKGTask {
                    group_index,
                    epoch,
                    size,
                    threshold,
                    members,
                    assignment_block_height,
                    coordinator_address,
                } = dkg_task;

                let members = members
                    .into_iter()
                    .map(|(address, index)| (address_to_string(address), index as u32))
                    .collect();

                Response::new(DkgTaskReply {
                    group_index: group_index as u32,
                    epoch: epoch as u32,
                    size: size as u32,
                    threshold: threshold as u32,
                    members,
                    assignment_block_height: assignment_block_height as u32,
                    coordinator_address: address_to_string(coordinator_address),
                })
            })
            .map_err(|e| Status::not_found(e.to_string()))
    }
}

#[tonic::async_trait]
impl CoordinatorTransactions for MockCoordinator {
    async fn publish(&self, request: Request<PublishRequest>) -> Result<Response<()>, Status> {
        let req_index = self.check_and_fetch_coordinator_group_index_from_request(&request)?;

        let req = request.into_inner();

        let id_address = req.id_address.parse::<Address>().unwrap();

        self.controller
            .write()
            .coordinators
            .get_mut(&req_index)
            .unwrap()
            .1
            .publish(id_address, req.value)
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }
}

#[tonic::async_trait]
impl CoordinatorViews for MockCoordinator {
    async fn get_shares(&self, request: Request<()>) -> Result<Response<SharesReply>, Status> {
        let req_index = self.check_and_fetch_coordinator_group_index_from_request(&request)?;

        self.controller
            .read()
            .coordinators
            .get(&req_index)
            .unwrap()
            .1
            .get_shares()
            .map(|shares| Response::new(SharesReply { shares }))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_responses(
        &self,
        request: Request<()>,
    ) -> Result<Response<ResponsesReply>, Status> {
        let req_index = self.check_and_fetch_coordinator_group_index_from_request(&request)?;

        self.controller
            .read()
            .coordinators
            .get(&req_index)
            .unwrap()
            .1
            .get_responses()
            .map(|responses| Response::new(ResponsesReply { responses }))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_justifications(
        &self,
        request: Request<()>,
    ) -> Result<Response<JustificationsReply>, Status> {
        let req_index = self.check_and_fetch_coordinator_group_index_from_request(&request)?;

        self.controller
            .read()
            .coordinators
            .get(&req_index)
            .unwrap()
            .1
            .get_justifications()
            .map(|justifications| Response::new(JustificationsReply { justifications }))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_participants(
        &self,
        request: Request<()>,
    ) -> Result<Response<ParticipantsReply>, Status> {
        let req_index = self.check_and_fetch_coordinator_group_index_from_request(&request)?;

        self.controller
            .read()
            .coordinators
            .get(&req_index)
            .unwrap()
            .1
            .get_participants()
            .map(|participants| {
                let participants = participants.into_iter().map(address_to_string).collect();

                Response::new(ParticipantsReply { participants })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_bls_keys(
        &self,
        request: Request<()>,
    ) -> Result<Response<BlsKeysReply>, tonic::Status> {
        let req_index = self.check_and_fetch_coordinator_group_index_from_request(&request)?;

        self.controller
            .read()
            .coordinators
            .get(&req_index)
            .unwrap()
            .1
            .get_bls_keys()
            .map(|(threshold, bls_keys)| {
                Response::new(BlsKeysReply {
                    threshold: threshold as u32,
                    bls_keys,
                })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn in_phase(&self, request: Request<()>) -> Result<Response<InPhaseReply>, Status> {
        let req_index = self.check_and_fetch_coordinator_group_index_from_request(&request)?;

        self.controller
            .read()
            .coordinators
            .get(&req_index)
            .unwrap()
            .1
            .in_phase()
            .map(|phase| {
                Response::new(InPhaseReply {
                    phase: phase as u32,
                })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }
}

impl From<Node> for NodeReply {
    fn from(n: Node) -> Self {
        let mut b1 = vec![0u8; 32];
        n.staking.to_big_endian(&mut b1);

        NodeReply {
            id_address: address_to_string(n.id_address),
            id_public_key: n.id_public_key,
            state: n.state,
            pending_until_block: n.pending_until_block as u32,
            staking: b1,
        }
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

pub async fn start_controller_server(
    endpoint: String,
    controller: Arc<RwLock<Controller>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = endpoint.parse()?;

    Server::builder()
        .add_service(ControllerTransactionsServer::with_interceptor(
            MockController::new(controller.clone()),
            intercept,
        ))
        .add_service(ControllerViewsServer::with_interceptor(
            MockController::new(controller.clone()),
            intercept,
        ))
        .add_service(CoordinatorTransactionsServer::with_interceptor(
            MockCoordinator::new(controller.clone()),
            intercept,
        ))
        .add_service(CoordinatorViewsServer::with_interceptor(
            MockCoordinator::new(controller.clone()),
            intercept,
        ))
        .add_service(AdapterTransactionsServer::with_interceptor(
            MockAdapter::new(controller.clone()),
            intercept,
        ))
        .add_service(AdapterViewsServer::with_interceptor(
            MockAdapter::new(controller.clone()),
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
