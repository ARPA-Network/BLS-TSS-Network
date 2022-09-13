use super::Subscriber;
use crate::node::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    committer::{
        client::MockCommitterClient, CommitterClient, CommitterClientHandler, CommitterService,
    },
    contract_client::adapter::AdapterClientBuilder,
    contract_client::{adapter::AdapterViews, types::Group as ContractGroup},
    dal::types::{GroupRelayConfirmation, GroupRelayConfirmationTask, Status, TaskType},
    dal::{
        cache::GroupRelayConfirmationResultCache,
        ChainIdentity,
        {GroupInfoFetcher, SignatureResultCacheFetcher, SignatureResultCacheUpdater},
    },
    error::{NodeError, NodeResult},
    event::{
        ready_to_handle_group_relay_confirmation_task::ReadyToHandleGroupRelayConfirmationTask,
        types::Topic, Event,
    },
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use async_trait::async_trait;
use ethers::types::Address;
use log::{error, info};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct ReadyToHandleGroupRelayConfirmationTaskSubscriber<
    G: GroupInfoFetcher,
    I: ChainIdentity + AdapterClientBuilder,
    C: SignatureResultCacheUpdater<GroupRelayConfirmationResultCache>
        + SignatureResultCacheFetcher<GroupRelayConfirmationResultCache>,
> {
    pub chain_id: usize,
    main_chain_identity: Arc<RwLock<I>>,
    chain_identity: Arc<RwLock<I>>,
    group_cache: Arc<RwLock<G>>,
    group_relay_confirmation_signature_cache: Arc<RwLock<C>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl<
        G: GroupInfoFetcher,
        I: ChainIdentity + AdapterClientBuilder,
        C: SignatureResultCacheUpdater<GroupRelayConfirmationResultCache>
            + SignatureResultCacheFetcher<GroupRelayConfirmationResultCache>,
    > ReadyToHandleGroupRelayConfirmationTaskSubscriber<G, I, C>
{
    pub fn new(
        chain_id: usize,
        main_chain_identity: Arc<RwLock<I>>,
        chain_identity: Arc<RwLock<I>>,
        group_cache: Arc<RwLock<G>>,
        group_relay_confirmation_signature_cache: Arc<RwLock<C>>,
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
    async fn handle(self) -> NodeResult<()>;
}

pub struct GeneralGroupRelayConfirmationHandler<
    G: GroupInfoFetcher,
    I: ChainIdentity + AdapterClientBuilder,
    C: SignatureResultCacheUpdater<GroupRelayConfirmationResultCache>
        + SignatureResultCacheFetcher<GroupRelayConfirmationResultCache>,
> {
    chain_id: usize,
    main_chain_identity: Arc<RwLock<I>>,
    chain_identity: Arc<RwLock<I>>,
    tasks: Vec<GroupRelayConfirmationTask>,
    group_cache: Arc<RwLock<G>>,
    group_relay_confirmation_signature_cache: Arc<RwLock<C>>,
}

impl<
        G: GroupInfoFetcher,
        I: ChainIdentity + AdapterClientBuilder,
        C: SignatureResultCacheUpdater<GroupRelayConfirmationResultCache>
            + SignatureResultCacheFetcher<GroupRelayConfirmationResultCache>,
    > CommitterClientHandler<MockCommitterClient, G>
    for GeneralGroupRelayConfirmationHandler<G, I, C>
{
    fn get_id_address(&self) -> Address {
        self.main_chain_identity.read().get_id_address()
    }

    fn get_group_cache(&self) -> Arc<RwLock<G>> {
        self.group_cache.clone()
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher + Sync + Send,
        I: ChainIdentity + AdapterClientBuilder + Sync + Send,
        C: SignatureResultCacheUpdater<GroupRelayConfirmationResultCache>
            + SignatureResultCacheFetcher<GroupRelayConfirmationResultCache>
            + Sync
            + Send,
    > GroupRelayConfirmationHandler for GeneralGroupRelayConfirmationHandler<G, I, C>
{
    async fn handle(self) -> NodeResult<()> {
        let main_id_address = self.main_chain_identity.read().get_id_address();
        let controller_client = self
            .main_chain_identity
            .read()
            .build_adapter_client(main_id_address);

        let adapter_client = self
            .chain_identity
            .read()
            .build_adapter_client(main_id_address);

        let committers = self.prepare_committer_clients()?;

        for task in self.tasks {
            let relayed_group = controller_client
                .get_group(task.relayed_group_index)
                .await?;

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

            info!("group_relay_confirmation: {:?}", group_relay_confirmation);

            let group_relay_confirmation_as_bytes = bincode::serialize(&group_relay_confirmation)?;

            let bls_core = SimpleBLSCore {};

            let partial_signature = bls_core.partial_sign(
                self.group_cache.read().get_secret_share()?,
                &group_relay_confirmation_as_bytes,
            )?;

            let threshold = self.group_cache.read().get_threshold()?;

            let current_group_index = self.group_cache.read().get_index()?;

            if self.group_cache.read().is_committer(main_id_address)? {
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
                        main_id_address,
                        partial_signature.clone(),
                    )?;
            }

            for committer in committers.iter() {
                let retry_strategy = FixedInterval::from_millis(2000).take(3);

                let chain_id = self.chain_id;

                if let Err(err) = RetryIf::spawn(
                    retry_strategy,
                    || {
                        committer.clone().commit_partial_signature(
                            chain_id,
                            TaskType::GroupRelayConfirmation,
                            group_relay_confirmation_as_bytes.clone(),
                            task.index,
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

impl<
        G: GroupInfoFetcher + Sync + Send + 'static,
        I: ChainIdentity + AdapterClientBuilder + Sync + Send + 'static,
        C: SignatureResultCacheUpdater<GroupRelayConfirmationResultCache>
            + SignatureResultCacheFetcher<GroupRelayConfirmationResultCache>
            + Sync
            + Send
            + 'static,
    > Subscriber for ReadyToHandleGroupRelayConfirmationTaskSubscriber<G, I, C>
{
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        info!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut ReadyToHandleGroupRelayConfirmationTask;

            let ReadyToHandleGroupRelayConfirmationTask { chain_id: _, tasks } =
                *Box::from_raw(struct_ptr);

            let chain_id = self.chain_identity.read().get_id();

            let main_chain_identity = self.main_chain_identity.clone();

            let chain_identity = self.chain_identity.clone();

            let group_cache_for_handler = self.group_cache.clone();

            let group_relay_confirmation_signature_cache_for_handler =
                self.group_relay_confirmation_signature_cache.clone();

            self.ts.write().add_task(async move {
                let handler = GeneralGroupRelayConfirmationHandler {
                    chain_id,
                    main_chain_identity,
                    chain_identity,
                    tasks,
                    group_cache: group_cache_for_handler,
                    group_relay_confirmation_signature_cache:
                        group_relay_confirmation_signature_cache_for_handler,
                };

                if let Err(e) = handler.handle().await {
                    error!("{:?}", e);
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
