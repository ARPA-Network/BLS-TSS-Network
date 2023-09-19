use crate::{ConfigError, SchedulerError};
use ethers_core::rand::{thread_rng, Rng};
use ethers_core::{k256::ecdsa::SigningKey, types::Address};
use ethers_signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Wallet};
use serde::de;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::env;
use std::fmt::{self};
use std::time::Duration;
use std::{fs::read_to_string, path::PathBuf};

pub const PLACEHOLDER_ADDRESS: Address = Address::zero();

pub const DEFAULT_LISTENER_INTERVAL_MILLIS: u64 = 10000;
pub const DEFAULT_LISTENER_USE_JITTER: bool = true;

pub const DEFAULT_DKG_TIMEOUT_DURATION: usize = 10 * 4;
pub const DEFAULT_RANDOMNESS_TASK_EXCLUSIVE_WINDOW: usize = 10;
pub const DEFAULT_DKG_WAIT_FOR_PHASE_INTERVAL_MILLIS: u64 = 10000;
pub const DEFAULT_DKG_WAIT_FOR_PHASE_USE_JITTER: bool = true;

pub const DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_BASE: u64 = 2;
pub const DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_FACTOR: u64 = 1000;
pub const DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_MAX_ATTEMPTS: usize = 5;
pub const DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_USE_JITTER: bool = true;

pub const DEFAULT_CONTRACT_TRANSACTION_RETRY_BASE: u64 = 2;
pub const DEFAULT_CONTRACT_TRANSACTION_RETRY_FACTOR: u64 = 1000;
pub const DEFAULT_CONTRACT_TRANSACTION_RETRY_MAX_ATTEMPTS: usize = 3;
pub const DEFAULT_CONTRACT_TRANSACTION_RETRY_USE_JITTER: bool = true;

pub const DEFAULT_CONTRACT_VIEW_RETRY_BASE: u64 = 2;
pub const DEFAULT_CONTRACT_VIEW_RETRY_FACTOR: u64 = 500;
pub const DEFAULT_CONTRACT_VIEW_RETRY_MAX_ATTEMPTS: usize = 3;
pub const DEFAULT_CONTRACT_VIEW_RETRY_USE_JITTER: bool = true;

pub const DEFAULT_PROVIDER_POLLING_INTERVAL_MILLIS: u64 = 10000;

pub const DEFAULT_DYNAMIC_TASK_CLEANER_INTERVAL_MILLIS: u64 = 1000;

pub const FULFILL_RANDOMNESS_GAS_EXCEPT_CALLBACK: u32 = 670000;
pub const RANDOMNESS_REWARD_GAS: u32 = 9000;
pub const VERIFICATION_GAS_OVER_MINIMUM_THRESHOLD: u32 = 50000;
pub const DEFAULT_MINIMUM_THRESHOLD: u32 = 3;

pub const DEFAULT_ROLLING_LOG_FILE_SIZE: u64 = 10 * 1024 * 1024 * 1024;

pub const DEFAULT_BLOCK_TIME: usize = 12;
pub const DEFAULT_MAX_RANDOMNESS_FULFILLMENT_ATTEMPTS: usize = 3;

