use self::controller::{
    transactions_server::{
        Transactions as ControllerTransactions, TransactionsServer as ControllerTransactionsServer,
    },
    views_server::{Views as ControllerViews, ViewsServer as ControllerViewsServer},
    CommitDkgRequest, GetGroupRequest, GroupReply, Member, NodeRegisterRequest,
    PostProcessDkgRequest,
};
use self::coordinator::{
    transactions_server::{
        Transactions as CoordinatorTransactions,
        TransactionsServer as CoordinatorTransactionsServer,
    },
    views_server::{Views as CoordinatorViews, ViewsServer as CoordinatorViewsServer},
    BlsKeysReply, InPhaseReply, JustificationsReply, ParticipantsReply, PublishRequest,
    ResponsesReply, SharesReply,
};
use super::adapter_server::{
    adapter::{
        transactions_server::TransactionsServer as AdapterTransactionsServer,
        views_server::ViewsServer as AdapterViewsServer,
    },
    MockAdapter,
};
use crate::contract::{
    adapter::AdapterViews,
    controller::{Controller, ControllerMockHelper, ControllerTransactions as ModelControllerTrxs},
    coordinator::{Transactions, Views},
    errors::ControllerError,
    types::{DKGTask, Group, GroupRelayTask, Member as ModelMember},
};
use controller::{DkgTaskReply, GroupRelayTaskReply, MineReply, MineRequest};
use parking_lot::RwLock;
use std::{collections::BTreeMap, sync::Arc};
use tonic::{transport::Server, Request, Response, Status};

pub mod controller {
    include!("../../stub/controller.rs");
}

pub mod coordinator {
    include!("../../stub/coordinator.rs");
}

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

    fn check_coordinator_index_and_epoch<T>(
        &self,
        req: &Request<T>,
    ) -> Result<(usize, usize), Status> {
        let req_index = req
            .metadata()
            .get("index")
            .ok_or_else(|| Status::invalid_argument("group index is empty"))?
            .to_str()
            .map(|i| i.parse::<usize>().unwrap())
            .map_err(|_| Status::invalid_argument("group index is invalid"))?;

        let req_epoch = req
            .metadata()
            .get("epoch")
            .ok_or_else(|| Status::invalid_argument("group epoch is empty"))?
            .to_str()
            .map(|i| i.parse::<usize>().unwrap())
            .map_err(|_| Status::invalid_argument("group epoch is invalid"))?;

        let controller = self.controller.read();

        let (_, coordinator) = controller
            .coordinators
            .get(&req_index)
            .ok_or_else(|| Status::not_found(ControllerError::CoordinatorNotExisted.to_string()))?;

        if coordinator.epoch != req_epoch {
            return Err(Status::internal(
                ControllerError::CoordinatorEpochObsolete(controller.epoch).to_string(),
            ));
        }

        Ok((req_index, req_epoch))
    }
}

#[tonic::async_trait]
impl ControllerTransactions for MockController {
    async fn node_register(
        &self,
        request: Request<NodeRegisterRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        self.controller
            .write()
            .node_register(req.id_address, req.id_public_key)
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn commit_dkg(&self, request: Request<CommitDkgRequest>) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        self.controller
            .write()
            .commit_dkg(
                req.id_address,
                req.group_index as usize,
                req.group_epoch as usize,
                req.public_key,
                req.partial_public_key,
                req.disqualified_nodes,
            )
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn post_process_dkg(
        &self,
        request: Request<PostProcessDkgRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        self.controller
            .write()
            .post_process_dkg(
                &req.id_address,
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
    async fn get_group(
        &self,
        request: Request<GetGroupRequest>,
    ) -> Result<Response<GroupReply>, Status> {
        let req = request.into_inner();

        match self.controller.read().get_group(req.index as usize) {
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
                    .map(|(address, index)| (address, index as u32))
                    .collect();

                Response::new(DkgTaskReply {
                    group_index: group_index as u32,
                    epoch: epoch as u32,
                    size: size as u32,
                    threshold: threshold as u32,
                    members,
                    assignment_block_height: assignment_block_height as u32,
                    coordinator_address,
                })
            })
            .map_err(|e| Status::not_found(e.to_string()))
    }

    async fn emit_group_relay_task(
        &self,
        _request: Request<()>,
    ) -> Result<Response<GroupRelayTaskReply>, Status> {
        self.controller
            .read()
            .emit_group_relay_task()
            .map(|group_relay_task| {
                let GroupRelayTask {
                    controller_global_epoch,
                    relayed_group_index,
                    relayed_group_epoch,
                    assignment_block_height,
                } = group_relay_task;

                Response::new(GroupRelayTaskReply {
                    controller_global_epoch: controller_global_epoch as u32,
                    relayed_group_index: relayed_group_index as u32,
                    relayed_group_epoch: relayed_group_epoch as u32,
                    assignment_block_height: assignment_block_height as u32,
                })
            })
            .map_err(|e| Status::not_found(e.to_string()))
    }
}

#[tonic::async_trait]
impl CoordinatorTransactions for MockCoordinator {
    async fn publish(&self, request: Request<PublishRequest>) -> Result<Response<()>, Status> {
        let (req_index, _) = self.check_coordinator_index_and_epoch(&request)?;

        let req = request.into_inner();

        self.controller
            .write()
            .coordinators
            .get_mut(&req_index)
            .unwrap()
            .1
            .publish(req.id_address, req.value)
            .map(|()| Response::new(()))
            .map_err(|e| Status::internal(e.to_string()))
    }
}

#[tonic::async_trait]
impl CoordinatorViews for MockCoordinator {
    async fn get_shares(&self, request: Request<()>) -> Result<Response<SharesReply>, Status> {
        let (req_index, _) = self.check_coordinator_index_and_epoch(&request)?;

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
        let (req_index, _) = self.check_coordinator_index_and_epoch(&request)?;

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
        let (req_index, _) = self.check_coordinator_index_and_epoch(&request)?;

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
        let (req_index, _) = self.check_coordinator_index_and_epoch(&request)?;

        self.controller
            .read()
            .coordinators
            .get(&req_index)
            .unwrap()
            .1
            .get_participants()
            .map(|participants| Response::new(ParticipantsReply { participants }))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_bls_keys(
        &self,
        request: Request<()>,
    ) -> Result<Response<BlsKeysReply>, tonic::Status> {
        let (req_index, _) = self.check_coordinator_index_and_epoch(&request)?;

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
        let (req_index, _) = self.check_coordinator_index_and_epoch(&request)?;

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

impl From<ModelMember> for Member {
    fn from(member: ModelMember) -> Self {
        Member {
            index: member.index as u32,
            id_address: member.id_address,
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
