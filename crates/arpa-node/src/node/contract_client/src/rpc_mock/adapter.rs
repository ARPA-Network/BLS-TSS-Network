use crate::adapter::{AdapterClientBuilder, AdapterLogs, AdapterTransactions, AdapterViews};
use crate::error::{ContractClientError, ContractClientResult};
use crate::ServiceClient;

use crate::rpc_stub::adapter::{
    transactions_client::TransactionsClient as AdapterTransactionsClient,
    views_client::ViewsClient as AdapterViewsClient, GetGroupRequest, GroupReply, Member,
};
use crate::rpc_stub::adapter::{
    FulfillRandomnessRequest, IsTaskPendingRequest, MineRequest, RequestRandomnessRequest,
    SignatureTaskReply,
};
use arpa_node_core::{
    address_to_string, ChainIdentity, Group, Member as ModelMember, MockChainIdentity,
    RandomnessTask,
};
use async_trait::async_trait;
use ethers::types::{Address, H256, U256};
use log::{debug, error};
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::marker::PhantomData;
use threshold_bls::group::PairingCurve;
use tonic::{Code, Request, Response};

#[async_trait]
pub trait AdapterMockHelper {
    async fn mine(&self, block_number_increment: usize) -> ContractClientResult<usize>;

    async fn emit_signature_task(&self) -> ContractClientResult<RandomnessTask>;
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

impl<C: PairingCurve> AdapterClientBuilder<C> for MockChainIdentity {
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
}

#[async_trait]
impl AdapterTransactions for MockAdapterClient {
    async fn request_randomness(&self, seed: U256) -> ContractClientResult<H256> {
        let mut seed_bytes = vec![0u8; 32];
        seed.to_big_endian(&mut seed_bytes);

        let request = Request::new(RequestRandomnessRequest { seed: seed_bytes });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .request_randomness(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| {
                let e: ContractClientError = status.into();
                e
            })?;

        Ok(H256::zero())
    }

    async fn fulfill_randomness(
        &self,
        group_index: usize,
        request_id: Vec<u8>,
        signature: Vec<u8>,
        partial_signatures: HashMap<Address, Vec<u8>>,
    ) -> ContractClientResult<H256> {
        let partial_signatures: HashMap<String, Vec<u8>> = partial_signatures
            .into_iter()
            .map(|(id_address, sig)| (address_to_string(id_address), sig))
            .collect();

        let request = Request::new(FulfillRandomnessRequest {
            id_address: address_to_string(self.id_address),
            group_index: group_index as u32,
            request_id,
            signature,
            partial_signatures,
        });

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .fulfill_randomness(request)
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
                    request_id,
                    seed,
                    group_index,
                    assignment_block_height,
                } = r.into_inner();

                let seed = U256::from_big_endian(&seed);

                RandomnessTask {
                    request_id,
                    seed,
                    group_index: group_index as usize,
                    request_confirmations: 0,
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
impl<C: PairingCurve> AdapterViews<C> for MockAdapterClient {
    async fn get_group(&self, group_index: usize) -> ContractClientResult<Group<C>> {
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

    async fn get_last_output(&self) -> ContractClientResult<U256> {
        let request = Request::new(());

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_last_output(request)
            .await
            .map(|r| {
                let last_output_reply = r.into_inner();
                U256::from_big_endian(&last_output_reply.last_output)
            })
            .map_err(|status| status.into())
    }

    async fn is_task_pending(&self, request_id: &[u8]) -> ContractClientResult<bool> {
        let request = Request::new(IsTaskPendingRequest {
            request_id: request_id.to_vec(),
        });

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .is_task_pending(request)
            .await
            .map(|r| {
                let reply = r.into_inner();

                reply.state
            })
            .map_err(|status| status.into())
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

fn parse_group_reply<C: PairingCurve>(reply: Response<GroupReply>) -> Group<C> {
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

    let members: BTreeMap<Address, ModelMember<C>> = members
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
        c: PhantomData,
    }
}