pub fn jitter(duration: Duration) -> Duration {
    duration.mul_f64(thread_rng().gen_range(0.5..=1.0))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub node_committer_rpc_endpoint: String,
    pub node_advertised_committer_rpc_endpoint: Option<String>,
    pub node_management_rpc_endpoint: String,
    pub node_management_rpc_token: String,
    pub provider_endpoint: String,
    pub chain_id: usize,
    pub controller_address: String,
    pub controller_relayer_address: String,
    pub adapter_address: String,
    adapter_deployed_block_height: Option<u64>,
    arpa_contract_address: Option<String>,
    // Data file for persistence
    pub data_path: Option<String>,
    pub account: Account,
    pub listeners: Option<Vec<ListenerDescriptor>>,
    pub logger: Option<LoggerDescriptor>,
    pub time_limits: Option<TimeLimitDescriptor>,
    pub relayed_chains: Vec<RelayedChain>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            node_committer_rpc_endpoint: "[::1]:50060".to_string(),
            node_advertised_committer_rpc_endpoint: Some("[::1]:50060".to_string()),
            node_management_rpc_endpoint: "[::1]:50099".to_string(),
            node_management_rpc_token: "for_test".to_string(),
            provider_endpoint: "localhost:8545".to_string(),
            chain_id: 0,
            controller_address: PLACEHOLDER_ADDRESS.to_string(),
            controller_relayer_address: PLACEHOLDER_ADDRESS.to_string(),
            adapter_address: PLACEHOLDER_ADDRESS.to_string(),
            adapter_deployed_block_height: Some(0),
            arpa_contract_address: None,
            data_path: None,
            account: Default::default(),
            listeners: Default::default(),
            logger: Default::default(),
            time_limits: Default::default(),
            relayed_chains: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayedChain {
    pub chain_id: usize,
    pub description: String,
    pub provider_endpoint: String,
    pub controller_oracle_address: String,
    pub adapter_address: String,
    adapter_deployed_block_height: Option<u64>,
    arpa_contract_address: Option<String>,
    pub listeners: Option<Vec<ListenerDescriptor>>,
    pub time_limits: Option<TimeLimitDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggerDescriptor {
    pub node_id: String,
    pub context_logging: bool,
    pub log_file_path: String,
    #[serde(deserialize_with = "deserialize_limit")]
    pub rolling_file_size: u64,
}

impl Default for LoggerDescriptor {
    fn default() -> Self {
        Self {
            node_id: "running".to_string(),
            context_logging: false,
            log_file_path: "log/running".to_string(),
            rolling_file_size: DEFAULT_ROLLING_LOG_FILE_SIZE,
        }
    }
}

fn deserialize_limit<'de, D>(d: D) -> Result<u64, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct V;

    impl<'de2> de::Visitor<'de2> for V {
        type Value = u64;

        fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            fmt.write_str("a size")
        }

        fn visit_u64<E>(self, v: u64) -> Result<u64, E>
        where
            E: de::Error,
        {
            Ok(v)
        }

        fn visit_i64<E>(self, v: i64) -> Result<u64, E>
        where
            E: de::Error,
        {
            if v < 0 {
                return Err(E::invalid_value(
                    de::Unexpected::Signed(v),
                    &"a non-negative number",
                ));
            }

            Ok(v as u64)
        }

        fn visit_str<E>(self, v: &str) -> Result<u64, E>
        where
            E: de::Error,
        {
            let (number, unit) = match v.find(|c: char| !c.is_ascii_digit()) {
                Some(n) => (v[..n].trim(), Some(v[n..].trim())),
                None => (v.trim(), None),
            };

            let number = match number.parse::<u64>() {
                Ok(n) => n,
                Err(_) => return Err(E::invalid_value(de::Unexpected::Str(number), &"a number")),
            };

            let unit = match unit {
                Some(u) => u,
                None => return Ok(number),
            };

            let number = if unit.eq_ignore_ascii_case("b") {
                Some(number)
            } else if unit.eq_ignore_ascii_case("kb") || unit.eq_ignore_ascii_case("kib") {
                number.checked_mul(1024)
            } else if unit.eq_ignore_ascii_case("mb") || unit.eq_ignore_ascii_case("mib") {
                number.checked_mul(1024 * 1024)
            } else if unit.eq_ignore_ascii_case("gb") || unit.eq_ignore_ascii_case("gib") {
                number.checked_mul(1024 * 1024 * 1024)
            } else if unit.eq_ignore_ascii_case("tb") || unit.eq_ignore_ascii_case("tib") {
                number.checked_mul(1024 * 1024 * 1024 * 1024)
            } else {
                return Err(E::invalid_value(de::Unexpected::Str(unit), &"a valid unit"));
            };

            match number {
                Some(n) => Ok(n),
                None => Err(E::invalid_value(de::Unexpected::Str(v), &"a byte size")),
            }
        }
    }

    d.deserialize_any(V)
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ListenerDescriptor {
    pub l_type: ListenerType,
    pub interval_millis: u64,
    pub use_jitter: bool,
}

impl ListenerDescriptor {
    pub fn build(l_type: ListenerType, interval_millis: u64) -> Self {
        Self {
            l_type,
            interval_millis,
            use_jitter: DEFAULT_LISTENER_USE_JITTER,
        }
    }

    pub fn default(l_type: ListenerType) -> Self {
        Self {
            l_type,
            interval_millis: DEFAULT_LISTENER_INTERVAL_MILLIS,
            use_jitter: DEFAULT_LISTENER_USE_JITTER,
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TimeLimitDescriptor {
    pub block_time: usize,
    pub listener_interval_millis: u64,
    pub dkg_wait_for_phase_interval_millis: Option<u64>,
    pub dkg_timeout_duration: Option<usize>,
    pub randomness_task_exclusive_window: usize,
    pub provider_polling_interval_millis: u64,
    pub contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    pub contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    pub commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ExponentialBackoffRetryDescriptor {
    pub base: u64,
    pub factor: u64,
    pub max_attempts: usize,
    pub use_jitter: bool,
}

impl Config {
    pub fn load(config_path: PathBuf) -> Config {
        let config_str = &read_to_string(config_path).unwrap_or_else(|e| {
            panic!(
                "Error loading configuration file: {:?}, please check the configuration!",
                e
            )
        });

        let config: Config =
            serde_yaml::from_str(config_str).expect("Error loading configuration file");

        config.initialize()
    }

    pub fn get_node_management_rpc_token(&self) -> Result<String, ConfigError> {
        if self.node_management_rpc_token.eq("env") {
            let token = env::var("ARPA_NODE_MANAGEMENT_SERVER_TOKEN")?;
            return Ok(token);
        }
        Ok(self.node_management_rpc_token.clone())
    }

    pub fn initialize(mut self) -> Self {
        if self.node_advertised_committer_rpc_endpoint.is_none() {
            self.node_advertised_committer_rpc_endpoint =
                Some(self.node_committer_rpc_endpoint.clone());
        }

        if self.data_path.is_none() {
            self.data_path = Some(String::from("data.sqlite"));
        }

        if self.logger.is_none() {
            self.logger = Some(LoggerDescriptor::default());
        }

        if self.listeners.is_none() {
            let listeners = vec![
                ListenerDescriptor::default(ListenerType::Block),
                ListenerDescriptor::default(ListenerType::PreGrouping),
                ListenerDescriptor::default(ListenerType::PostCommitGrouping),
                ListenerDescriptor::default(ListenerType::PostGrouping),
                ListenerDescriptor::default(ListenerType::NewRandomnessTask),
                ListenerDescriptor::default(ListenerType::ReadyToHandleRandomnessTask),
                ListenerDescriptor::default(ListenerType::RandomnessSignatureAggregation),
            ];
            self.listeners = Some(listeners);
        }

        if self.adapter_deployed_block_height.is_none() {
            self.adapter_deployed_block_height = Some(0);
        }

        match self.time_limits.as_mut() {
            Some(time_limits) if time_limits.listener_interval_millis == 0 => {
                time_limits.listener_interval_millis = DEFAULT_LISTENER_INTERVAL_MILLIS;
            }
            Some(time_limits)
                if time_limits.dkg_wait_for_phase_interval_millis == Some(0)
                    || time_limits.dkg_wait_for_phase_interval_millis.is_none() =>
            {
                time_limits.dkg_wait_for_phase_interval_millis =
                    Some(DEFAULT_DKG_WAIT_FOR_PHASE_INTERVAL_MILLIS);
            }
            Some(time_limits)
                if time_limits.dkg_timeout_duration == Some(0)
                    || time_limits.dkg_timeout_duration.is_none() =>
            {
                time_limits.dkg_timeout_duration = Some(DEFAULT_DKG_TIMEOUT_DURATION);
            }
            Some(time_limits) if time_limits.randomness_task_exclusive_window == 0 => {
                time_limits.randomness_task_exclusive_window =
                    DEFAULT_RANDOMNESS_TASK_EXCLUSIVE_WINDOW;
            }
            Some(time_limits) if time_limits.provider_polling_interval_millis == 0 => {
                time_limits.provider_polling_interval_millis =
                    DEFAULT_PROVIDER_POLLING_INTERVAL_MILLIS;
            }
            Some(_) => {}
            None => {
                self.time_limits = Some(TimeLimitDescriptor {
                    block_time: DEFAULT_BLOCK_TIME,
                    listener_interval_millis: DEFAULT_LISTENER_INTERVAL_MILLIS,
                    dkg_wait_for_phase_interval_millis: Some(
                        DEFAULT_DKG_WAIT_FOR_PHASE_INTERVAL_MILLIS,
                    ),
                    dkg_timeout_duration: Some(DEFAULT_DKG_TIMEOUT_DURATION),
                    randomness_task_exclusive_window: DEFAULT_RANDOMNESS_TASK_EXCLUSIVE_WINDOW,
                    provider_polling_interval_millis: DEFAULT_PROVIDER_POLLING_INTERVAL_MILLIS,
                    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor {
                        base: DEFAULT_CONTRACT_TRANSACTION_RETRY_BASE,
                        factor: DEFAULT_CONTRACT_TRANSACTION_RETRY_FACTOR,
                        max_attempts: DEFAULT_CONTRACT_TRANSACTION_RETRY_MAX_ATTEMPTS,
                        use_jitter: DEFAULT_CONTRACT_TRANSACTION_RETRY_USE_JITTER,
                    },
                    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor {
                        base: DEFAULT_CONTRACT_VIEW_RETRY_BASE,
                        factor: DEFAULT_CONTRACT_VIEW_RETRY_FACTOR,
                        max_attempts: DEFAULT_CONTRACT_VIEW_RETRY_MAX_ATTEMPTS,
                        use_jitter: DEFAULT_CONTRACT_VIEW_RETRY_USE_JITTER,
                    },
                    commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor {
                        base: DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_BASE,
                        factor: DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_FACTOR,
                        max_attempts: DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_MAX_ATTEMPTS,
                        use_jitter: DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_USE_JITTER,
                    },
                })
            }
        }

        for relayed_chain in self.relayed_chains.iter_mut() {
            relayed_chain.initialize();
        }

        self
    }

    pub fn main_chain_id(&self) -> usize {
        self.chain_id
    }

    pub fn relayed_chain_ids(&self) -> Vec<usize> {
        self.relayed_chains.iter().map(|c| c.chain_id).collect()
    }

    pub fn provider_endpoint(&self, chain_id: usize) -> anyhow::Result<String> {
        if chain_id == self.chain_id {
            Ok(self.provider_endpoint.clone())
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.provider_endpoint.clone())
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
        }
    }

    pub fn account(&self) -> Result<Wallet<SigningKey>, ConfigError> {
        build_wallet_from_config(&self.account)
    }

    pub fn arpa_address(&self, chain_id: usize) -> anyhow::Result<Address> {
        if chain_id == self.chain_id {
            self.arpa_contract_address
                .as_ref()
                .map(|a| a.parse().unwrap())
                .ok_or(ConfigError::LackOfARPAContractAddress.into())
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| {
                    c.arpa_contract_address
                        .as_ref()
                        .map(|a| a.parse().unwrap())
                        .ok_or(ConfigError::LackOfARPAContractAddress.into())
                })
                .unwrap_or_else(|| Err(ConfigError::InvalidChainId(chain_id).into()))
        }
    }

    pub fn adapter_address(&self, chain_id: usize) -> anyhow::Result<Address> {
        if chain_id == self.chain_id {
            Ok(self.adapter_address.parse().unwrap())
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.adapter_address.parse().unwrap())
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
        }
    }

    pub fn adapter_deployed_block_height(&self, chain_id: usize) -> anyhow::Result<u64> {
        if chain_id == self.chain_id {
            Ok(self.adapter_deployed_block_height.unwrap())
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.adapter_deployed_block_height.unwrap())
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
        }
    }

    pub fn contract_transaction_retry_descriptor(
        &self,
        chain_id: usize,
    ) -> anyhow::Result<ExponentialBackoffRetryDescriptor> {
        if chain_id == self.chain_id {
            Ok(self
                .time_limits
                .unwrap()
                .contract_transaction_retry_descriptor)
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.time_limits.unwrap().contract_transaction_retry_descriptor)
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
        }
    }

    pub fn contract_view_retry_descriptor(
        &self,
        chain_id: usize,
    ) -> anyhow::Result<ExponentialBackoffRetryDescriptor> {
        if chain_id == self.chain_id {
            Ok(self.time_limits.unwrap().contract_view_retry_descriptor)
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.time_limits.unwrap().contract_view_retry_descriptor)
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
        }
    }
}

