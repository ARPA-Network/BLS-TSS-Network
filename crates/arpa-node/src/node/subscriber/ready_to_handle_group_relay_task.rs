use super::Subscriber;
use crate::node::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    committer::{
        client::GeneralCommitterClient, CommitterClient, CommitterClientHandler, CommitterService,
    },
    error::{NodeError, NodeResult},
    event::{ready_to_handle_group_relay_task::ReadyToHandleGroupRelayTask, types::Topic, Event},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use arpa_node_contract_client::adapter::{AdapterClientBuilder, AdapterViews};
use arpa_node_core::{ChainIdentity, ContractGroup, GroupRelayTask, TaskType};
use arpa_node_dal::{
    cache::GroupRelayResultCache, GroupInfoFetcher, SignatureResultCacheFetcher,
    SignatureResultCacheUpdater,
};
use async_trait::async_trait;
use ethers::types::Address;
use log::{debug, error};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct ReadyToHandleGroupRelayTaskSubscriber<
    G: GroupInfoFetcher,
    I: ChainIdentity + AdapterClientBuilder,
    C: SignatureResultCacheUpdater<GroupRelayResultCache>
        + SignatureResultCacheFetcher<GroupRelayResultCache>,
> {
    main_chain_identity: Arc<RwLock<I>>,
    group_cache: Arc<RwLock<G>>,
    group_relay_signature_cache: Arc<RwLock<C>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl<
        G: GroupInfoFetcher,
        I: ChainIdentity + AdapterClientBuilder,
        C: SignatureResultCacheUpdater<GroupRelayResultCache>
            + SignatureResultCacheFetcher<GroupRelayResultCache>,
    > ReadyToHandleGroupRelayTaskSubscriber<G, I, C>
{
    pub fn new(
        main_chain_identity: Arc<RwLock<I>>,
        group_cache: Arc<RwLock<G>>,
        group_relay_signature_cache: Arc<RwLock<C>>,
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
    async fn handle(self) -> NodeResult<()>;
}

pub struct GeneralGroupRelayHandler<
    G: GroupInfoFetcher,
    I: ChainIdentity + AdapterClientBuilder,
    C: SignatureResultCacheUpdater<GroupRelayResultCache>
        + SignatureResultCacheFetcher<GroupRelayResultCache>,
> {
    main_chain_identity: Arc<RwLock<I>>,
    tasks: Vec<GroupRelayTask>,
    group_cache: Arc<RwLock<G>>,
    group_relay_signature_cache: Arc<RwLock<C>>,
}

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send,
        I: ChainIdentity + AdapterClientBuilder + Sync + Send,
        C: SignatureResultCacheUpdater<GroupRelayResultCache>
            + SignatureResultCacheFetcher<GroupRelayResultCache>
            + Sync
            + Send,
    > CommitterClientHandler<GeneralCommitterClient, G> for GeneralGroupRelayHandler<G, I, C>
{
    async fn get_id_address(&self) -> Address {
        self.main_chain_identity.read().await.get_id_address()
    }

    fn get_group_cache(&self) -> Arc<RwLock<G>> {
        self.group_cache.clone()
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send,
        I: ChainIdentity + AdapterClientBuilder + Sync + Send,
        C: SignatureResultCacheUpdater<GroupRelayResultCache>
            + SignatureResultCacheFetcher<GroupRelayResultCache>
            + Sync
            + Send,
    > GroupRelayHandler for GeneralGroupRelayHandler<G, I, C>
{
    async fn handle(self) -> NodeResult<()> {
        let main_id_address = self.main_chain_identity.read().await.get_id_address();
        let client = self
            .main_chain_identity
            .read()
            .await
            .build_adapter_client(main_id_address);

        let committers = self.prepare_committer_clients().await?;

        for task in self.tasks {
            let relayed_group = client.get_group(task.relayed_group_index).await?;

            let relayed_group: ContractGroup = relayed_group.into();

            if relayed_group.epoch != task.relayed_group_epoch {
                continue;
            }

            let relayed_group_as_bytes = bincode::serialize(&relayed_group)?;

            let bls_core = SimpleBLSCore {};

            let partial_signature = bls_core.partial_sign(
                self.group_cache.read().await.get_secret_share()?,
                &relayed_group_as_bytes,
            )?;

            let threshold = self.group_cache.read().await.get_threshold()?;

            let current_group_index = self.group_cache.read().await.get_index()?;

            if self
                .group_cache
                .read()
                .await
                .is_committer(main_id_address)?
            {
                let contained_res = self
                    .group_relay_signature_cache
                    .read()
                    .await
                    .contains(task.controller_global_epoch);
                if !contained_res {
                    self.group_relay_signature_cache.write().await.add(
                        current_group_index,
                        task.controller_global_epoch,
                        relayed_group,
                        threshold,
                    )?;
                }

                self.group_relay_signature_cache
                    .write()
                    .await
                    .add_partial_signature(
                        task.controller_global_epoch,
                        main_id_address,
                        partial_signature.clone(),
                    )?;
            }

            for committer in committers.iter() {
                let retry_strategy = FixedInterval::from_millis(2000).take(3);

                if let Err(err) = RetryIf::spawn(
                    retry_strategy,
                    || {
                        committer.clone().commit_partial_signature(
                            0,
                            TaskType::GroupRelay,
                            relayed_group_as_bytes.clone(),
                            task.controller_global_epoch,
                            partial_signature.clone(),
                        )
                    },
                    |e: &NodeError| {
                        error!(
                            "send partial signature to committer {0} failed. Retry... Error: {1:?}",
                            committer.get_id_address(),
                            e
                        );
                        true
                    },
                )
                .await
                {
                    error!("{:?}", err);
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send + 'static,
        I: ChainIdentity + AdapterClientBuilder + Sync + Send + 'static,
        C: SignatureResultCacheUpdater<GroupRelayResultCache>
            + SignatureResultCacheFetcher<GroupRelayResultCache>
            + Sync
            + Send
            + 'static,
    > Subscriber for ReadyToHandleGroupRelayTaskSubscriber<G, I, C>
{
    async fn notify(&self, topic: Topic, payload: &(dyn Event + Send + Sync)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let ReadyToHandleGroupRelayTask { tasks } = payload
            .as_any()
            .downcast_ref::<ReadyToHandleGroupRelayTask>()
            .unwrap()
            .clone();

        let main_chain_identity = self.main_chain_identity.clone();

        let group_cache_for_handler = self.group_cache.clone();

        let group_relay_signature_cache_for_handler = self.group_relay_signature_cache.clone();

        self.ts.write().await.add_task(async move {
            let handler = GeneralGroupRelayHandler {
                main_chain_identity,
                tasks,
                group_cache: group_cache_for_handler,
                group_relay_signature_cache: group_relay_signature_cache_for_handler,
            };

            if let Err(e) = handler.handle().await {
                error!("{:?}", e);
            }
        });

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write()
            .await
            .subscribe(Topic::ReadyToHandleGroupRelayTask, subscriber);
    }
}
