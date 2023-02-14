use super::{
    chain::{types::GeneralMainChain, Chain, MainChainFetcher},
    CommitterServerStarter, Context, ContextFetcher, ManagementServerStarter, TaskWaiter,
};
use crate::node::{
    committer::server as committer_server,
    error::ConfigError,
    management::server as management_server,
    queue::event_queue::EventQueue,
    scheduler::{
        dynamic::SimpleDynamicTaskScheduler, fixed::SimpleFixedTaskScheduler, ListenerType,
        RpcServerType, TaskScheduler, TaskType,
    },
};
use arpa_node_contract_client::{
    adapter::AdapterClientBuilder, controller::ControllerClientBuilder,
    coordinator::CoordinatorClientBuilder, provider::ChainProviderBuilder,
};
use arpa_node_core::{ChainIdentity, RandomnessTask, SchedulerResult};
use arpa_node_dal::{
    BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater, MdcContextUpdater,
    NodeInfoFetcher, NodeInfoUpdater,
};
use async_trait::async_trait;
use ethers::{
    prelude::k256::ecdsa::SigningKey,
    signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Wallet},
};
use log::error;
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub node_committer_rpc_endpoint: String,
    pub node_management_rpc_endpoint: String,
    pub node_management_rpc_token: String,
    pub provider_endpoint: String,
    pub controller_address: String,
    // Data file for persistence
    pub data_path: Option<String>,
    pub account: Account,
    pub listeners: Option<Vec<ListenerType>>,
}