impl RelayedChain {
    pub fn initialize(&mut self) {
        if self.adapter_deployed_block_height.is_none() {
            self.adapter_deployed_block_height = Some(0);
        }

        if self.listeners.is_none() {
            let listeners = vec![
                ListenerDescriptor::default(ListenerType::Block),
                ListenerDescriptor::default(ListenerType::NewRandomnessTask),
                ListenerDescriptor::default(ListenerType::ReadyToHandleRandomnessTask),
                ListenerDescriptor::default(ListenerType::RandomnessSignatureAggregation),
            ];
            self.listeners = Some(listeners);
        }

        match self.time_limits.as_mut() {
            Some(time_limits) if time_limits.listener_interval_millis == 0 => {
                time_limits.listener_interval_millis = DEFAULT_LISTENER_INTERVAL_MILLIS;
            }
            Some(time_limits) if time_limits.randomness_task_exclusive_window == 0 => {
                time_limits.randomness_task_exclusive_window =
                    DEFAULT_RANDOMNESS_TASK_EXCLUSIVE_WINDOW;
            }
            Some(time_limits) if time_limits.provider_polling_interval_millis == 0 => {
                time_limits.provider_polling_interval_millis =
                    DEFAULT_PROVIDER_POLLING_INTERVAL_MILLIS;
            }
            Some(_) => {}
            None => {
                self.time_limits = Some(TimeLimitDescriptor {
                    block_time: DEFAULT_BLOCK_TIME,
                    listener_interval_millis: DEFAULT_LISTENER_INTERVAL_MILLIS,
                    dkg_wait_for_phase_interval_millis: None,
                    dkg_timeout_duration: None,
                    randomness_task_exclusive_window: DEFAULT_RANDOMNESS_TASK_EXCLUSIVE_WINDOW,
                    provider_polling_interval_millis: DEFAULT_PROVIDER_POLLING_INTERVAL_MILLIS,
                    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor {
                        base: DEFAULT_CONTRACT_TRANSACTION_RETRY_BASE,
                        factor: DEFAULT_CONTRACT_TRANSACTION_RETRY_FACTOR,
                        max_attempts: DEFAULT_CONTRACT_TRANSACTION_RETRY_MAX_ATTEMPTS,
                        use_jitter: DEFAULT_CONTRACT_TRANSACTION_RETRY_USE_JITTER,
                    },
                    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor {
                        base: DEFAULT_CONTRACT_VIEW_RETRY_BASE,
                        factor: DEFAULT_CONTRACT_VIEW_RETRY_FACTOR,
                        max_attempts: DEFAULT_CONTRACT_VIEW_RETRY_MAX_ATTEMPTS,
                        use_jitter: DEFAULT_CONTRACT_VIEW_RETRY_USE_JITTER,
                    },
                    commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor {
                        base: DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_BASE,
                        factor: DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_FACTOR,
                        max_attempts: DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_MAX_ATTEMPTS,
                        use_jitter: DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_USE_JITTER,
                    },
                })
            }
        }
    }
}

