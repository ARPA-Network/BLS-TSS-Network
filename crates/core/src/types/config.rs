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

pub const DEFAULT_PROVIDER_RESET_INTERVAL_MILLIS: u64 = 5000;
pub const DEFAULT_PROVIDER_RESET_MAX_ATTEMPTS: usize = 17280;
pub const DEFAULT_PROVIDER_RESET_USE_JITTER: bool = true;

pub const DEFAULT_PROVIDER_POLLING_INTERVAL_MILLIS: u64 = 10000;

pub const DEFAULT_DYNAMIC_TASK_CLEANER_INTERVAL_MILLIS: u64 = 1000;

pub const FULFILL_RANDOMNESS_GAS_EXCEPT_CALLBACK: u32 = 670000;
pub const RANDOMNESS_REWARD_GAS: u32 = 9000;
pub const VERIFICATION_GAS_OVER_MINIMUM_THRESHOLD: u32 = 50000;
pub const DEFAULT_MINIMUM_THRESHOLD: u32 = 3;

pub const DEFAULT_ROLLING_LOG_FILE_SIZE: u64 = 10 * 1024 * 1024 * 1024;

pub const DEFAULT_BLOCK_TIME: usize = 12;
pub const DEFAULT_MAX_RANDOMNESS_FULFILLMENT_ATTEMPTS: usize = 3;

pub const DEFAULT_WEBSOCKET_PROVIDER_RECONNECT_TIMES: usize = 1000000;

