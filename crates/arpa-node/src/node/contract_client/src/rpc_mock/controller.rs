use std::future::Future;

use crate::controller::{
    ControllerClientBuilder, ControllerLogs, ControllerTransactions, ControllerViews,
};
use crate::error::{ContractClientError, ContractClientResult};
use crate::ServiceClient;

use crate::rpc_stub::controller::{
    transactions_client::TransactionsClient as ControllerTransactionsClient,
    views_client::ViewsClient as ControllerViewsClient, CommitDkgRequest, GetNodeRequest, Member,
    NodeRegisterRequest, NodeReply, PostProcessDkgRequest,
};
use crate::rpc_stub::controller::{DkgTaskReply, MineRequest};
use arpa_node_core::{
    address_to_string, ChainIdentity, DKGTask, Member as ModelMember, MockChainIdentity, Node,
};
use async_trait::async_trait;
use ethers::types::{Address, H256, U256};
use log::{debug, error};
use threshold_bls::group::PairingCurve;
use tonic::{Code, Request};

#[async_trait]
pub trait ControllerMockHelper {
    async fn mine(&self, block_number_increment: usize) -> ContractClientResult<usize>;

    async fn emit_dkg_task(&self) -> ContractClientResult<DKGTask>;
}

pub struct MockControllerClient {
    id_address: Address,
    controller_rpc_endpoint: String,
}

impl MockControllerClient {
    pub fn new(controller_rpc_endpoint: String, id_address: Address) -> Self {
        MockControllerClient {
            id_address,
            controller_rpc_endpoint,
        }
    }
}

impl ControllerClientBuilder for MockChainIdentity {
    type Service = MockControllerClient;

    fn build_controller_client(&self) -> MockControllerClient {
        MockControllerClient::new(
            self.get_provider_rpc_endpoint().to_string(),
            self.get_id_address(),
        )
    }
}

type TransactionsClient = ControllerTransactionsClient<tonic::transport::Channel>;

#[async_trait]
impl ServiceClient<TransactionsClient> for MockControllerClient {
    async fn prepare_service_client(&self) -> ContractClientResult<TransactionsClient> {
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
    async fn prepare_service_client(&self) -> ContractClientResult<ViewsClient> {
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
impl ControllerLogs for MockControllerClient {
    async fn subscribe_dkg_task<
        C: FnMut(DKGTask) -> F + Send,
        F: Future<Output = ContractClientResult<()>> + Send,
    >(
        &self,
        mut cb: C,
    ) -> ContractClientResult<()> {
        loop {
            let task_res = self.emit_dkg_task().await;
            match task_res {
                Ok(task) => cb(task).await?,
                Err(e) => match e {
                    ContractClientError::NoTaskAvailable => debug!("{:?}", e),
                    _ => error!("{:?}", e),
                },
            }
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}

#[async_trait]
impl ControllerTransactions for MockControllerClient {
    async fn node_register(&self, id_public_key: Vec<u8>) -> ContractClientResult<H256> {
        let request = Request::new(NodeRegisterRequest {
            id_address: address_to_string(self.id_address),
            id_public_key,
        });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .node_register(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| {
                let e: ContractClientError = status.into();
                e
            })?;

        Ok(H256::zero())
    }

    async fn commit_dkg(
        &self,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<Address>,
    ) -> ContractClientResult<H256> {
        let disqualified_nodes = disqualified_nodes
            .into_iter()
            .map(address_to_string)
            .collect::<Vec<_>>();

        let request = Request::new(CommitDkgRequest {
            id_address: address_to_string(self.id_address),
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
            .map_err(|status| {
                let e: ContractClientError = status.into();
                e
            })?;

        Ok(H256::zero())
    }

    async fn post_process_dkg(
        &self,
        group_index: usize,
        group_epoch: usize,
    ) -> ContractClientResult<H256> {
        let request = Request::new(PostProcessDkgRequest {
            id_address: address_to_string(self.id_address),
            group_index: group_index as u32,
            group_epoch: group_epoch as u32,
        });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .post_process_dkg(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| {
                let e: ContractClientError = status.into();
                e
            })?;

        Ok(H256::zero())
    }
}

#[async_trait]
impl ControllerMockHelper for MockControllerClient {
    async fn mine(&self, block_number_increment: usize) -> ContractClientResult<usize> {
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

    async fn emit_dkg_task(&self) -> ContractClientResult<DKGTask> {
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
                    .into_keys()
                    .map(|address| address.parse().unwrap())
                    .collect();

                DKGTask {
                    group_index: group_index as usize,
                    epoch: epoch as usize,
                    size: size as usize,
                    threshold: threshold as usize,
                    members,
                    assignment_block_height: assignment_block_height as usize,
                    coordinator_address: coordinator_address.parse().unwrap(),
                }
            })
            .map_err(|status| match status.code() {
                Code::NotFound => ContractClientError::NoTaskAvailable,
                _ => status.into(),
            })
    }
}

#[async_trait]
impl ControllerViews for MockControllerClient {
    async fn get_node(&self, id_address: Address) -> ContractClientResult<Node> {
        let request = Request::new(GetNodeRequest {
            id_address: address_to_string(id_address),
        });

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_node(request)
            .await
            .map(|r| {
                let node_reply = r.into_inner();
                node_reply.into()
            })
            .map_err(|status| status.into())
    }
}

impl From<NodeReply> for Node {
    fn from(r: NodeReply) -> Self {
        Node {
            id_address: r.id_address.parse::<Address>().unwrap(),
            id_public_key: r.id_public_key,
            state: r.state,
            pending_until_block: r.pending_until_block as usize,
            staking: U256::from_big_endian(&r.staking),
        }
    }
}

impl<C: PairingCurve> From<Member> for ModelMember<C> {
    fn from(member: Member) -> Self {
        let partial_public_key = if member.partial_public_key.is_empty() {
            None
        } else {
            Some(bincode::deserialize(&member.partial_public_key).unwrap())
        };

        ModelMember {
            index: member.index as usize,
            id_address: member.id_address.parse().unwrap(),
            rpc_endpoint: None,
            partial_public_key,
        }
    }
}