#[derive(Debug, Eq, Clone, Copy, Hash, PartialEq)]
pub enum TaskType {
    Listener(usize, ListenerType),
    Subscriber(usize, SubscriberType),
    RpcServer(RpcServerType),
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TaskType::Listener(id, l) => f
                .debug_struct("TaskType")
                .field("chain_id", id)
                .field("listener", l)
                .finish(),
            TaskType::Subscriber(id, s) => f
                .debug_struct("TaskType")
                .field("chain_id", id)
                .field("subscriber", s)
                .finish(),
            TaskType::RpcServer(r) => f.debug_struct("TaskType").field("rpc server", r).finish(),
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum ListenerType {
    Block,
    PreGrouping,
    PostCommitGrouping,
    PostGrouping,
    NewRandomnessTask,
    ReadyToHandleRandomnessTask,
    RandomnessSignatureAggregation,
}

impl TryFrom<i32> for ListenerType {
    type Error = SchedulerError;

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(ListenerType::Block),
            1 => Ok(ListenerType::PreGrouping),
            2 => Ok(ListenerType::PostCommitGrouping),
            3 => Ok(ListenerType::PostGrouping),
            4 => Ok(ListenerType::NewRandomnessTask),
            5 => Ok(ListenerType::ReadyToHandleRandomnessTask),
            6 => Ok(ListenerType::RandomnessSignatureAggregation),
            _ => Err(SchedulerError::TaskNotFound),
        }
    }
}

