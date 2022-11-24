use super::Subscriber;
use crate::node::{
    algorithm::dkg::{DKGCore, MockDKGCore},
    contract_client::{
        controller::ControllerTransactions,
        rpc_mock::{controller::MockControllerClient, coordinator::MockCoordinatorClient},
    },
    dal::GroupInfoUpdater,
    dal::{
        types::{ChainIdentity, DKGStatus, DKGTask},
        {GroupInfoFetcher, NodeInfoFetcher},
    },
    error::NodeResult,
    event::{run_dkg::RunDKG, types::Topic, Event},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, DynamicTaskScheduler},
};
use async_trait::async_trait;
use log::{error, info};
use parking_lot::RwLock;
use rand::{prelude::ThreadRng, RngCore};
use std::sync::Arc;

pub struct InGroupingSubscriber<
    N: NodeInfoFetcher + Sync + Send,
    G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send,
> {
    main_chain_identity: Arc<RwLock<ChainIdentity>>,
    node_cache: Arc<RwLock<N>>,
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl<N: NodeInfoFetcher + Sync + Send, G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send>
    InGroupingSubscriber<N, G>
{
    pub fn new(
        main_chain_identity: Arc<RwLock<ChainIdentity>>,
        node_cache: Arc<RwLock<N>>,
        group_cache: Arc<RwLock<G>>,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        InGroupingSubscriber {
            main_chain_identity,
            node_cache,
            group_cache,
            eq,
            ts,
        }
    }
}

pub struct AllInOneDKGHandler<
    F: Fn() -> R,
    R: RngCore,
    N: NodeInfoFetcher + Sync + Send,
    G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send,
> {
    id_address: String,
    controller_address: String,
    coordinator_rpc_endpoint: String,
    rng: F,
    node_cache: Arc<RwLock<N>>,
    group_cache: Arc<RwLock<G>>,
}

impl<
        F: Fn() -> R,
        R: RngCore,
        N: NodeInfoFetcher + Sync + Send,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send,
    > AllInOneDKGHandler<F, R, N, G>
{
    pub fn new(
        id_address: String,
        controller_address: String,
        coordinator_rpc_endpoint: String,
        rng: F,
        node_cache: Arc<RwLock<N>>,
        group_cache: Arc<RwLock<G>>,
    ) -> Self {
        AllInOneDKGHandler {
            id_address,
            controller_address,
            coordinator_rpc_endpoint,
            rng,
            node_cache,
            group_cache,
        }
    }
}

#[async_trait]
pub trait DKGHandler<F, R> {
    async fn handle(&self, task: DKGTask) -> NodeResult<()>
    where
        R: RngCore,
        F: Fn() -> R + 'static;
}

#[async_trait]
impl<
        F: Fn() -> R + Send + Sync + Copy + 'static,
        R: RngCore + 'static,
        N: NodeInfoFetcher + Sync + Send,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send,
    > DKGHandler<F, R> for AllInOneDKGHandler<F, R, N, G>
{
    async fn handle(&self, task: DKGTask) -> NodeResult<()>
    where
        R: RngCore,
        F: Fn() -> R + Send + 'async_trait,
    {
        let node_rpc_endpoint = self.node_cache.read().get_node_rpc_endpoint()?.to_string();

        let controller_client =
            MockControllerClient::new(self.controller_address.clone(), self.id_address.clone());

        let dkg_core = MockDKGCore {};

        let dkg_private_key = *self.node_cache.read().get_dkg_private_key()?;

        let id_address = self.node_cache.read().get_id_address().to_string();

        let task_group_index = task.group_index;

        let task_epoch = task.epoch;

        let coordinator_client = MockCoordinatorClient::new(
            self.coordinator_rpc_endpoint.clone(),
            id_address,
            task.group_index,
            task.epoch,
        );

        let output = dkg_core
            .run_dkg(
                dkg_private_key,
                node_rpc_endpoint,
                self.rng,
                coordinator_client,
            )
            .await?;

        let (public_key, partial_public_key, disqualified_nodes) = self
            .group_cache
            .write()
            .save_output(task_group_index, task_epoch, output)?;

        controller_client
            .commit_dkg(
                task_group_index,
                task_epoch,
                bincode::serialize(&public_key).unwrap(),
                bincode::serialize(&partial_public_key).unwrap(),
                disqualified_nodes,
            )
            .await?;

        Ok(())
    }
}

impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
    > Subscriber for InGroupingSubscriber<N, G>
{
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        info!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut RunDKG;

            let RunDKG { dkg_task: task } = *Box::from_raw(struct_ptr);

            static RNG_FN: fn() -> ThreadRng = rand::thread_rng;

            let controller_address = self
                .main_chain_identity
                .read()
                .get_provider_rpc_endpoint()
                .to_string();

            let coordinator_address = self
                .main_chain_identity
                .read()
                .get_provider_rpc_endpoint()
                .to_string();

            let id_address = self.main_chain_identity.read().get_id_address().to_string();

            let handler = AllInOneDKGHandler::new(
                id_address,
                controller_address,
                coordinator_address,
                RNG_FN,
                self.node_cache.clone(),
                self.group_cache.clone(),
            );

            let group_cache_for_handler = self.group_cache.clone();

            let group_cache_for_handler_shutdown_signal = self.group_cache.clone();

            let task_group_index = task.group_index;

            let task_epoch = task.epoch;

            self.ts.write().add_task_with_shutdown_signal(
                async move {
                    if let Err(e) = handler.handle(task).await {
                        error!("{:?}", e);
                    } else if let Err(e) = group_cache_for_handler.write().update_dkg_status(
                        task_group_index,
                        task_epoch,
                        DKGStatus::CommitSuccess,
                    ) {
                        error!("{:?}", e);
                    }
                },
                move || {
                    let cache_index = group_cache_for_handler_shutdown_signal
                        .read()
                        .get_index()
                        .unwrap_or(0);

                    let cache_epoch = group_cache_for_handler_shutdown_signal
                        .read()
                        .get_epoch()
                        .unwrap_or(0);

                    cache_index != task_group_index || cache_epoch != task_epoch
                    //NodeError::GroupIndexObsolete(cache_index)
                    //NodeError::GroupEpochObsolete(cache_epoch)
                },
                2000,
            );
        }

        Ok(())
    }

    fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write().subscribe(Topic::RunDKG, subscriber);
    }
}