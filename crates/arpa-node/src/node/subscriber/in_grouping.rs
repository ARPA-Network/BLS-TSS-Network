use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::node::{
    algorithm::dkg::{AllPhasesDKGCore, DKGCore},
    error::NodeResult,
    event::{run_dkg::RunDKG, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, DynamicTaskScheduler},
};
use arpa_node_contract_client::{
    controller::{ControllerClientBuilder, ControllerTransactions},
    coordinator::CoordinatorClientBuilder,
};
use arpa_node_core::{ChainIdentity, DKGStatus, DKGTask};
use arpa_node_dal::{
    ContextInfoUpdater, GroupInfoFetcher, GroupInfoUpdater, NodeInfoFetcher, NodeInfoUpdater,
};
use async_trait::async_trait;
use core::fmt::Debug;
use log::{debug, error};
use rand::{prelude::ThreadRng, RngCore};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::{curve::bn254::Scalar, group::PairingCurve};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct InGroupingSubscriber<
    N: NodeInfoFetcher<C>,
    G: GroupInfoFetcher<C> + GroupInfoUpdater<C>,
    I: ChainIdentity + ControllerClientBuilder + CoordinatorClientBuilder + std::fmt::Debug,
    C: PairingCurve,
> {
    main_chain_identity: Arc<RwLock<I>>,
    node_cache: Arc<RwLock<N>>,
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<C>,
}

impl<
        N: NodeInfoFetcher<C>,
        G: GroupInfoFetcher<C> + GroupInfoUpdater<C>,
        I: ChainIdentity + ControllerClientBuilder + CoordinatorClientBuilder + std::fmt::Debug,
        C: PairingCurve,
    > InGroupingSubscriber<N, G, I, C>
{
    pub fn new(
        main_chain_identity: Arc<RwLock<I>>,
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
            c: PhantomData,
        }
    }
}

pub struct AllInOneDKGHandler<
    F: Fn() -> R,
    R: RngCore,
    I: ChainIdentity + ControllerClientBuilder + CoordinatorClientBuilder,
    N: NodeInfoFetcher<C>,
    G: GroupInfoFetcher<C> + GroupInfoUpdater<C>,
    C: PairingCurve,
> {
    rng: F,
    main_chain_identity: Arc<RwLock<I>>,
    node_cache: Arc<RwLock<N>>,
    group_cache: Arc<RwLock<G>>,
    c: PhantomData<C>,
}

impl<
        F: Fn() -> R,
        R: RngCore,
        I: ChainIdentity + ControllerClientBuilder + CoordinatorClientBuilder,
        N: NodeInfoFetcher<C>,
        G: GroupInfoFetcher<C> + GroupInfoUpdater<C>,
        C: PairingCurve,
    > AllInOneDKGHandler<F, R, I, N, G, C>
{
    pub fn new(
        rng: F,
        main_chain_identity: Arc<RwLock<I>>,
        node_cache: Arc<RwLock<N>>,
        group_cache: Arc<RwLock<G>>,
    ) -> Self {
        AllInOneDKGHandler {
            rng,
            main_chain_identity,
            node_cache,
            group_cache,
            c: PhantomData,
        }
    }
}

#[async_trait]
pub trait DKGHandler<F, R> {
    async fn handle(&mut self, task: DKGTask) -> NodeResult<()>
    where
        R: RngCore,
        F: Fn() -> R + 'static;
}

#[async_trait]
impl<
        F: Fn() -> R + Debug + Send + Sync + Copy + 'static,
        R: RngCore + 'static,
        I: ChainIdentity + ControllerClientBuilder + CoordinatorClientBuilder + Sync + Send,
        N: NodeInfoFetcher<C> + Sync + Send,
        G: GroupInfoFetcher<C> + GroupInfoUpdater<C> + Sync + Send,
        C: PairingCurve + Sync + Send + 'static,
    > DKGHandler<F, R> for AllInOneDKGHandler<F, R, I, N, G, C>
{
    async fn handle(&mut self, task: DKGTask) -> NodeResult<()>
    where
        R: RngCore,
        F: Fn() -> R + Send + Debug + 'async_trait,
    {
        let node_rpc_endpoint = self
            .node_cache
            .read()
            .await
            .get_node_rpc_endpoint()?
            .to_string();

        let controller_client = self
            .main_chain_identity
            .read()
            .await
            .build_controller_client();

        let dkg_private_key = self.node_cache.read().await.get_dkg_private_key()?.clone();

        let dkg_private_key: Scalar = bincode::deserialize(&bincode::serialize(&dkg_private_key)?)?;

        let task_group_index = task.group_index;

        let task_epoch = task.epoch;

        let coordinator_client = self
            .main_chain_identity
            .read()
            .await
            .build_coordinator_client(task.coordinator_address);

        let mut dkg_core = AllPhasesDKGCore::new(coordinator_client);

        let output = dkg_core
            .run_dkg(dkg_private_key, node_rpc_endpoint, self.rng)
            .await?;

        let (public_key, partial_public_key, disqualified_nodes) = self
            .group_cache
            .write()
            .await
            .save_output(task_group_index, task_epoch, output)
            .await?;

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

#[async_trait]
impl<
        N: NodeInfoFetcher<C>
            + NodeInfoUpdater<C>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<C>
            + GroupInfoUpdater<C>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + std::fmt::Debug
            + Sync
            + Send
            + 'static,
        C: PairingCurve + std::fmt::Debug + Sync + Send + 'static,
    > Subscriber for InGroupingSubscriber<N, G, I, C>
{
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let RunDKG { dkg_task: task, .. } =
            payload.as_any().downcast_ref::<RunDKG>().unwrap().clone();

        static RNG_FN: fn() -> ThreadRng = rand::thread_rng;

        let chain_identity = self.main_chain_identity.clone();

        let group_cache_for_handler = self.group_cache.clone();

        let group_cache_for_handler_shutdown_signal = self.group_cache.clone();

        let task_group_index = task.group_index;

        let task_epoch = task.epoch;

        let mut handler = AllInOneDKGHandler::new(
            RNG_FN,
            chain_identity,
            self.node_cache.clone(),
            self.group_cache.clone(),
        );

        self.ts.write().await.add_task_with_shutdown_signal(
            async move {
                if let Err(e) = handler.handle(task).await {
                    error!("{:?}", e);
                } else if let Err(e) = group_cache_for_handler
                    .write()
                    .await
                    .update_dkg_status(task_group_index, task_epoch, DKGStatus::CommitSuccess)
                    .await
                {
                    error!("{:?}", e);
                }
            },
            move || {
                let group_cache = group_cache_for_handler_shutdown_signal.clone();
                async move {
                    let cache_index = group_cache.clone().read().await.get_index().unwrap_or(0);

                    let cache_epoch = group_cache.clone().read().await.get_epoch().unwrap_or(0);

                    cache_index != task_group_index || cache_epoch != task_epoch
                    //NodeError::GroupIndexObsolete(cache_index)
                    //NodeError::GroupEpochObsolete(cache_epoch)
                }
            },
            2000,
        );

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write().await.subscribe(Topic::RunDKG, subscriber);
    }
}

impl<
        N: NodeInfoFetcher<C>
            + NodeInfoUpdater<C>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<C>
            + GroupInfoUpdater<C>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + std::fmt::Debug
            + Sync
            + Send
            + 'static,
        C: PairingCurve + std::fmt::Debug + Sync + Send + 'static,
    > DebuggableSubscriber for InGroupingSubscriber<N, G, I, C>
{
}
