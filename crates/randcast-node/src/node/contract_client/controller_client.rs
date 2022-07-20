use self::controller::{
    transactions_client::TransactionsClient as ControllerTransactionsClient,
    views_client::ViewsClient as ControllerViewsClient, CommitDkgRequest, GetGroupRequest,
    GroupReply, Member, NodeRegisterRequest, PostProcessDkgRequest,
};
use self::controller::{DkgTaskReply, GroupRelayTaskReply, MineRequest};
use self::coordinator::transactions_client::TransactionsClient as CoordinatorTransactionsClient;
use self::coordinator::views_client::ViewsClient as CoordinatorViewsClient;
use self::coordinator::{BlsKeysReply, PublishRequest};
use crate::node::dao::types::{DKGTask, Group, GroupRelayTask, Member as ModelMember};
use crate::node::error::errors::{NodeError, NodeResult};
use async_trait::async_trait;
use dkg_core::{
    primitives::{BundledJustification, BundledResponses, BundledShares},
    BoardPublisher,
};
use std::collections::BTreeMap;
use thiserror::Error;
use threshold_bls::curve::bls12381::Curve;
use tonic::metadata::MetadataValue;
use tonic::{Code, Request};

pub mod controller {
    include!("../../../stub/controller.rs");
}

pub mod coordinator {
    include!("../../../stub/coordinator.rs");
}

#[async_trait]
pub trait CommitterService {
    async fn commit_partial_signature(
        &mut self,
        signature_index: usize,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool>;
}

#[async_trait]
pub trait ControllerTransactions {
    async fn node_register(&mut self, id_public_key: Vec<u8>) -> NodeResult<()>;

    async fn commit_dkg(
        &mut self,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<String>,
    ) -> NodeResult<()>;

    async fn post_process_dkg(&mut self, group_index: usize, group_epoch: usize) -> NodeResult<()>;
}

#[async_trait]
pub trait ControllerMockHelper {
    async fn mine(&mut self, block_number_increment: usize) -> NodeResult<usize>;

    async fn emit_dkg_task(&mut self) -> NodeResult<DKGTask>;

    async fn emit_group_relay_task(&mut self) -> NodeResult<GroupRelayTask>;
}

#[async_trait]
pub trait ControllerViews {
    async fn get_group(&mut self, group_index: usize) -> NodeResult<Group>;
}

#[async_trait]
pub trait CoordinatorTransactions {
    /// Participant publishes their data and depending on the phase the data gets inserted
    /// in the shares, responses or justifications mapping. Reverts if the participant
    /// has already published their data for a phase or if the DKG has ended.
    async fn publish(&mut self, value: Vec<u8>) -> NodeResult<()>;
}

#[async_trait]
pub trait CoordinatorViews {
    // Helpers to fetch data in the mappings. If a participant has registered but not
    // published their data for a phase, the array element at their index is expected to be 0

    /// Gets the participants' shares
    async fn get_shares(&mut self) -> NodeResult<Vec<Vec<u8>>>;

    /// Gets the participants' responses
    async fn get_responses(&mut self) -> NodeResult<Vec<Vec<u8>>>;

    /// Gets the participants' justifications
    async fn get_justifications(&mut self) -> NodeResult<Vec<Vec<u8>>>;

    /// Gets the participants' ethereum addresses
    async fn get_participants(&mut self) -> NodeResult<Vec<String>>;

    /// Gets the participants' BLS keys along with the thershold of the DKG
    async fn get_bls_keys(&mut self) -> NodeResult<(usize, Vec<Vec<u8>>)>;

