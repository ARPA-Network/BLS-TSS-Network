pub mod chain;
pub mod types;

use self::{chain::RelayedChain, types::ContextHandle};

use crate::{
    error::NodeResult,
    queue::event_queue::EventQueue,
    scheduler::{dynamic::SimpleDynamicTaskScheduler, fixed::SimpleFixedTaskScheduler},
};
use arpa_contract_client::{
    adapter::AdapterClientBuilder,
    controller::ControllerClientBuilder,
    controller_oracle::ControllerOracleClientBuilder,
    controller_relayer::ControllerRelayerClientBuilder,
    coordinator::CoordinatorClientBuilder,
    ethers::{
        adapter::AdapterClient, controller::ControllerClient,
        controller_oracle::ControllerOracleClient, controller_relayer::ControllerRelayerClient,
        coordinator::CoordinatorClient,
    },
};
use arpa_core::{ChainIdentity, ChainProviderManager, Config, RandomnessTask, SchedulerResult};
use arpa_dal::cache::RandomnessResultCache;
use arpa_dal::{
    BLSTasksHandler, BlockInfoHandler, GroupInfoHandler, NodeInfoHandler,
    SignatureResultCacheHandler,
};
use async_trait::async_trait;
use std::sync::Arc;
use threshold_bls::{
    group::Curve,
    sig::{SignatureScheme, ThresholdScheme},
};
use tokio::sync::RwLock;

pub trait ChainIdentityHandler<PC: Curve>:
    ChainIdentity
    + ChainProviderManager
    + ControllerClientBuilder<PC>
    + ControllerRelayerClientBuilder
    + ControllerOracleClientBuilder<PC>
    + CoordinatorClientBuilder<PC>
    + AdapterClientBuilder
    + std::fmt::Debug
    + Sync
    + Send
{
}

pub type ChainIdentityHandlerType<PC> = Box<
    dyn ChainIdentityHandler<
        PC,
        ControllerService = ControllerClient,
        ControllerRelayerService = ControllerRelayerClient,
        ControllerOracleService = ControllerOracleClient,
        CoordinatorService = CoordinatorClient,
        AdapterService = AdapterClient,
    >,
>;

pub type RelayedChainType<PC, S> = Box<
    dyn RelayedChain<
            PC,
            S,
            NodeInfoCache = Box<dyn NodeInfoHandler<PC>>,
            GroupInfoCache = Box<dyn GroupInfoHandler<PC>>,
            BlockInfoCache = Box<dyn BlockInfoHandler>,
            RandomnessTasksQueue = Box<dyn BLSTasksHandler<RandomnessTask>>,
            RandomnessResultCaches = Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>,
            ChainIdentity = ChainIdentityHandlerType<PC>,
        > + Sync
        + Send,
>;

pub trait Context<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
>
{
    type MainChain;

    fn get_main_chain(&self) -> &Self::MainChain;

    fn contains_relayed_chain(&self, index: usize) -> bool;

    fn get_relayed_chain(&self, index: usize) -> Option<&RelayedChainType<PC, S>>;

    fn add_relayed_chain(&mut self, relayed_chain: RelayedChainType<PC, S>) -> NodeResult<()>;

    async fn deploy(self) -> SchedulerResult<ContextHandle>;
}

#[async_trait]
pub trait TaskWaiter {
    async fn wait_task(&self);
}

pub trait ContextFetcher {
    fn get_supported_relayed_chains(&self) -> Vec<usize>;

    fn get_fixed_task_handler(&self) -> Arc<RwLock<SimpleFixedTaskScheduler>>;

    fn get_dynamic_task_handler(&self) -> Arc<RwLock<SimpleDynamicTaskScheduler>>;

    fn get_event_queue(&self) -> Arc<RwLock<EventQueue>>;

    fn get_config(&self) -> &Config;
}

pub(crate) trait CommitterServerStarter<
    C: Context<PC, S>,
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
>
{
    fn start_committer_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<C>>,
    ) -> SchedulerResult<()>;
}

pub(crate) trait ManagementServerStarter<
    C: Context<PC, S>,
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
>
{
    fn start_management_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<C>>,
    ) -> SchedulerResult<()>;
}
