use super::types::Subscriber;
use crate::node::{
    algorithm::bls::{BLSCore, MockBLSCore},
    committer::committer_client::{CommitterService, MockCommitterClient},
    contract_client::controller_client::{ControllerViews, MockControllerClient},
    contract_client::types::Group as ContractGroup,
    dao::cache::{GroupRelayResultCache, InMemoryGroupInfoCache, InMemorySignatureResultCache},
    dao::{
        api::{GroupInfoFetcher, SignatureResultCacheFetcher, SignatureResultCacheUpdater},
        types::{ChainIdentity, GroupRelayTask, TaskType},
    },
    error::errors::NodeResult,
    event::{
        ready_to_handle_group_relay_task::ReadyToHandleGroupRelayTask,
        types::{Event, Topic},
    },
    queue::event_queue::{EventQueue, EventSubscriber},
    scheduler::dynamic::{DynamicTaskScheduler, SimpleDynamicTaskScheduler},
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct ReadyToHandleGroupRelayTaskSubscriber {
    main_chain_identity: Arc<RwLock<ChainIdentity>>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    group_relay_signature_cache: Arc<RwLock<InMemorySignatureResultCache<GroupRelayResultCache>>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl ReadyToHandleGroupRelayTaskSubscriber {
    pub fn new(
        main_chain_identity: Arc<RwLock<ChainIdentity>>,
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
        group_relay_signature_cache: Arc<
            RwLock<InMemorySignatureResultCache<GroupRelayResultCache>>,
        >,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        ReadyToHandleGroupRelayTaskSubscriber {
            main_chain_identity,
            group_cache,
            group_relay_signature_cache,
            eq,
            ts,
        }
    }
}

#[async_trait]
pub trait GroupRelayHandler {
    async fn handle(self, committer_clients: Vec<MockCommitterClient>) -> NodeResult<()>;

    async fn prepare_committer_clients(&self) -> NodeResult<Vec<MockCommitterClient>>;
}

pub struct MockGroupRelayHandler {
    controller_address: String,
    id_address: String,
    tasks: Vec<GroupRelayTask>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    group_relay_signature_cache: Arc<RwLock<InMemorySignatureResultCache<GroupRelayResultCache>>>,
}

#[async_trait]
impl GroupRelayHandler for MockGroupRelayHandler {
    async fn handle(self, mut committer_clients: Vec<MockCommitterClient>) -> NodeResult<()> {
        let mut client =
            MockControllerClient::new(self.controller_address.clone(), self.id_address.clone())
                .await?;

        for task in self.tasks {
            let relayed_group = client.get_group(task.relayed_group_index).await?;

            let relayed_group: ContractGroup = relayed_group.into();

            if relayed_group.epoch != task.relayed_group_epoch {
                continue;
            }

            let relayed_group_as_bytes = bincode::serialize(&relayed_group)?;

            let bls_core = MockBLSCore {};

            let partial_signature = bls_core.partial_sign(
                self.group_cache.read().get_secret_share()?,
                &relayed_group_as_bytes,
            )?;

            let threshold = self.group_cache.read().get_threshold()?;

            let current_group_index = self.group_cache.read().get_index()?;

            if self.group_cache.read().is_committer(&self.id_address)? {
                if !self
                    .group_relay_signature_cache
                    .read()
                    .contains(task.controller_global_epoch)
                {
                    self.group_relay_signature_cache.write().add(
                        current_group_index,
                        task.controller_global_epoch,
                        relayed_group,
                        threshold,
                    )?;
                }

                self.group_relay_signature_cache
                    .write()
                    .add_partial_signature(
                        task.controller_global_epoch,
                        self.id_address.clone(),
                        partial_signature.clone(),
                    )?;
            }

            // TODO retry
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

            for committer in committer_clients.iter_mut() {
                committer
                    .commit_partial_signature(
                        0,
                        TaskType::GroupRelay,
                        relayed_group_as_bytes.clone(),
                        task.controller_global_epoch,
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

impl Subscriber for ReadyToHandleGroupRelayTaskSubscriber {
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        println!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut ReadyToHandleGroupRelayTask;

            let ReadyToHandleGroupRelayTask { tasks } = *Box::from_raw(struct_ptr);

            let controller_address = self
                .main_chain_identity
                .read()
                .get_provider_rpc_endpoint()
                .to_string();

            let id_address = self.main_chain_identity.read().get_id_address().to_string();

            let group_cache_for_handler = self.group_cache.clone();

            let group_relay_signature_cache_for_handler = self.group_relay_signature_cache.clone();

            self.ts.write().add_task(async move {
                let handler = MockGroupRelayHandler {
                    controller_address,
                    id_address,
                    tasks,
                    group_cache: group_cache_for_handler,
                    group_relay_signature_cache: group_relay_signature_cache_for_handler,
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

        let subscriber = Box::new(self);

        eq.write()
            .subscribe(Topic::ReadyToHandleGroupRelayTask, subscriber);
    }
}