impl std::fmt::Display for ListenerType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ListenerType::Block => write!(f, "Block"),
            ListenerType::PreGrouping => write!(f, "PreGrouping"),
            ListenerType::PostGrouping => write!(f, "PostGrouping"),
            ListenerType::ReadyToHandleRandomnessTask => write!(f, "ReadyToHandleRandomnessTask"),
            ListenerType::RandomnessSignatureAggregation => {
                write!(f, "RandomnessSignatureAggregation")
            }
            ListenerType::PostCommitGrouping => write!(f, "PostCommitGrouping"),
            ListenerType::NewRandomnessTask => write!(f, "NewRandomnessTask"),
        }
    }
}

#[derive(Debug, Eq, Clone, Copy, Hash, PartialEq)]
pub enum SubscriberType {
    Block,
    PreGrouping,
    InGrouping,
    PostSuccessGrouping,
    PostGrouping,
    ReadyToHandleRandomnessTask,
    RandomnessSignatureAggregation,
    SendingPartialSignature,
}

impl std::fmt::Display for SubscriberType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SubscriberType::Block => write!(f, "Block"),
            SubscriberType::PreGrouping => write!(f, "PreGrouping"),
            SubscriberType::InGrouping => write!(f, "InGrouping"),
            SubscriberType::PostSuccessGrouping => write!(f, "PostSuccessGrouping"),
            SubscriberType::PostGrouping => write!(f, "PostGrouping"),
            SubscriberType::ReadyToHandleRandomnessTask => write!(f, "ReadyToHandleRandomnessTask"),
            SubscriberType::RandomnessSignatureAggregation => {
                write!(f, "RandomnessSignatureAggregation")
            }
            SubscriberType::SendingPartialSignature => write!(f, "SendingPartialSignature"),
        }
    }
}