pub fn jitter(duration: Duration) -> Duration {
    duration.mul_f64(thread_rng().gen_range(0.5..=1.0))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigHolder {
    pub node_committer_rpc_endpoint: String,
    pub node_advertised_committer_rpc_endpoint: Option<String>,
    pub node_management_rpc_endpoint: String,
    pub node_management_rpc_token: String,
    pub provider_endpoint: String,
    pub chain_id: usize,
    pub controller_address: String,
    pub controller_relayer_address: String,
    pub adapter_address: String,
    pub adapter_deployed_block_height: Option<u64>,
    pub arpa_contract_address: Option<String>,
    // Data file for persistence
    pub data_path: Option<String>,
    pub account: Account,
    pub listeners: Option<Vec<ListenerDescriptorHolder>>,
    pub logger: Option<LoggerDescriptor>,
    pub time_limits: Option<TimeLimitDescriptorHolder>,
    pub relayed_chains: Vec<RelayedChainHolder>,
}

impl Default for ConfigHolder {
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
pub struct LoggerDescriptor {
    node_id: String,
    context_logging: bool,
    log_file_path: String,
    #[serde(deserialize_with = "deserialize_limit")]
    rolling_file_size: u64,
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

impl LoggerDescriptor {
    pub fn get_node_id(&self) -> &str {
        &self.node_id
    }

    pub fn get_context_logging(&self) -> bool {
        self.context_logging
    }

    pub fn get_log_file_path(&self) -> &str {
        &self.log_file_path
    }

    pub fn get_rolling_file_size(&self) -> u64 {
        self.rolling_file_size
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
struct ListenerDescriptorHolder {
    pub l_type: ListenerType,
    pub interval_millis: u64,
    pub use_jitter: bool,
    pub reset_descriptor: Option<FixedIntervalRetryDescriptor>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ListenerDescriptor {
    pub l_type: ListenerType,
    pub interval_millis: u64,
    pub use_jitter: bool,
    pub reset_descriptor: FixedIntervalRetryDescriptor,
}

impl ListenerDescriptor {
    fn from(
        listener_descriptor_holder: ListenerDescriptorHolder,
        provider_reset_descriptor: FixedIntervalRetryDescriptor,
    ) -> Self {
        let l_type = listener_descriptor_holder.l_type;
        let interval_millis = listener_descriptor_holder.interval_millis;
        let use_jitter = listener_descriptor_holder.use_jitter;
        let reset_descriptor = listener_descriptor_holder
            .reset_descriptor
            .unwrap_or(provider_reset_descriptor);

        Self {
            l_type,
            interval_millis,
            use_jitter,
            reset_descriptor,
        }
    }

    fn build(
        l_type: ListenerType,
        interval_millis: u64,
        reset_descriptor: FixedIntervalRetryDescriptor,
    ) -> Self {
        Self {
            l_type,
            interval_millis,
            use_jitter: DEFAULT_LISTENER_USE_JITTER,
            reset_descriptor,
        }
    }

    pub fn default(l_type: ListenerType) -> Self {
        Self {
            l_type,
            interval_millis: DEFAULT_LISTENER_INTERVAL_MILLIS,
            use_jitter: DEFAULT_LISTENER_USE_JITTER,
            reset_descriptor: FixedIntervalRetryDescriptor {
                interval_millis: DEFAULT_PROVIDER_RESET_INTERVAL_MILLIS,
                max_attempts: DEFAULT_PROVIDER_RESET_MAX_ATTEMPTS,
                use_jitter: DEFAULT_PROVIDER_RESET_USE_JITTER,
            },
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TimeLimitDescriptorHolder {
    pub block_time: usize,
    pub listener_interval_millis: u64,
    pub dkg_wait_for_phase_interval_millis: Option<u64>,
    pub dkg_timeout_duration: Option<usize>,
    pub randomness_task_exclusive_window: usize,
    pub provider_polling_interval_millis: u64,
    pub provider_reset_descriptor: FixedIntervalRetryDescriptor,
    pub contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    pub contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    pub commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TimeLimitDescriptor {
    pub block_time: usize,
    pub listener_interval_millis: u64,
    pub dkg_wait_for_phase_interval_millis: u64,
    pub dkg_timeout_duration: usize,
    pub randomness_task_exclusive_window: usize,
    pub provider_polling_interval_millis: u64,
    pub provider_reset_descriptor: FixedIntervalRetryDescriptor,
    pub contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    pub contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    pub commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl Default for TimeLimitDescriptor {
    fn default() -> Self {
        TimeLimitDescriptor {
            block_time: DEFAULT_BLOCK_TIME,
            listener_interval_millis: DEFAULT_LISTENER_INTERVAL_MILLIS,
            dkg_wait_for_phase_interval_millis: DEFAULT_DKG_WAIT_FOR_PHASE_INTERVAL_MILLIS,
            dkg_timeout_duration: DEFAULT_DKG_TIMEOUT_DURATION,
            randomness_task_exclusive_window: DEFAULT_RANDOMNESS_TASK_EXCLUSIVE_WINDOW,
            provider_polling_interval_millis: DEFAULT_PROVIDER_POLLING_INTERVAL_MILLIS,
            provider_reset_descriptor: FixedIntervalRetryDescriptor {
                interval_millis: DEFAULT_PROVIDER_RESET_INTERVAL_MILLIS,
                max_attempts: DEFAULT_PROVIDER_RESET_MAX_ATTEMPTS,
                use_jitter: DEFAULT_PROVIDER_RESET_USE_JITTER,
            },
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
        }
    }
}

impl From<TimeLimitDescriptorHolder> for TimeLimitDescriptor {
    fn from(time_limit_descriptor_holder: TimeLimitDescriptorHolder) -> Self {
        let block_time = time_limit_descriptor_holder.block_time;
        let listener_interval_millis = if time_limit_descriptor_holder.listener_interval_millis == 0
        {
            DEFAULT_LISTENER_INTERVAL_MILLIS
        } else {
            time_limit_descriptor_holder.listener_interval_millis
        };
        let dkg_wait_for_phase_interval_millis =
            match time_limit_descriptor_holder.dkg_wait_for_phase_interval_millis {
                None => DEFAULT_DKG_WAIT_FOR_PHASE_INTERVAL_MILLIS,
                Some(0) => DEFAULT_DKG_WAIT_FOR_PHASE_INTERVAL_MILLIS,
                Some(v) => v,
            };

        let dkg_timeout_duration = match time_limit_descriptor_holder.dkg_timeout_duration {
            None => DEFAULT_DKG_TIMEOUT_DURATION,
            Some(0) => DEFAULT_DKG_TIMEOUT_DURATION,
            Some(v) => v,
        };
        let randomness_task_exclusive_window =
            if time_limit_descriptor_holder.randomness_task_exclusive_window == 0 {
                DEFAULT_RANDOMNESS_TASK_EXCLUSIVE_WINDOW
            } else {
                time_limit_descriptor_holder.randomness_task_exclusive_window
            };
        let provider_polling_interval_millis =
            if time_limit_descriptor_holder.provider_polling_interval_millis == 0 {
                DEFAULT_PROVIDER_POLLING_INTERVAL_MILLIS
            } else {
                time_limit_descriptor_holder.provider_polling_interval_millis
            };
        let provider_reset_descriptor = time_limit_descriptor_holder.provider_reset_descriptor;
        let contract_transaction_retry_descriptor =
            time_limit_descriptor_holder.contract_transaction_retry_descriptor;
        let contract_view_retry_descriptor =
            time_limit_descriptor_holder.contract_view_retry_descriptor;
        let commit_partial_signature_retry_descriptor =
            time_limit_descriptor_holder.commit_partial_signature_retry_descriptor;

        TimeLimitDescriptor {
            block_time,
            listener_interval_millis,
            dkg_wait_for_phase_interval_millis,
            dkg_timeout_duration,
            randomness_task_exclusive_window,
            provider_polling_interval_millis,
            provider_reset_descriptor,
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
            commit_partial_signature_retry_descriptor,
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct FixedIntervalRetryDescriptor {
    pub interval_millis: u64,
    pub max_attempts: usize,
    pub use_jitter: bool,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ExponentialBackoffRetryDescriptor {
    pub base: u64,
    pub factor: u64,
    pub max_attempts: usize,
    pub use_jitter: bool,
}

#[derive(Debug, Clone)]
pub struct Config {
    node_committer_rpc_endpoint: String,
    node_advertised_committer_rpc_endpoint: String,
    node_management_rpc_endpoint: String,
    node_management_rpc_token: String,
    provider_endpoint: String,
    chain_id: usize,
    controller_address: String,
    controller_relayer_address: String,
    adapter_address: String,
    adapter_deployed_block_height: u64,
    arpa_contract_address: String,
    // Data file for persistence
    data_path: String,
    account: Account,
    listeners: Vec<ListenerDescriptor>,
    logger: LoggerDescriptor,
    time_limits: TimeLimitDescriptor,
    relayed_chains: Vec<RelayedChain>,
}

impl From<ConfigHolder> for Config {
    fn from(config_holder: ConfigHolder) -> Self {
        let node_committer_rpc_endpoint = config_holder.node_committer_rpc_endpoint.clone();
        let node_advertised_committer_rpc_endpoint = if config_holder
            .node_advertised_committer_rpc_endpoint
            .is_none()
        {
            config_holder.node_committer_rpc_endpoint.clone()
        } else {
            config_holder
                .node_advertised_committer_rpc_endpoint
                .clone()
                .unwrap()
        };
        let node_management_rpc_endpoint = config_holder.node_management_rpc_endpoint.clone();
        let node_management_rpc_token = if config_holder.node_management_rpc_token.eq("env") {
            env::var("ARPA_NODE_MANAGEMENT_SERVER_TOKEN").unwrap()
        } else {
            config_holder.node_management_rpc_token.clone()
        };
        let provider_endpoint = config_holder.provider_endpoint.clone();
        let chain_id = config_holder.chain_id;
        let controller_address = config_holder.controller_address.clone();
        let controller_relayer_address = config_holder.controller_relayer_address.clone();
        let adapter_address = config_holder.adapter_address.clone();
        let adapter_deployed_block_height = if config_holder.adapter_deployed_block_height.is_none()
        {
            0
        } else {
            config_holder.adapter_deployed_block_height.unwrap()
        };
        let arpa_contract_address = if config_holder.arpa_contract_address.is_none() {
            String::from("")
        } else {
            config_holder.arpa_contract_address.unwrap()
        };
        let data_path = if config_holder.data_path.is_none() {
            String::from("data.sqlite")
        } else {
            config_holder.data_path.unwrap()
        };
        let account = config_holder.account.clone();
        let logger = if config_holder.logger.is_none() {
            LoggerDescriptor::default()
        } else {
            config_holder.logger.unwrap()
        };
        let time_limits = if config_holder.time_limits.is_none() {
            TimeLimitDescriptor::default()
        } else {
            config_holder.time_limits.unwrap().into()
        };
        let listeners = if config_holder.listeners.is_none() {
            vec![
                ListenerDescriptor::build(
                    ListenerType::Block,
                    time_limits.listener_interval_millis,
                    time_limits.provider_reset_descriptor,
                ),
                ListenerDescriptor::build(
                    ListenerType::PreGrouping,
                    time_limits.listener_interval_millis,
                    time_limits.provider_reset_descriptor,
                ),
                ListenerDescriptor::build(
                    ListenerType::PostCommitGrouping,
                    time_limits.listener_interval_millis,
                    time_limits.provider_reset_descriptor,
                ),
                ListenerDescriptor::build(
                    ListenerType::PostGrouping,
                    time_limits.listener_interval_millis,
                    time_limits.provider_reset_descriptor,
                ),
                ListenerDescriptor::build(
                    ListenerType::NewRandomnessTask,
                    time_limits.listener_interval_millis,
                    time_limits.provider_reset_descriptor,
                ),
                ListenerDescriptor::build(
                    ListenerType::ReadyToHandleRandomnessTask,
                    time_limits.listener_interval_millis,
                    time_limits.provider_reset_descriptor,
                ),
                ListenerDescriptor::build(
                    ListenerType::RandomnessSignatureAggregation,
                    time_limits.listener_interval_millis,
                    time_limits.provider_reset_descriptor,
                ),
            ]
        } else {
            config_holder
                .listeners
                .map(|l| {
                    l.iter()
                        .map(|l| {
                            ListenerDescriptor::from(*l, time_limits.provider_reset_descriptor)
                        })
                        .collect()
                })
                .unwrap()
        };

        let relayed_chains = config_holder
            .relayed_chains
            .into_iter()
            .map(|c| c.into())
            .collect();

        Self {
            node_committer_rpc_endpoint,
            node_advertised_committer_rpc_endpoint,
            node_management_rpc_endpoint,
            node_management_rpc_token,
            provider_endpoint,
            chain_id,
            controller_address,
            controller_relayer_address,
            adapter_address,
            adapter_deployed_block_height,
            arpa_contract_address,
            data_path,
            account,
            listeners,
            logger,
            time_limits,
            relayed_chains,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        ConfigHolder::default().into()
    }
}

impl Config {
    pub fn load(config_path: PathBuf) -> Config {
        let config_str = &read_to_string(config_path).unwrap_or_else(|e| {
            panic!(
                "Error loading configuration file: {:?}, please check the configuration!",
                e
            )
        });

        let config: ConfigHolder =
            serde_yaml::from_str(config_str).expect("Error loading configuration file");

        config.into()
    }

    pub fn get_main_chain_id(&self) -> usize {
        self.chain_id
    }

    pub fn get_relayed_chain_ids(&self) -> Vec<usize> {
        self.relayed_chains.iter().map(|c| c.chain_id).collect()
    }

    pub fn get_node_committer_rpc_endpoint(&self) -> &str {
        &self.node_committer_rpc_endpoint
    }

    pub fn get_node_advertised_committer_rpc_endpoint(&self) -> &str {
        &self.node_advertised_committer_rpc_endpoint
    }

    pub fn get_node_management_rpc_endpoint(&self) -> &str {
        &self.node_management_rpc_endpoint
    }

    pub fn get_node_management_rpc_token(&self) -> &str {
        &self.node_management_rpc_token
    }

    pub fn get_provider_endpoint(&self) -> &str {
        &self.provider_endpoint
    }

    pub fn get_controller_address(&self) -> &str {
        &self.controller_address
    }

    pub fn get_controller_relayer_address(&self) -> &str {
        &self.controller_relayer_address
    }

    pub fn get_adapter_address(&self) -> &str {
        &self.adapter_address
    }

    pub fn get_adapter_deployed_block_height(&self) -> u64 {
        self.adapter_deployed_block_height
    }

    pub fn get_arpa_contract_address(&self) -> &str {
        &self.arpa_contract_address
    }

    pub fn get_account(&self) -> &Account {
        &self.account
    }

    pub fn find_provider_endpoint(&self, chain_id: usize) -> anyhow::Result<String> {
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

    pub fn find_controller_address(&self, chain_id: usize) -> anyhow::Result<Address> {
        if chain_id == self.chain_id {
            Ok(self.controller_address.parse()?)
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.controller_oracle_address.parse().unwrap())
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
        }
    }

    pub fn find_controller_relayer_address(&self, chain_id: usize) -> anyhow::Result<Address> {
        if chain_id == self.chain_id {
            Ok(self.controller_relayer_address.parse()?)
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.controller_oracle_address.parse().unwrap())
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
        }
    }

    pub fn find_arpa_address(&self, chain_id: usize) -> anyhow::Result<Address> {
        if chain_id == self.chain_id {
            if self.arpa_contract_address.is_empty() {
                return Err(ConfigError::LackOfARPAContractAddress.into());
            }
            Ok(self.arpa_contract_address.parse()?)
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| {
                    if c.arpa_contract_address.is_empty() {
                        return Err(ConfigError::LackOfARPAContractAddress.into());
                    }
                    Ok(c.arpa_contract_address.parse().unwrap())
                })
                .unwrap_or_else(|| Err(ConfigError::InvalidChainId(chain_id).into()))
        }
    }

    pub fn find_adapter_address(&self, chain_id: usize) -> anyhow::Result<Address> {
        if chain_id == self.chain_id {
            Ok(self.adapter_address.parse()?)
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.adapter_address.parse().unwrap())
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
        }
    }

    pub fn find_adapter_deployed_block_height(&self, chain_id: usize) -> anyhow::Result<u64> {
        if chain_id == self.chain_id {
            Ok(self.adapter_deployed_block_height)
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.adapter_deployed_block_height)
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
        }
    }

    pub fn get_data_path(&self) -> &str {
        &self.data_path
    }

    pub fn get_listeners(&self) -> &Vec<ListenerDescriptor> {
        &self.listeners
    }

    pub fn get_logger_descriptor(&self) -> &LoggerDescriptor {
        &self.logger
    }

    pub fn get_time_limits(&self) -> &TimeLimitDescriptor {
        &self.time_limits
    }

    pub fn get_relayed_chains(&self) -> &Vec<RelayedChain> {
        &self.relayed_chains
    }

    pub fn contract_transaction_retry_descriptor(
        &self,
        chain_id: usize,
    ) -> anyhow::Result<ExponentialBackoffRetryDescriptor> {
        if chain_id == self.chain_id {
            Ok(self.time_limits.contract_transaction_retry_descriptor)
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.time_limits.contract_transaction_retry_descriptor)
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
        }
    }

    pub fn contract_view_retry_descriptor(
        &self,
        chain_id: usize,
    ) -> anyhow::Result<ExponentialBackoffRetryDescriptor> {
        if chain_id == self.chain_id {
            Ok(self.time_limits.contract_view_retry_descriptor)
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.time_limits.contract_view_retry_descriptor)
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelayedChainHolder {
    pub chain_id: usize,
    pub description: String,
    pub provider_endpoint: String,
    pub controller_oracle_address: String,
    pub adapter_address: String,
    pub adapter_deployed_block_height: Option<u64>,
    pub arpa_contract_address: Option<String>,
    pub listeners: Option<Vec<ListenerDescriptorHolder>>,
    pub time_limits: Option<TimeLimitDescriptorHolder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayedChain {
    chain_id: usize,
    description: String,
    provider_endpoint: String,
    controller_oracle_address: String,
    adapter_address: String,
    adapter_deployed_block_height: u64,
    arpa_contract_address: String,
    listeners: Vec<ListenerDescriptor>,
    time_limits: TimeLimitDescriptor,
}

impl From<RelayedChainHolder> for RelayedChain {
    fn from(relayed_chain_holder: RelayedChainHolder) -> Self {
        let chain_id = relayed_chain_holder.chain_id;
        let description = relayed_chain_holder.description;
        let provider_endpoint = relayed_chain_holder.provider_endpoint;
        let controller_oracle_address = relayed_chain_holder.controller_oracle_address;
        let adapter_address = relayed_chain_holder.adapter_address;
        let adapter_deployed_block_height =
            if relayed_chain_holder.adapter_deployed_block_height.is_none() {
                0
            } else {
                relayed_chain_holder.adapter_deployed_block_height.unwrap()
            };
        let arpa_contract_address = if relayed_chain_holder.arpa_contract_address.is_none() {
            String::from("")
        } else {
            relayed_chain_holder.arpa_contract_address.unwrap()
        };

        let time_limits = if relayed_chain_holder.time_limits.is_none() {
            TimeLimitDescriptor::default()
        } else {
            relayed_chain_holder.time_limits.unwrap().into()
        };

        let listeners = if relayed_chain_holder.listeners.is_none() {
            vec![
                ListenerDescriptor::build(
                    ListenerType::Block,
                    time_limits.listener_interval_millis,
                    time_limits.provider_reset_descriptor,
                ),
                ListenerDescriptor::build(
                    ListenerType::NewRandomnessTask,
                    time_limits.listener_interval_millis,
                    time_limits.provider_reset_descriptor,
                ),
                ListenerDescriptor::build(
                    ListenerType::ReadyToHandleRandomnessTask,
                    time_limits.listener_interval_millis,
                    time_limits.provider_reset_descriptor,
                ),
                ListenerDescriptor::build(
                    ListenerType::RandomnessSignatureAggregation,
                    time_limits.listener_interval_millis,
                    time_limits.provider_reset_descriptor,
                ),
            ]
        } else {
            relayed_chain_holder
                .listeners
                .map(|l| {
                    l.iter()
                        .map(|l| {
                            ListenerDescriptor::from(*l, time_limits.provider_reset_descriptor)
                        })
                        .collect()
                })
                .unwrap()
        };

        Self {
            chain_id,
            description,
            provider_endpoint,
            controller_oracle_address,
            adapter_address,
            adapter_deployed_block_height,
            arpa_contract_address,
            listeners,
            time_limits,
        }
    }
}

impl RelayedChain {
    pub fn get_chain_id(&self) -> usize {
        self.chain_id
    }

    pub fn get_description(&self) -> &str {
        &self.description
    }

    pub fn get_provider_endpoint(&self) -> &str {
        &self.provider_endpoint
    }

    pub fn get_controller_oracle_address(&self) -> &str {
        &self.controller_oracle_address
    }

    pub fn get_adapter_address(&self) -> &str {
        &self.adapter_address
    }

    pub fn get_adapter_deployed_block_height(&self) -> u64 {
        self.adapter_deployed_block_height
    }

    pub fn get_arpa_contract_address(&self) -> &str {
        &self.arpa_contract_address
    }

    pub fn get_listeners(&self) -> &Vec<ListenerDescriptor> {
        &self.listeners
    }

    pub fn get_time_limits(&self) -> &TimeLimitDescriptor {
        &self.time_limits
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

    use crate::{jitter, types::config::ConfigHolder, Config, ListenerType};

    #[test]
    fn test_enum_serialization() {
        let listener_type = ListenerType::Block;
        let serialize = serde_json::to_string(&listener_type).unwrap();
        println!("serialize = {}", serialize);
    }

    #[test]
    fn test_read_from_config_example() {
        let config_str = &read_to_string("../arpa-node/conf/config.yml").unwrap_or_else(|e| {
            panic!(
                "Error loading configuration file: {:?}, please check the configuration!",
                e
            )
        });

        let config: ConfigHolder =
            serde_yaml::from_str(config_str).expect("Error loading configuration file");

        println!("config = {:#?}", Config::from(config));
    }

    #[test]
    fn test_deserialization_from_config() {
        let config_holder = ConfigHolder::default();

        let mut file = fs::File::create("config.yml").unwrap();
        file.write_all(serde_yaml::to_string(&config_holder).unwrap().as_bytes())
            .unwrap();

        let config_str = &read_to_string("config.yml").unwrap_or_else(|e| {
            panic!(
                "Error loading configuration file: {:?}, please check the configuration!",
                e
            )
        });

        let config: ConfigHolder =
            serde_yaml::from_str(config_str).expect("Error loading configuration file");

        println!("config = {:#?}", Config::from(config));

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