    /// Returns the current phase of the DKG.
    async fn in_phase(&mut self) -> NodeResult<usize>;
}

pub struct MockControllerClient {
    id_address: String,
    transactions_client: ControllerTransactionsClient<tonic::transport::Channel>,
    views_client: ControllerViewsClient<tonic::transport::Channel>,
}

impl MockControllerClient {
    pub async fn new(
        controller_rpc_endpoint: String,
        id_address: String,
    ) -> NodeResult<MockControllerClient> {
        let transactions_client: ControllerTransactionsClient<tonic::transport::Channel> =
            ControllerTransactionsClient::connect(format!(
                "{}{}",
                "http://",
                controller_rpc_endpoint.clone()
            ))
            .await?;

        let views_client: ControllerViewsClient<tonic::transport::Channel> =
            ControllerViewsClient::connect(format!("{}{}", "http://", controller_rpc_endpoint))
                .await?;

        Ok(MockControllerClient {
            id_address,
            transactions_client,
            views_client,
        })
    }
}

#[async_trait]
impl ControllerTransactions for MockControllerClient {
    async fn node_register(&mut self, id_public_key: Vec<u8>) -> NodeResult<()> {
        let request = Request::new(NodeRegisterRequest {
            id_address: self.id_address.to_string(),
            id_public_key,
        });

        self.transactions_client
            .node_register(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn commit_dkg(
        &mut self,
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

        self.transactions_client
            .commit_dkg(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn post_process_dkg(&mut self, group_index: usize, group_epoch: usize) -> NodeResult<()> {
        let request = Request::new(PostProcessDkgRequest {
            id_address: self.id_address.to_string(),
            group_index: group_index as u32,
            group_epoch: group_epoch as u32,
        });

        self.transactions_client
            .post_process_dkg(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }
}

#[async_trait]
impl ControllerMockHelper for MockControllerClient {
    async fn mine(&mut self, block_number_increment: usize) -> NodeResult<usize> {
        let request = Request::new(MineRequest {
            block_number_increment: block_number_increment as u32,
        });

        self.transactions_client
            .mine(request)
            .await
            .map(|r| r.into_inner().block_number as usize)
            .map_err(|status| status.into())
    }

    async fn emit_dkg_task(&mut self) -> NodeResult<DKGTask> {
        let request = Request::new(());

        self.views_client
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

    async fn emit_group_relay_task(&mut self) -> NodeResult<GroupRelayTask> {
        let request = Request::new(());
        self.views_client
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
    async fn get_group(&mut self, group_index: usize) -> NodeResult<Group> {
        let request = Request::new(GetGroupRequest {
            index: group_index as u32,
        });

        self.views_client
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

pub struct MockCoordinatorClient {
    id_address: String,
    index: usize,
    epoch: usize,
    transactions_client: CoordinatorTransactionsClient<tonic::transport::Channel>,
    views_client: CoordinatorViewsClient<tonic::transport::Channel>,
}

impl MockCoordinatorClient {
    pub async fn new(
        coordinator_address: String,
        id_address: String,
        index: usize,
        epoch: usize,
    ) -> NodeResult<MockCoordinatorClient> {
        let transactions_client: CoordinatorTransactionsClient<tonic::transport::Channel> =
            CoordinatorTransactionsClient::connect(format!(
                "{}{}",
                "http://",
                coordinator_address.clone()
            ))
            .await?;

        let views_client: CoordinatorViewsClient<tonic::transport::Channel> =
            CoordinatorViewsClient::connect(format!("{}{}", "http://", coordinator_address))
                .await?;

        Ok(MockCoordinatorClient {
            id_address,
            index,
            epoch,
            transactions_client,
            views_client,
        })
    }

    fn set_metadata<T>(&self, req: &mut Request<T>) {
        req.metadata_mut().insert(
            "index",
            MetadataValue::from_str(&self.index.to_string()).unwrap(),
        );

        req.metadata_mut().insert(
            "epoch",
            MetadataValue::from_str(&self.epoch.to_string()).unwrap(),
        );
    }
}

#[async_trait]
impl CoordinatorTransactions for MockCoordinatorClient {
    async fn publish(&mut self, value: Vec<u8>) -> NodeResult<()> {
        let mut request = Request::new(PublishRequest {
            id_address: self.id_address.to_string(),
            value,
        });

        self.set_metadata(&mut request);

        self.transactions_client
            .publish(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }
}

#[async_trait]
impl CoordinatorViews for MockCoordinatorClient {
    async fn get_shares(&mut self) -> NodeResult<Vec<Vec<u8>>> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        self.views_client
            .get_shares(request)
            .await
            .map(|r| r.into_inner().shares)
            .map_err(|status| status.into())
    }

    async fn get_responses(&mut self) -> NodeResult<Vec<Vec<u8>>> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        self.views_client
            .get_responses(request)
            .await
            .map(|r| r.into_inner().responses)
            .map_err(|status| status.into())
    }

    async fn get_justifications(&mut self) -> NodeResult<Vec<Vec<u8>>> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        self.views_client
            .get_justifications(request)
            .await
            .map(|r| r.into_inner().justifications)
            .map_err(|status| status.into())
    }

    async fn get_participants(&mut self) -> NodeResult<Vec<String>> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        self.views_client
            .get_participants(request)
            .await
            .map(|r| r.into_inner().participants)
            .map_err(|status| status.into())
    }

    async fn get_bls_keys(&mut self) -> NodeResult<(usize, Vec<Vec<u8>>)> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        self.views_client
            .get_bls_keys(request)
            .await
            .map(|r| {
                let BlsKeysReply {
                    threshold,
                    bls_keys,
                } = r.into_inner();
                (threshold as usize, bls_keys)
            })
            .map_err(|status| status.into())
    }

    async fn in_phase(&mut self) -> NodeResult<usize> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        self.views_client
            .in_phase(request)
            .await
            .map(|r| r.into_inner().phase as usize)
            .map_err(|status| status.into())
    }
}

#[derive(Debug, Error)]
pub enum DKGContractError {
    #[error(transparent)]
    SerializationError(#[from] bincode::Error),
    #[error(transparent)]
    PublishingError(#[from] NodeError),
}

#[async_trait]
impl BoardPublisher<Curve> for MockCoordinatorClient {
    type Error = DKGContractError;

    async fn publish_shares(&mut self, shares: BundledShares<Curve>) -> Result<(), Self::Error> {
        println!("called publish_shares");
        let serialized = bincode::serialize(&shares)?;
        self.publish(serialized).await.map_err(|e| e.into())
    }

    async fn publish_responses(&mut self, responses: BundledResponses) -> Result<(), Self::Error> {
        println!("called publish_responses");
        let serialized = bincode::serialize(&responses)?;
        self.publish(serialized).await.map_err(|e| e.into())
    }

    async fn publish_justifications(
        &mut self,
        justifications: BundledJustification<Curve>,
    ) -> Result<(), Self::Error> {
        let serialized = bincode::serialize(&justifications)?;
        self.publish(serialized).await.map_err(|e| e.into())
    }
}