impl Config {
    pub fn get_node_management_rpc_token(&self) -> Result<String, ConfigError> {
        if self.node_management_rpc_token.eq("env") {
            let token = env::var("ARPA_NODE_MANAGEMENT_SERVER_TOKEN")?;
            return Ok(token);
        }
        Ok(self.node_management_rpc_token.clone())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Adapter {
    pub id: usize,
    pub name: String,
    pub endpoint: String,
    pub account: Account,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub hdwallet: Option<HDWallet>,
    pub keystore: Option<Keystore>,
    // not recommended
    pub private_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keystore {
    pub path: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HDWallet {
    pub mnemonic: String,
    pub path: Option<String>,
    pub index: u32,
    pub passphrase: Option<String>,
}

#[derive(Debug)]
pub struct GeneralContext<
    N: NodeInfoFetcher<C> + NodeInfoUpdater<C> + MdcContextUpdater,
    G: GroupInfoFetcher<C> + GroupInfoUpdater<C> + MdcContextUpdater,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
    I: ChainIdentity + ControllerClientBuilder + CoordinatorClientBuilder + AdapterClientBuilder<C>,
    C: PairingCurve,
> {
    config: Config,
    main_chain: GeneralMainChain<N, G, T, I, C>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    f_ts: Arc<RwLock<SimpleFixedTaskScheduler>>,
}

impl<
        N: NodeInfoFetcher<C> + NodeInfoUpdater<C> + MdcContextUpdater + Sync + Send + 'static,
        G: GroupInfoFetcher<C> + GroupInfoUpdater<C> + MdcContextUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<C>
            + ChainProviderBuilder
            + Sync
            + Send
            + 'static,
        C: PairingCurve,
    > GeneralContext<N, G, T, I, C>
{
    pub fn new(config: Config, main_chain: GeneralMainChain<N, G, T, I, C>) -> Self {
        GeneralContext {
            config,
            main_chain,
            eq: Arc::new(RwLock::new(EventQueue::new())),
            ts: Arc::new(RwLock::new(SimpleDynamicTaskScheduler::new())),
            f_ts: Arc::new(RwLock::new(SimpleFixedTaskScheduler::new())),
        }
    }
}

impl<
        N: NodeInfoFetcher<C>
            + NodeInfoUpdater<C>
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<C>
            + GroupInfoUpdater<C>
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<C>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > Context for GeneralContext<N, G, T, I, C>
{
    type MainChain = GeneralMainChain<N, G, T, I, C>;

    async fn deploy(self) -> SchedulerResult<ContextHandle> {
        self.get_main_chain().init_components(&self).await?;

        let f_ts = self.get_fixed_task_handler();

        let rpc_endpoint = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_node_rpc_endpoint()
            .unwrap()
            .to_string();

        let management_server_rpc_endpoint = self.config.node_management_rpc_endpoint.clone();

        let context = Arc::new(RwLock::new(self));

        f_ts.write()
            .await
            .start_committer_server(rpc_endpoint, context.clone())?;

        f_ts.write()
            .await
            .start_management_server(management_server_rpc_endpoint, context.clone())?;

        let ts = context.read().await.get_dynamic_task_handler();

        Ok(ContextHandle { ts })
    }
}

impl<
        N: NodeInfoFetcher<C>
            + NodeInfoUpdater<C>
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<C>
            + GroupInfoUpdater<C>
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<C>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > ContextFetcher<GeneralContext<N, G, T, I, C>> for GeneralContext<N, G, T, I, C>
{
    fn get_config(&self) -> &Config {
        &self.config
    }

    fn get_main_chain(&self) -> &<GeneralContext<N, G, T, I, C> as Context>::MainChain {
        &self.main_chain
    }

    fn get_fixed_task_handler(&self) -> Arc<RwLock<SimpleFixedTaskScheduler>> {
        self.f_ts.clone()
    }

    fn get_dynamic_task_handler(&self) -> Arc<RwLock<SimpleDynamicTaskScheduler>> {
        self.ts.clone()
    }

    fn get_event_queue(&self) -> Arc<RwLock<EventQueue>> {
        self.eq.clone()
    }
}

pub struct ContextHandle {
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

#[async_trait]
impl TaskWaiter for ContextHandle {
    async fn wait_task(&self) {
        loop {
            while !self.ts.read().await.dynamic_tasks.is_empty() {
                let (task_recv, task_monitor) = self.ts.write().await.dynamic_tasks.pop().unwrap();

                let _ = task_recv.await;

                if let Some(monitor) = task_monitor {
                    monitor.abort();
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}

impl<
        N: NodeInfoFetcher<C>
            + NodeInfoUpdater<C>
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<C>
            + GroupInfoUpdater<C>
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<C>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > CommitterServerStarter<GeneralContext<N, G, T, I, C>> for SimpleFixedTaskScheduler
{
    fn start_committer_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<GeneralContext<N, G, T, I, C>>>,
    ) -> SchedulerResult<()> {
        self.add_task(TaskType::RpcServer(RpcServerType::Committer), async move {
            if let Err(e) = committer_server::start_committer_server(rpc_endpoint, context).await {
                error!("{:?}", e);
            };
        })
    }
}

impl<
        N: NodeInfoFetcher<C>
            + NodeInfoUpdater<C>
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<C>
            + GroupInfoUpdater<C>
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<C>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > ManagementServerStarter<GeneralContext<N, G, T, I, C>> for SimpleFixedTaskScheduler
{
    fn start_management_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<GeneralContext<N, G, T, I, C>>>,
    ) -> SchedulerResult<()> {
        self.add_task(TaskType::RpcServer(RpcServerType::Management), async move {
            if let Err(e) = management_server::start_management_server(rpc_endpoint, context).await
            {
                error!("{:?}", e);
            };
        })
    }
}

pub fn build_wallet_from_config(account: &Account) -> Result<Wallet<SigningKey>, ConfigError> {
    if account.hdwallet.is_some() {
        let mut hd = account.hdwallet.clone().unwrap();
        if hd.mnemonic.eq("env") {
            hd.mnemonic = env::var("ARPA_NODE_HD_ACCOUNT_MNEMONIC")?;
        }
        let mut wallet = MnemonicBuilder::<English>::default().phrase(&*hd.mnemonic);

        if hd.path.is_some() {
            wallet = wallet.derivation_path(&hd.path.unwrap()).unwrap();
        }
        if hd.passphrase.is_some() {
            wallet = wallet.password(&hd.passphrase.unwrap());
        }
        return Ok(wallet.index(hd.index).unwrap().build()?);
    } else if account.keystore.is_some() {
        let mut keystore = account.keystore.clone().unwrap();
        if keystore.password.eq("env") {
            keystore.password = env::var("ARPA_NODE_ACCOUNT_KEYSTORE_PASSWORD")?;
        }
        return Ok(LocalWallet::decrypt_keystore(
            &keystore.path,
            &keystore.password,
        )?);
    } else if account.private_key.is_some() {
        let mut private_key = account.private_key.clone().unwrap();
        if private_key.eq("env") {
            private_key = env::var("ARPA_NODE_ACCOUNT_PRIVATE_KEY")?;
        }
        return Ok(private_key.parse::<Wallet<SigningKey>>()?);
    }

    Err(ConfigError::LackOfAccount)
}

#[cfg(test)]
mod tests {
    use std::fs::read_to_string;

    use crate::node::{context::types::Config, scheduler::ListenerType};

    #[test]
    fn test_enum_serialization() {
        let listener_type = ListenerType::Block;
        let serialize = serde_json::to_string(&listener_type).unwrap();
        println!("serialize = {}", serialize);
    }

    #[test]
    fn test_deserialization_from_config() {
        let config_str = &read_to_string("config.yml").unwrap_or_else(|_| {
            panic!("Error loading configuration file, please check the configuration!")
        });

        let config: Config =
            serde_yaml::from_str(config_str).expect("Error loading configuration file");
        println!("listeners = {:?}", config.listeners);
    }
}