#[derive(Debug, Eq, Clone, Copy, Hash, PartialEq)]
pub enum RpcServerType {
    Committer,
    Management,
}

impl std::fmt::Display for RpcServerType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RpcServerType::Committer => write!(f, "Committer"),
            RpcServerType::Management => write!(f, "Management"),
        }
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
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
    use std::{
        fs::{self, read_to_string},
        io::Write,
        time::Duration,
    };

    use crate::{jitter, Config, ListenerType};

    #[test]
    fn test_enum_serialization() {
        let listener_type = ListenerType::Block;
        let serialize = serde_json::to_string(&listener_type).unwrap();
        println!("serialize = {}", serialize);
    }

    #[test]
    fn test_read_from_config_example() {
        let config_str = &read_to_string("../arpa-node/test/conf/config_test_3.yml")
            .unwrap_or_else(|e| {
                panic!(
                    "Error loading configuration file: {:?}, please check the configuration!",
                    e
                )
            });

        let config: Config =
            serde_yaml::from_str(config_str).expect("Error loading configuration file");

        println!("config = {:#?}", config.initialize());
    }

    #[test]
    fn test_deserialization_from_config() {
        let config = Config::default();

        let mut file = fs::File::create("config.yml").unwrap();
        file.write_all(serde_yaml::to_string(&config).unwrap().as_bytes())
            .unwrap();

        let config_str = &read_to_string("config.yml").unwrap_or_else(|e| {
            panic!(
                "Error loading configuration file: {:?}, please check the configuration!",
                e
            )
        });

        let config: Config =
            serde_yaml::from_str(config_str).expect("Error loading configuration file");

        println!("config = {:#?}", config.initialize());

        fs::remove_file("config.yml").unwrap();
    }

    #[test]
    fn test_jitter() {
        for _ in 0..100 {
            let jitter = jitter(Duration::from_millis(1000));
            // println!("jitter = {:?}", jitter(Duration::from_millis(1000)));
            assert!(500 <= jitter.as_millis() && jitter.as_millis() <= 1000);
        }
    }
}
