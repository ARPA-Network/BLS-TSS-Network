use std::collections::{BTreeMap, HashMap};

use self::adapter::{
    transactions_client::TransactionsClient as AdapterTransactionsClient,
    views_client::ViewsClient as AdapterViewsClient, GetGroupRequest, GroupReply, Member,
};
use self::adapter::{
    CancelInvalidRelayConfirmationTaskRequest, ConfirmRelayRequest, FulfillRandomnessRequest,
    FulfillRelayRequest, GetGroupRelayCacheRequest, GetGroupRelayConfirmationTaskStateRequest,
    GetSignatureTaskCompletionStateRequest, GroupRelayConfirmationTaskReply, MineRequest,
    RequestRandomnessRequest, SetInitialGroupRequest, SignatureTaskReply,
};
use async_trait::async_trait;
use tonic::{Code, Request, Response};

use crate::node::dao::types::{
    Group, GroupRelayConfirmationTask, GroupRelayConfirmationTaskState, Member as ModelMember,
    RandomnessTask,
};
use crate::node::error::errors::{NodeError, NodeResult};

pub mod adapter {
    include!("../../../stub/adapter.rs");
}

#[async_trait]
pub trait AdapterTransactions {
    async fn request_randomness(&mut self, message: &str) -> NodeResult<()>;

    async fn fulfill_randomness(
        &mut self,
        group_index: usize,
        signature_index: usize,
        signature: Vec<u8>,
        partial_signatures: HashMap<String, Vec<u8>>,
    ) -> NodeResult<()>;

    async fn fulfill_relay(
        &mut self,
        relayer_group_index: usize,
        task_index: usize,
        signature: Vec<u8>,
        group_as_bytes: Vec<u8>,
    ) -> NodeResult<()>;

    async fn cancel_invalid_relay_confirmation_task(&mut self, task_index: usize)
        -> NodeResult<()>;

    async fn confirm_relay(
        &mut self,
        task_index: usize,
        group_relay_confirmation_as_bytes: Vec<u8>,
        signature: Vec<u8>,
    ) -> NodeResult<()>;

    async fn set_initial_group(&mut self, group: Vec<u8>) -> NodeResult<()>;
}

#[async_trait]
pub trait AdapterMockHelper {
    async fn mine(&mut self, block_number_increment: usize) -> NodeResult<usize>;

    async fn emit_signature_task(&mut self) -> NodeResult<RandomnessTask>;

    async fn emit_group_relay_confirmation_task(
        &mut self,
    ) -> NodeResult<GroupRelayConfirmationTask>;
}

#[async_trait]
pub trait AdapterViews {
    async fn get_group(&mut self, group_index: usize) -> NodeResult<Group>;

    async fn get_last_output(&mut self) -> NodeResult<u64>;

    async fn get_signature_task_completion_state(&mut self, index: usize) -> NodeResult<bool>;

    async fn get_group_relay_cache(&mut self, group_index: usize) -> NodeResult<Group>;

    async fn get_group_relay_confirmation_task_state(
        &mut self,
        task_index: usize,
    ) -> NodeResult<GroupRelayConfirmationTaskState>;
}

pub struct MockAdapterClient {
    id_address: String,
    transactions_client: AdapterTransactionsClient<tonic::transport::Channel>,
    views_client: AdapterViewsClient<tonic::transport::Channel>,
}

impl MockAdapterClient {
    pub async fn new(
        adapter_rpc_endpoint: String,
        id_address: String,
    ) -> NodeResult<MockAdapterClient> {
        let transactions_client: AdapterTransactionsClient<tonic::transport::Channel> =
            AdapterTransactionsClient::connect(format!(
                "{}{}",
                "http://",
                adapter_rpc_endpoint.clone()
            ))
            .await?;

        let views_client: AdapterViewsClient<tonic::transport::Channel> =
            AdapterViewsClient::connect(format!("{}{}", "http://", adapter_rpc_endpoint)).await?;

        Ok(MockAdapterClient {
            id_address,
            transactions_client,
            views_client,
        })
    }
}

