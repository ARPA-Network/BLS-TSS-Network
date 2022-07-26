use crate::adapter::{AdapterClientBuilder, AdapterLogs, AdapterTransactions, AdapterViews};
use crate::error::{ContractClientError, ContractClientResult};
use crate::ServiceClient;

use self::adapter_stub::{
    transactions_client::TransactionsClient as AdapterTransactionsClient,
    views_client::ViewsClient as AdapterViewsClient, GetGroupRequest, GroupReply, Member,
};
use self::adapter_stub::{
    CancelInvalidRelayConfirmationTaskRequest, ConfirmRelayRequest, FulfillRandomnessRequest,
    FulfillRelayRequest, GetGroupRelayCacheRequest, GetGroupRelayConfirmationTaskStateRequest,
    GetSignatureTaskCompletionStateRequest, GroupRelayConfirmationTaskReply, MineRequest,
    RequestRandomnessRequest, SetInitialGroupRequest, SignatureTaskReply,
};
use arpa_node_core::{
    address_to_string, ChainIdentity, Group, GroupRelayConfirmationTask,
    GroupRelayConfirmationTaskState, Member as ModelMember, MockChainIdentity, RandomnessTask,
};
use async_trait::async_trait;
use ethers::types::Address;
use log::{debug, error};
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use tonic::{Code, Request, Response};

pub mod adapter_stub {
    include!("../../../../../rpc_stub/adapter.rs");
}

#[async_trait]
pub trait AdapterMockHelper {
    async fn mine(&self, block_number_increment: usize) -> ContractClientResult<usize>;

    async fn emit_signature_task(&self) -> ContractClientResult<RandomnessTask>;

    async fn emit_group_relay_confirmation_task(
        &self,
    ) -> ContractClientResult<GroupRelayConfirmationTask>;
}

pub struct MockAdapterClient {
    id_address: Address,
    adapter_rpc_endpoint: String,
}

impl MockAdapterClient {
    pub fn new(adapter_rpc_endpoint: String, id_address: Address) -> Self {
        MockAdapterClient {
            id_address,
            adapter_rpc_endpoint,
        }
    }
}

impl AdapterClientBuilder for MockChainIdentity {
    type Service = MockAdapterClient;

    fn build_adapter_client(&self, main_id_address: Address) -> MockAdapterClient {
        MockAdapterClient::new(
            self.get_provider_rpc_endpoint().to_string(),
            main_id_address,
        )
    }
}

type TransactionsClient = AdapterTransactionsClient<tonic::transport::Channel>;

#[async_trait]
impl ServiceClient<TransactionsClient> for MockAdapterClient {
    async fn prepare_service_client(&self) -> ContractClientResult<TransactionsClient> {
        TransactionsClient::connect(format!(
            "{}{}",
            "http://",
            self.adapter_rpc_endpoint.clone()
        ))
        .await
        .map_err(|err| err.into())
    }
}

type ViewsClient = AdapterViewsClient<tonic::transport::Channel>;

#[async_trait]
impl ServiceClient<ViewsClient> for MockAdapterClient {
    async fn prepare_service_client(&self) -> ContractClientResult<ViewsClient> {
        ViewsClient::connect(format!(
            "{}{}",
            "http://",
            self.adapter_rpc_endpoint.clone()
        ))
        .await
        .map_err(|err| err.into())
    }
}

