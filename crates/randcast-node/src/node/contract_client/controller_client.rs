use self::controller::{
    transactions_client::TransactionsClient as ControllerTransactionsClient,
    views_client::ViewsClient as ControllerViewsClient, CommitDkgRequest, GetGroupRequest,
    GroupReply, Member, NodeRegisterRequest, PostProcessDkgRequest,
};
use self::controller::{DkgTaskReply, GroupRelayTaskReply, MineRequest};
use crate::node::dao::types::{DKGTask, Group, GroupRelayTask, Member as ModelMember};
use crate::node::error::errors::{NodeError, NodeResult};
use crate::node::ServiceClient;
use async_trait::async_trait;
use std::collections::BTreeMap;
use tonic::{Code, Request};

pub mod controller {
    include!("../../../stub/controller.rs");
}

#[async_trait]
pub trait ControllerTransactions {
    async fn node_register(&self, id_public_key: Vec<u8>) -> NodeResult<()>;

    async fn commit_dkg(
        &self,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<String>,
    ) -> NodeResult<()>;

    async fn post_process_dkg(&self, group_index: usize, group_epoch: usize) -> NodeResult<()>;
}

#[async_trait]
pub trait ControllerMockHelper {
    async fn mine(&self, block_number_increment: usize) -> NodeResult<usize>;

    async fn emit_dkg_task(&self) -> NodeResult<DKGTask>;

    async fn emit_group_relay_task(&self) -> NodeResult<GroupRelayTask>;
}

#[async_trait]
pub trait ControllerViews {
    async fn get_group(&self, group_index: usize) -> NodeResult<Group>;
}

pub struct MockControllerClient {
    id_address: String,
    controller_rpc_endpoint: String,
}

impl MockControllerClient {
    pub fn new(controller_rpc_endpoint: String, id_address: String) -> Self {
        MockControllerClient {
            id_address,
            controller_rpc_endpoint,
        }
    }
}

type TransactionsClient = ControllerTransactionsClient<tonic::transport::Channel>;

#[async_trait]
impl ServiceClient<TransactionsClient> for MockControllerClient {
    async fn prepare_service_client(&self) -> NodeResult<TransactionsClient> {
        TransactionsClient::connect(format!(
            "{}{}",
            "http://",
            self.controller_rpc_endpoint.clone()
        ))
        .await
        .map_err(|err| err.into())
    }
}

type ViewsClient = ControllerViewsClient<tonic::transport::Channel>;

#[async_trait]
impl ServiceClient<ViewsClient> for MockControllerClient {
    async fn prepare_service_client(&self) -> NodeResult<ViewsClient> {
        ViewsClient::connect(format!(
            "{}{}",
            "http://",
            self.controller_rpc_endpoint.clone()
        ))
        .await
        .map_err(|err| err.into())
    }
}

#[async_trait]
impl ControllerTransactions for MockControllerClient {
    async fn node_register(&self, id_public_key: Vec<u8>) -> NodeResult<()> {
        let request = Request::new(NodeRegisterRequest {
            id_address: self.id_address.to_string(),
            id_public_key,
        });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .node_register(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn commit_dkg(
        &self,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<String>,
    ) -> NodeResult<()> {
        let request = Request::new(CommitDkgRequest {
            id_address: self.id_address.to_string(),
            group_index: group_index as u32,
            group_epoch: group_epoch as u32,
            public_key,
            partial_public_key,
            disqualified_nodes,
        });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .commit_dkg(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn post_process_dkg(&self, group_index: usize, group_epoch: usize) -> NodeResult<()> {
        let request = Request::new(PostProcessDkgRequest {
            id_address: self.id_address.to_string(),
            group_index: group_index as u32,
            group_epoch: group_epoch as u32,
        });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .post_process_dkg(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }
}

#[async_trait]
impl ControllerMockHelper for MockControllerClient {
    async fn mine(&self, block_number_increment: usize) -> NodeResult<usize> {
        let request = Request::new(MineRequest {
            block_number_increment: block_number_increment as u32,
        });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .mine(request)
            .await
            .map(|r| r.into_inner().block_number as usize)
            .map_err(|status| status.into())
    }

    async fn emit_dkg_task(&self) -> NodeResult<DKGTask> {
        let request = Request::new(());

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .emit_dkg_task(request)
            .await
            .map(|r| {
                let DkgTaskReply {
                    group_index,
                    epoch,
                    size,
                    threshold,
                    members,
                    assignment_block_height,
                    coordinator_address,
                } = r.into_inner();

                let members = members
                    .into_iter()
                    .map(|(address, index)| (address, index as usize))
                    .collect();

                DKGTask {
                    group_index: group_index as usize,
                    epoch: epoch as usize,
                    size: size as usize,
                    threshold: threshold as usize,
                    members,
                    assignment_block_height: assignment_block_height as usize,
                    coordinator_address,
                }
            })
            .map_err(|status| status.into())
    }

    async fn emit_group_relay_task(&self) -> NodeResult<GroupRelayTask> {
        let request = Request::new(());

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .emit_group_relay_task(request)
            .await
            .map(|r| {
                let GroupRelayTaskReply {
                    controller_global_epoch,
                    relayed_group_index,
                    relayed_group_epoch,
                    assignment_block_height,
                } = r.into_inner();

                GroupRelayTask {
                    controller_global_epoch: controller_global_epoch as usize,
                    relayed_group_index: relayed_group_index as usize,
                    relayed_group_epoch: relayed_group_epoch as usize,
                    assignment_block_height: assignment_block_height as usize,
                }
            })
            .map_err(|status| match status.code() {
                Code::NotFound => NodeError::NoTaskAvailable,
                _ => status.into(),
            })
    }
}

#[async_trait]
impl ControllerViews for MockControllerClient {
    async fn get_group(&self, group_index: usize) -> NodeResult<Group> {
        let request = Request::new(GetGroupRequest {
            index: group_index as u32,
        });

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_group(request)
            .await
            .map(|r| {
                let GroupReply {
                    index,
                    epoch,
                    size,
                    threshold,
                    state,
                    public_key,
                    members,
                    committers,
                    ..
                } = r.into_inner();

                let members: BTreeMap<String, ModelMember> = members
                    .into_iter()
                    .map(|(id_address, m)| (id_address, m.into()))
                    .collect();

                let public_key = if public_key.is_empty() {
                    None
                } else {
                    Some(bincode::deserialize(&public_key).unwrap())
                };

                Group {
                    index: index as usize,
                    epoch: epoch as usize,
                    size: size as usize,
                    threshold: threshold as usize,
                    state,
                    public_key,
                    members,
                    committers,
                }
            })
            .map_err(|status| status.into())
    }
}

impl From<Member> for ModelMember {
    fn from(member: Member) -> Self {
        let partial_public_key = if member.partial_public_key.is_empty() {
            None
        } else {
            Some(bincode::deserialize(&member.partial_public_key).unwrap())
        };

        ModelMember {
            index: member.index as usize,
            id_address: member.id_address,
            rpc_endpint: None,
            partial_public_key,
        }
    }
}