#[async_trait]
impl AdapterTransactions for MockAdapterClient {
    async fn request_randomness(&mut self, message: &str) -> NodeResult<()> {
        let request = Request::new(RequestRandomnessRequest {
            message: message.to_string(),
        });

        self.transactions_client
            .request_randomness(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn fulfill_randomness(
        &mut self,
        group_index: usize,
        signature_index: usize,
        signature: Vec<u8>,
        partial_signatures: HashMap<String, Vec<u8>>,
    ) -> NodeResult<()> {
        let request = Request::new(FulfillRandomnessRequest {
            id_address: self.id_address.to_string(),
            group_index: group_index as u32,
            signature_index: signature_index as u32,
            signature,
            partial_signatures,
        });

        self.transactions_client
            .fulfill_randomness(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn fulfill_relay(
        &mut self,
        relayer_group_index: usize,
        task_index: usize,
        signature: Vec<u8>,
        group_as_bytes: Vec<u8>,
    ) -> NodeResult<()> {
        let request = Request::new(FulfillRelayRequest {
            id_address: self.id_address.to_string(),
            relayer_group_index: relayer_group_index as u32,
            task_index: task_index as u32,
            signature,
            group_as_bytes,
        });

        self.transactions_client
            .fulfill_relay(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn cancel_invalid_relay_confirmation_task(
        &mut self,
        task_index: usize,
    ) -> NodeResult<()> {
        let request = Request::new(CancelInvalidRelayConfirmationTaskRequest {
            id_address: self.id_address.to_string(),
            task_index: task_index as u32,
        });

        self.transactions_client
            .cancel_invalid_relay_confirmation_task(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn confirm_relay(
        &mut self,
        task_index: usize,
        group_relay_confirmation_as_bytes: Vec<u8>,
        signature: Vec<u8>,
    ) -> NodeResult<()> {
        let request = Request::new(ConfirmRelayRequest {
            id_address: self.id_address.to_string(),
            task_index: task_index as u32,
            signature,
            group_relay_confirmation_as_bytes,
        });

        self.transactions_client
            .confirm_relay(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn set_initial_group(&mut self, group: Vec<u8>) -> NodeResult<()> {
        let request = Request::new(SetInitialGroupRequest {
            id_address: self.id_address.to_string(),
            group,
        });

        self.transactions_client
            .set_initial_group(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }
}

#[async_trait]
impl AdapterMockHelper for MockAdapterClient {
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

    async fn emit_signature_task(&mut self) -> NodeResult<RandomnessTask> {
        let request = Request::new(());
        self.views_client
            .emit_signature_task(request)
            .await
            .map(|r| {
                let SignatureTaskReply {
                    index,
                    message,
                    group_index,
                    assignment_block_height,
                } = r.into_inner();

                RandomnessTask {
                    index: index as usize,
                    message,
                    group_index: group_index as usize,
                    assignment_block_height: assignment_block_height as usize,
                }
            })
            .map_err(|status| match status.code() {
                Code::NotFound => NodeError::NoTaskAvailable,
                _ => status.into(),
            })
    }

    async fn emit_group_relay_confirmation_task(
        &mut self,
    ) -> NodeResult<GroupRelayConfirmationTask> {
        let request = Request::new(());
        self.views_client
            .emit_group_relay_confirmation_task(request)
            .await
            .map(|r| {
                let GroupRelayConfirmationTaskReply {
                    index,
                    group_relay_cache_index,
                    relayed_group_index,
                    relayed_group_epoch,
                    relayer_group_index,
                    assignment_block_height,
                } = r.into_inner();

                GroupRelayConfirmationTask {
                    index: index as usize,
                    group_relay_cache_index: group_relay_cache_index as usize,
                    relayed_group_index: relayed_group_index as usize,
                    relayed_group_epoch: relayed_group_epoch as usize,
                    relayer_group_index: relayer_group_index as usize,
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
impl AdapterViews for MockAdapterClient {
    async fn get_group(&mut self, group_index: usize) -> NodeResult<Group> {
        let request = Request::new(GetGroupRequest {
            index: group_index as u32,
        });

        self.views_client
            .get_group(request)
            .await
            .map(parse_group_reply)
            .map_err(|status| status.into())
    }

    async fn get_last_output(&mut self) -> NodeResult<u64> {
        let request = Request::new(());

        self.views_client
            .get_last_output(request)
            .await
            .map(|r| {
                let last_output_reply = r.into_inner();

                last_output_reply.last_output
            })
            .map_err(|status| status.into())
    }

    async fn get_signature_task_completion_state(&mut self, index: usize) -> NodeResult<bool> {
        let request = Request::new(GetSignatureTaskCompletionStateRequest {
            index: index as u32,
        });

        self.views_client
            .get_signature_task_completion_state(request)
            .await
            .map(|r| {
                let reply = r.into_inner();

                reply.state
            })
            .map_err(|status| status.into())
    }

    async fn get_group_relay_cache(&mut self, group_index: usize) -> NodeResult<Group> {
        let request = Request::new(GetGroupRelayCacheRequest {
            index: group_index as u32,
        });

        self.views_client
            .get_group_relay_cache(request)
            .await
            .map(parse_group_reply)
            .map_err(|status| status.into())
    }

    async fn get_group_relay_confirmation_task_state(
        &mut self,
        task_index: usize,
    ) -> NodeResult<GroupRelayConfirmationTaskState> {
        let request = Request::new(GetGroupRelayConfirmationTaskStateRequest {
            index: task_index as u32,
        });

        self.views_client
            .get_group_relay_confirmation_task_state(request)
            .await
            .map(|r| {
                let reply = r.into_inner();
                GroupRelayConfirmationTaskState::from(reply.state)
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

fn parse_group_reply(reply: Response<GroupReply>) -> Group {
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
    } = reply.into_inner();

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
}