#[async_trait]
impl AdapterLogs for MockAdapterClient {
    async fn subscribe_randomness_task<
        C: FnMut(RandomnessTask) -> F + Send,
        F: Future<Output = ContractClientResult<()>> + Send,
    >(
        &self,
        mut cb: C,
    ) -> ContractClientResult<()> {
        loop {
            let task_res = self.emit_signature_task().await;
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

    async fn subscribe_group_relay_confirmation_task<
        C: FnMut(GroupRelayConfirmationTask) -> F + Send,
        F: Future<Output = ContractClientResult<()>> + Send,
    >(
        &self,
        mut cb: C,
    ) -> ContractClientResult<()> {
        loop {
            let task_res = self.emit_group_relay_confirmation_task().await;
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
impl AdapterTransactions for MockAdapterClient {
    async fn request_randomness(&self, message: &str) -> ContractClientResult<()> {
        let request = Request::new(RequestRandomnessRequest {
            message: message.to_string(),
        });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .request_randomness(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn fulfill_randomness(
        &self,
        group_index: usize,
        signature_index: usize,
        signature: Vec<u8>,
        partial_signatures: HashMap<Address, Vec<u8>>,
    ) -> ContractClientResult<()> {
        let partial_signatures: HashMap<String, Vec<u8>> = partial_signatures
            .into_iter()
            .map(|(id_address, sig)| (address_to_string(id_address), sig))
            .collect();

        let request = Request::new(FulfillRandomnessRequest {
            id_address: address_to_string(self.id_address),
            group_index: group_index as u32,
            signature_index: signature_index as u32,
            signature,
            partial_signatures,
        });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .fulfill_randomness(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn fulfill_relay(
        &self,
        relayer_group_index: usize,
        task_index: usize,
        signature: Vec<u8>,
        group_as_bytes: Vec<u8>,
    ) -> ContractClientResult<()> {
        let request = Request::new(FulfillRelayRequest {
            id_address: address_to_string(self.id_address),
            relayer_group_index: relayer_group_index as u32,
            task_index: task_index as u32,
            signature,
            group_as_bytes,
        });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .fulfill_relay(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn cancel_invalid_relay_confirmation_task(
        &self,
        task_index: usize,
    ) -> ContractClientResult<()> {
        let request = Request::new(CancelInvalidRelayConfirmationTaskRequest {
            id_address: address_to_string(self.id_address),
            task_index: task_index as u32,
        });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .cancel_invalid_relay_confirmation_task(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn confirm_relay(
        &self,
        task_index: usize,
        group_relay_confirmation_as_bytes: Vec<u8>,
        signature: Vec<u8>,
    ) -> ContractClientResult<()> {
        let request = Request::new(ConfirmRelayRequest {
            id_address: address_to_string(self.id_address),
            task_index: task_index as u32,
            signature,
            group_relay_confirmation_as_bytes,
        });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .confirm_relay(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }

    async fn set_initial_group(&self, group: Vec<u8>) -> ContractClientResult<()> {
        let request = Request::new(SetInitialGroupRequest {
            id_address: address_to_string(self.id_address),
            group,
        });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .set_initial_group(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }
}

#[async_trait]
impl AdapterMockHelper for MockAdapterClient {
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

    async fn emit_signature_task(&self) -> ContractClientResult<RandomnessTask> {
        let request = Request::new(());

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
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
                Code::NotFound => ContractClientError::NoTaskAvailable,
                _ => status.into(),
            })
    }

    async fn emit_group_relay_confirmation_task(
        &self,
    ) -> ContractClientResult<GroupRelayConfirmationTask> {
        let request = Request::new(());

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
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
                Code::NotFound => ContractClientError::NoTaskAvailable,
                _ => status.into(),
            })
    }
}

#[async_trait]
impl AdapterViews for MockAdapterClient {
    async fn get_group(&self, group_index: usize) -> ContractClientResult<Group> {
        let request = Request::new(GetGroupRequest {
            index: group_index as u32,
        });

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_group(request)
            .await
            .map(parse_group_reply)
            .map_err(|status| status.into())
    }

    async fn get_last_output(&self) -> ContractClientResult<u64> {
        let request = Request::new(());

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_last_output(request)
            .await
            .map(|r| {
                let last_output_reply = r.into_inner();

                last_output_reply.last_output
            })
            .map_err(|status| status.into())
    }

    async fn get_signature_task_completion_state(
        &self,
        index: usize,
    ) -> ContractClientResult<bool> {
        let request = Request::new(GetSignatureTaskCompletionStateRequest {
            index: index as u32,
        });

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_signature_task_completion_state(request)
            .await
            .map(|r| {
                let reply = r.into_inner();

                reply.state
            })
            .map_err(|status| status.into())
    }

    async fn get_group_relay_cache(&self, group_index: usize) -> ContractClientResult<Group> {
        let request = Request::new(GetGroupRelayCacheRequest {
            index: group_index as u32,
        });

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_group_relay_cache(request)
            .await
            .map(parse_group_reply)
            .map_err(|status| status.into())
    }

    async fn get_group_relay_confirmation_task_state(
        &self,
        task_index: usize,
    ) -> ContractClientResult<GroupRelayConfirmationTaskState> {
        let request = Request::new(GetGroupRelayConfirmationTaskStateRequest {
            index: task_index as u32,
        });

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
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
            id_address: member.id_address.parse().unwrap(),
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

    let members: BTreeMap<Address, ModelMember> = members
        .into_iter()
        .map(|(id_address, m)| (id_address.parse().unwrap(), m.into()))
        .collect();

    let committers = committers.into_iter().map(|c| c.parse().unwrap()).collect();

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
