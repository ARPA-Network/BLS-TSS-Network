use arpa_core::{
    ExponentialBackoffRetryDescriptor, DEFAULT_CONTRACT_TRANSACTION_RETRY_BASE,
    DEFAULT_CONTRACT_TRANSACTION_RETRY_FACTOR, DEFAULT_CONTRACT_TRANSACTION_RETRY_MAX_ATTEMPTS,
    DEFAULT_CONTRACT_TRANSACTION_RETRY_USE_JITTER, DEFAULT_CONTRACT_VIEW_RETRY_BASE,
    DEFAULT_CONTRACT_VIEW_RETRY_FACTOR, DEFAULT_CONTRACT_VIEW_RETRY_MAX_ATTEMPTS,
    DEFAULT_CONTRACT_VIEW_RETRY_USE_JITTER,
};
use ethers::core::k256::ecdsa::SigningKey;
use ethers::signers::WalletError;
use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Wallet};
use ethers::types::Address;
use serde::{Deserialize, Serialize};
use std::env::{self, VarError};
use std::{fs::read_to_string, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    provider_endpoint: String,
    chain_id: u32,
    adapter_address: String,
    adapter_deployed_block_height: u64,
    staking_address: String,
    arpa_address: String,
    account: Account,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
    relayed_chains: Vec<RelayedChain>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider_endpoint: "localhost:8545".to_string(),
            chain_id: 0,
            adapter_address: "0xa513e6e4b8f2a923d98304ec87f64353c4d5c853".to_string(),
            adapter_deployed_block_height: 0,
            staking_address: "0x5f3b5dfeb7b28cdbd7faba78963ee202a494e2a2".to_string(),
            arpa_address: "0x6f769e65c14ebd1f68817f5f1dcdbd5c5d5f6e6a".to_string(),
            account: Default::default(),
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
            relayed_chains: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayedChain {
    chain_id: u32,
    provider_endpoint: String,
    adapter_address: String,
    adapter_deployed_block_height: u64,
    arpa_address: String,
    account: Account,
    contract_transaction_retry_descriptor: ExponentialBackoffRetryDescriptor,
    contract_view_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl Config {
    pub fn load(config_path: PathBuf) -> Config {
        let config_str = &read_to_string(config_path).unwrap_or_else(|e| {
            panic!(
                "Error loading configuration file: {:?}, please check the configuration!",
                e
            )
        });

        serde_yaml::from_str(config_str).expect("Error loading configuration file")
    }

    pub fn main_chain_id(&self) -> u32 {
        self.chain_id
    }

    pub fn relayed_chain_ids(&self) -> Vec<u32> {
        self.relayed_chains.iter().map(|c| c.chain_id).collect()
    }

    pub fn provider_endpoint(&self, chain_id: u32) -> anyhow::Result<String> {
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

    pub fn account(&self, chain_id: u32) -> anyhow::Result<Wallet<SigningKey>> {
        if chain_id == self.chain_id {
            build_wallet_from_config(&self.account)
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| build_wallet_from_config(&c.account))
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id))?
        }
    }

    pub fn adapter_address(&self, chain_id: u32) -> anyhow::Result<Address> {
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

    pub fn adapter_deployed_block_height(&self, chain_id: u32) -> anyhow::Result<u64> {
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

    pub fn staking_address(&self) -> Address {
        self.staking_address.parse().unwrap()
    }

    pub fn arpa_address(&self, chain_id: u32) -> anyhow::Result<Address> {
        if chain_id == self.chain_id {
            Ok(self.arpa_address.parse().unwrap())
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.arpa_address.parse().unwrap())
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
        }
    }

    pub fn contract_transaction_retry_descriptor(
        &self,
        chain_id: u32,
    ) -> anyhow::Result<ExponentialBackoffRetryDescriptor> {
        if chain_id == self.chain_id {
            Ok(self.contract_transaction_retry_descriptor)
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.contract_transaction_retry_descriptor)
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
        }
    }

    pub fn contract_view_retry_descriptor(
        &self,
        chain_id: u32,
    ) -> anyhow::Result<ExponentialBackoffRetryDescriptor> {
        if chain_id == self.chain_id {
            Ok(self.contract_view_retry_descriptor)
        } else {
            self.relayed_chains
                .iter()
                .find(|c| c.chain_id == chain_id)
                .map(|c| c.contract_view_retry_descriptor)
                .ok_or_else(|| ConfigError::InvalidChainId(chain_id).into())
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
pub fn build_wallet_from_config(account: &Account) -> anyhow::Result<Wallet<SigningKey>> {
    if account.hdwallet.is_some() {
        let mut hd = account.hdwallet.clone().unwrap();
        if hd.mnemonic.starts_with('$') {
            hd.mnemonic = env::var(hd.mnemonic.trim_start_matches('$'))?;
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
        if keystore.password.starts_with('$') {
            keystore.password = env::var(keystore.password.trim_start_matches('$'))?;
        }
        return Ok(LocalWallet::decrypt_keystore(
            &keystore.path,
            &keystore.password,
        )?);
    } else if account.private_key.is_some() {
        let mut private_key = account.private_key.clone().unwrap();
        if private_key.starts_with('$') {
            private_key = env::var(private_key.trim_start_matches('$'))?;
        }
        return Ok(private_key.parse::<Wallet<SigningKey>>()?);
    }

    Err(ConfigError::LackOfAccount.into())
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("please provide at least a hdwallet, keystore or plain private key(not recommended)")]
    LackOfAccount,
    #[error("bad format")]
    BadFormat,
    #[error(transparent)]
    EnvVarNotExisted(#[from] VarError),
    #[error(transparent)]
    BuildingAccountError(#[from] WalletError),
    #[error("the chain id: {0} is not supported")]
    InvalidChainId(u32),
}
