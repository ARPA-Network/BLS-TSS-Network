use super::{
    chain::{
        types::{GeneralAdapterChain, GeneralMainChain},
        Chain, ChainFetcher, MainChainFetcher,
    },
    CommitterServerStarter, Context, ContextFetcher, TaskWaiter,
};
use crate::node::{
    committer::server,
    error::{ConfigError, NodeError, NodeResult},
    queue::event_queue::EventQueue,
    scheduler::{
        dynamic::SimpleDynamicTaskScheduler, fixed::SimpleFixedTaskScheduler, TaskScheduler,
    },
};
use arpa_node_contract_client::{
    adapter::AdapterClientBuilder, controller::ControllerClientBuilder,
    coordinator::CoordinatorClientBuilder, provider::ChainProviderBuilder,
};
use arpa_node_core::{ChainIdentity, RandomnessTask};
use arpa_node_dal::{
    BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater, NodeInfoFetcher,
};
use async_trait::async_trait;
use ethers::{
    prelude::k256::ecdsa::SigningKey,
    signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Wallet},
};
use log::error;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, sync::Arc};
use tokio::sync::RwLock;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub node_rpc_endpoint: String,
    pub provider_endpoint: String,
    pub controller_address: String,
    // Data file for persistence
    pub data_path: Option<String>,
    pub account: Account,
    pub adapters: Vec<Adapter>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Keystore {
    pub path: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HDWallet {
    pub mnemonic: String,
    pub path: Option<String>,
    pub index: u32,
    pub passphrase: Option<String>,
}

pub struct GeneralContext<
    N: NodeInfoFetcher,
    G: GroupInfoFetcher + GroupInfoUpdater,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
    I: ChainIdentity + ControllerClientBuilder + CoordinatorClientBuilder + AdapterClientBuilder,
> {
    main_chain: GeneralMainChain<N, G, T, I>,
    adapter_chains: HashMap<usize, GeneralAdapterChain<N, G, T, I>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    f_ts: Arc<RwLock<SimpleFixedTaskScheduler>>,
}

impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + Sync
            + Send
            + 'static,
    > GeneralContext<N, G, T, I>
{
    pub fn new(main_chain: GeneralMainChain<N, G, T, I>) -> Self {
        GeneralContext {
            main_chain,
            adapter_chains: HashMap::new(),
            eq: Arc::new(RwLock::new(EventQueue::new())),
            ts: Arc::new(RwLock::new(SimpleDynamicTaskScheduler::new())),
            f_ts: Arc::new(RwLock::new(SimpleFixedTaskScheduler::new())),
        }
    }

    pub fn add_adapter_chain(
        &mut self,
        adapter_chain: GeneralAdapterChain<N, G, T, I>,
    ) -> NodeResult<()> {
        if self.adapter_chains.contains_key(&adapter_chain.id()) {
            return Err(NodeError::RepeatedChainId);
        }

        self.adapter_chains
            .insert(adapter_chain.id(), adapter_chain);

        Ok(())
    }
}

impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + Sync
            + Send
            + 'static,
    > Context for GeneralContext<N, G, T, I>
{
    type MainChain = GeneralMainChain<N, G, T, I>;

    type AdapterChain = GeneralAdapterChain<N, G, T, I>;

    async fn deploy(self) -> ContextHandle {
        self.get_main_chain().init_components(&self).await;

        for adapter_chain in self.adapter_chains.values() {
            adapter_chain.init_components(&self).await;
        }

        let f_ts = self.get_fixed_task_handler();

        let rpc_endpoint = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_node_rpc_endpoint()
            .unwrap()
            .to_string();

        let context = Arc::new(RwLock::new(self));

        f_ts.write()
            .await
            .start_committer_server(rpc_endpoint, context.clone());

        let ts = context.read().await.get_dynamic_task_handler();

        ContextHandle { ts }
    }
}

impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + Sync
            + Send
            + 'static,
    > ContextFetcher<GeneralContext<N, G, T, I>> for GeneralContext<N, G, T, I>
{
    fn contains_chain(&self, index: usize) -> bool {
        self.adapter_chains.contains_key(&index)
    }

    fn get_adapter_chain(
        &self,
        index: usize,
    ) -> Option<&<GeneralContext<N, G, T, I> as Context>::AdapterChain> {
        self.adapter_chains.get(&index)
    }

    fn get_main_chain(&self) -> &<GeneralContext<N, G, T, I> as Context>::MainChain {
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
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + Sync
            + Send
            + 'static,
    > CommitterServerStarter<GeneralContext<N, G, T, I>> for SimpleFixedTaskScheduler
{
    fn start_committer_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<GeneralContext<N, G, T, I>>>,
    ) {
        self.add_task(async move {
            if let Err(e) = server::start_committer_server(rpc_endpoint, context).await {
                error!("{:?}", e);
            };
        });
    }
}

pub fn build_wallet_from_config(account: Account) -> Result<Wallet<SigningKey>, ConfigError> {
    if account.hdwallet.is_some() {
        let mut hd = account.hdwallet.unwrap();
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
        let mut keystore = account.keystore.unwrap();
        if keystore.password.eq("env") {
            keystore.password = env::var("ARPA_NODE_ACCOUNT_KEYSTORE_PASSWORD")?;
        }
        return Ok(LocalWallet::decrypt_keystore(
            &keystore.path,
            &keystore.password,
        )?);
    } else if account.private_key.is_some() {
        let mut private_key = account.private_key.unwrap();
        if private_key.eq("env") {
            private_key = env::var("ARPA_NODE_ACCOUNT_PRIVATE_KEY")?;
        }
        return Ok(private_key.parse::<Wallet<SigningKey>>()?);
    }

    Err(ConfigError::LackOfAccount)
}
