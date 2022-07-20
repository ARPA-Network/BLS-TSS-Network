use super::types::Subscriber;
use crate::node::{
    algorithm::bls::{BLSCore, MockBLSCore},
    committer::committer_client::{CommitterService, MockCommitterClient},
    contract_client::types::Group as ContractGroup,
    contract_client::{
        adapter_client::{AdapterViews, MockAdapterClient},
        controller_client::{ControllerViews, MockControllerClient},
    },
    dao::types::{GroupRelayConfirmation, GroupRelayConfirmationTask, Status, TaskType},
    dao::{
        api::{GroupInfoFetcher, SignatureResultCacheFetcher, SignatureResultCacheUpdater},
        cache::{
            GroupRelayConfirmationResultCache, InMemoryGroupInfoCache, InMemorySignatureResultCache,
        },
        types::ChainIdentity,
    },
    error::errors::NodeResult,
    event::{
        ready_to_handle_group_relay_confirmation_task::ReadyToHandleGroupRelayConfirmationTask,
        types::{Event, Topic},
    },
    queue::event_queue::{EventQueue, EventSubscriber},
    scheduler::dynamic::{DynamicTaskScheduler, SimpleDynamicTaskScheduler},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct ReadyToHandleGroupRelayConfirmationTaskSubscriber {
    pub chain_id: usize,
    main_chain_identity: Arc<RwLock<ChainIdentity>>,
    chain_identity: Arc<RwLock<ChainIdentity>>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    group_relay_confirmation_signature_cache:
        Arc<RwLock<InMemorySignatureResultCache<GroupRelayConfirmationResultCache>>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl ReadyToHandleGroupRelayConfirmationTaskSubscriber {
    pub fn new(
        chain_id: usize,
        main_chain_identity: Arc<RwLock<ChainIdentity>>,
        chain_identity: Arc<RwLock<ChainIdentity>>,
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
        group_relay_confirmation_signature_cache: Arc<
            RwLock<InMemorySignatureResultCache<GroupRelayConfirmationResultCache>>,
        >,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        ReadyToHandleGroupRelayConfirmationTaskSubscriber {
            chain_id,
            main_chain_identity,
            chain_identity,
            group_cache,
            group_relay_confirmation_signature_cache,
            eq,
            ts,
        }
    }
}

#[async_trait]
pub trait GroupRelayConfirmationHandler {
    async fn handle(self, committer_clients: Vec<MockCommitterClient>) -> NodeResult<()>;

    async fn prepare_committer_clients(&self) -> NodeResult<Vec<MockCommitterClient>>;
}

pub struct MockGroupRelayConfirmationHandler {
    chain_id: usize,
    controller_address: String,
    adapter_address: String,
    id_address: String,
    tasks: Vec<GroupRelayConfirmationTask>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    group_relay_confirmation_signature_cache:
        Arc<RwLock<InMemorySignatureResultCache<GroupRelayConfirmationResultCache>>>,
}

#[async_trait]
impl GroupRelayConfirmationHandler for MockGroupRelayConfirmationHandler {
    async fn handle(self, mut committer_clients: Vec<MockCommitterClient>) -> NodeResult<()> {
        let mut controller_client =
            MockControllerClient::new(self.controller_address.clone(), self.id_address.clone())
                .await?;

        let mut adapter_client =
            MockAdapterClient::new(self.adapter_address.clone(), self.id_address.clone()).await?;

        for task in self.tasks {
            let relayed_group = controller_client
                .get_group(task.relayed_group_index)
                .await?;

            println!("get group from controller success.");

            let relayed_group: ContractGroup = relayed_group.into();

            let relayed_group_as_bytes = bincode::serialize(&relayed_group)?;

            let relayed_group_cache = adapter_client
                .get_group_relay_cache(task.group_relay_cache_index)
                .await?;

            let relayed_group_cache: ContractGroup = relayed_group_cache.into();

            let relayed_group_cache_as_bytes = bincode::serialize(&relayed_group_cache)?;

            let status = Status::from(relayed_group_as_bytes == relayed_group_cache_as_bytes);

            let group_relay_confirmation = GroupRelayConfirmation {
                group: relayed_group_cache,
                status,
            };

            println!("group_relay_confirmation: {:?}", group_relay_confirmation);

            let group_relay_confirmation_as_bytes = bincode::serialize(&group_relay_confirmation)?;

            let bls_core = MockBLSCore {};

            let partial_signature = bls_core.partial_sign(
                self.group_cache.read().get_secret_share()?,
                &group_relay_confirmation_as_bytes,
            )?;

            let threshold = self.group_cache.read().get_threshold()?;

            let current_group_index = self.group_cache.read().get_index()?;

            if self.group_cache.read().is_committer(&self.id_address)? {
                if !self
                    .group_relay_confirmation_signature_cache
                    .read()
                    .contains(task.index)
                {
                    self.group_relay_confirmation_signature_cache.write().add(
                        current_group_index,
                        task.index,
                        group_relay_confirmation,
                        threshold,
                    )?;
                }

                self.group_relay_confirmation_signature_cache
                    .write()
                    .add_partial_signature(
                        task.index,
                        self.id_address.clone(),
                        partial_signature.clone(),
                    )?;
            }

            // TODO retry
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

            for committer in committer_clients.iter_mut() {
                committer
                    .commit_partial_signature(
                        self.chain_id,
                        TaskType::GroupRelayConfirmation,
                        group_relay_confirmation_as_bytes.clone(),
                        task.index,
                        partial_signature.clone(),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    async fn prepare_committer_clients(&self) -> NodeResult<Vec<MockCommitterClient>> {
        let mut committers = self
            .group_cache
            .read()
            .get_committers()?
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>();

        committers.retain(|c| *c != self.id_address);

        let mut committer_clients = vec![];

        for committer in committers {
            let endpoint = self
                .group_cache
                .read()
                .get_member(&committer)?
                .rpc_endpint
                .as_ref()
                .unwrap()
                .to_string();

            // we retry some times here as building tonic connection needs the target rpc server available
            let mut i = 0;
            while i < 3 {
                if let Ok(committer_client) =
                    MockCommitterClient::new(self.id_address.clone(), endpoint.clone()).await
                {
                    committer_clients.push(committer_client);
                    break;
                }
                i += 1;
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            }
        }

        Ok(committer_clients)
    }
}

impl Subscriber for ReadyToHandleGroupRelayConfirmationTaskSubscriber {
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        println!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut ReadyToHandleGroupRelayConfirmationTask;

            let ReadyToHandleGroupRelayConfirmationTask { chain_id: _, tasks } =
                *Box::from_raw(struct_ptr);

            let chain_id = self.chain_identity.read().get_id();

            let controller_address = self
                .main_chain_identity
                .read()
                .get_provider_rpc_endpoint()
                .to_string();

            let adapter_address = self
                .chain_identity
                .read()
                .get_provider_rpc_endpoint()
                .to_string();

            let id_address = self.main_chain_identity.read().get_id_address().to_string();

            let group_cache_for_handler = self.group_cache.clone();

            let group_relay_confirmation_signature_cache_for_handler =
                self.group_relay_confirmation_signature_cache.clone();

            self.ts.write().add_task(async move {
                let handler = MockGroupRelayConfirmationHandler {
                    chain_id,
                    controller_address,
                    adapter_address,
                    id_address,
                    tasks,
                    group_cache: group_cache_for_handler,
                    group_relay_confirmation_signature_cache:
                        group_relay_confirmation_signature_cache_for_handler,
                };

                if let Ok(committer_clients) = handler.prepare_committer_clients().await {
                    if let Err(e) = handler.handle(committer_clients).await {
                        println!("{:?}", e);
                    }
                }
            });
        }

        Ok(())
    }

    fn subscribe(self) {
        let eq = self.eq.clone();

        let chain_id = self.chain_id;

        let subscriber = Box::new(self);

        eq.write().subscribe(
            Topic::ReadyToHandleGroupRelayConfirmationTask(chain_id),
            subscriber,
        );
    }
}
