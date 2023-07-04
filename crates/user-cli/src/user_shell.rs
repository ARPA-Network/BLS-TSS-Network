use arpa_contract_client::contract_stub::adapter::Adapter as AdapterContract;
use arpa_contract_client::contract_stub::ierc20::IERC20 as ArpaContract;
use arpa_contract_client::contract_stub::staking::Staking as StakingContract;
use arpa_contract_client::ethers::adapter::AdapterClient;
use arpa_contract_client::{TransactionCaller, ViewCaller};
use arpa_core::u256_to_vec;
use arpa_core::RandomnessRequestType;
use arpa_core::{address_to_string, pad_to_bytes32, WalletSigner};
use arpa_user_cli::config::{build_wallet_from_config, Config};
use ethers::abi::AbiEncode;
use ethers::prelude::{NonceManagerMiddleware, SignerMiddleware};
use ethers::providers::{Http, Middleware, Provider};
use ethers::signers::Signer;
use ethers::types::{Address, BlockId, BlockNumber, Topic, H256, U256, U64};
use ethers::utils::Anvil;
use reedline_repl_rs::clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use reedline_repl_rs::Repl;
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use structopt::StructOpt;

pub const SIMPLE_ADAPTER_CODE: &str = "0x6080604052348015600f57600080fd5b506004361060325760003560e01c806376a911bc146037578063a39402d7146066575b600080fd5b60486042366004607e565b50600090565b60405167ffffffffffffffff90911681526020015b60405180910390f35b6071604236600460b9565b604051908152602001605d565b600060208284031215608f57600080fd5b813573ffffffffffffffffffffffffffffffffffffffff8116811460b257600080fd5b9392505050565b60006020828403121560ca57600080fd5b813567ffffffffffffffff81111560e057600080fd5b820160e0818503121560b257600080fdfea264697066735822122060db0656f5a3a02d609b3fb8d9ae455165807d775077e751b503136af39395c464736f6c63430008120033";
pub const RANDOMNESS_REWARD_GAS: u32 = 9000;
pub const DEFAULT_MINIMUM_THRESHOLD: u32 = 3;
pub const GAS_EXCEPT_CALLBACK: u32 = 550000 + RANDOMNESS_REWARD_GAS * DEFAULT_MINIMUM_THRESHOLD;
pub const MAX_HISTORY_CAPACITY: usize = 1000;
pub const DEFAULT_PROMPT: &str = "ARPA User CLI";

#[derive(StructOpt, Debug)]
#[structopt(name = DEFAULT_PROMPT)]
pub struct Opt {
    /// Set the config path
    #[structopt(
        short = "c",
        long,
        parse(from_os_str),
        default_value = "conf/user_config.yml"
    )]
    config_path: PathBuf,

    /// Set the history file path
    #[structopt(
        short = "H",
        long,
        parse(from_os_str),
        default_value = "user-shell.history"
    )]
    history_file_path: PathBuf,

    /// Set the block height when adapter contract deployed to accelerate the query of events
    #[structopt(short = "d", long, default_value = "0")]
    adapter_deployed_block_height: u64,
}

struct Context {
    config: Config,
    provider: Arc<Provider<Http>>,
    signer: Arc<WalletSigner>,
    adapter_deployed_block_height: u64,
    show_address: bool,
    history_file_path: PathBuf,
}

#[derive(Debug)]
pub struct Block {
    /// Hash of the block
    pub hash: Option<H256>,
    /// Hash of the parent
    pub parent_hash: H256,
    /// Hash of the uncles
    pub uncles_hash: H256,
    /// Miner/author's address. None if pending.
    pub author: Option<Address>,
    /// State root hash
    pub state_root: H256,
    /// Transactions root hash
    pub transactions_root: H256,
    /// Transactions receipts root hash
    pub receipts_root: H256,
    /// Block number. None if pending.
    pub number: Option<U64>,
    /// Gas Used
    pub gas_used: U256,
    /// Gas Limit
    pub gas_limit: U256,
    /// Timestamp
    pub timestamp: U256,
    /// Size in bytes
    pub size: Option<U256>,
}

impl<TX> From<ethers::types::Block<TX>> for Block {
    fn from(block: ethers::types::Block<TX>) -> Self {
        Self {
            hash: block.hash,
            parent_hash: block.parent_hash,
            uncles_hash: block.uncles_hash,
            author: block.author,
            state_root: block.state_root,
            transactions_root: block.transactions_root,
            receipts_root: block.receipts_root,
            number: block.number,
            gas_used: block.gas_used,
            gas_limit: block.gas_limit,
            timestamp: block.timestamp,
            size: block.size,
        }
    }
}

#[derive(Debug)]
pub struct RandomnessRequest {
    pub request_id: String,
    pub sub_id: u64,
    pub group_index: u32,
    pub request_type: RandomnessRequestType,
    pub params: ethers::core::types::Bytes,
    pub sender: ethers::core::types::Address,
    pub seed: ethers::core::types::U256,
    pub request_confirmations: u16,
    pub callback_gas_limit: u32,
    pub callback_max_gas_price: ethers::core::types::U256,
    pub estimated_payment: ethers::core::types::U256,
    pub fulfillment_result: Option<RandomnessRequestResult>,
}

#[derive(Debug)]
pub struct RandomnessRequestResult {
    pub request_id: String,
    pub group_index: u32,
    pub committer: ethers::core::types::Address,
    pub participant_members: Vec<ethers::core::types::Address>,
    pub randommness: ethers::core::types::U256,
    pub payment: ethers::core::types::U256,
    pub flat_fee: ethers::core::types::U256,
    pub success: bool,
}

#[derive(Debug)]
pub struct Consumer {
    pub address: Address,
    pub added_block: u64,
    pub nonce: u64,
}

pub struct StakingClient;

impl ViewCaller for StakingClient {}
impl TransactionCaller for StakingClient {}

pub struct ArpaClient;

impl ViewCaller for ArpaClient {}
impl TransactionCaller for ArpaClient {}

async fn send(args: ArgMatches, context: &mut Context) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("approve-arpa-to-staking", sub_matches)) => {
            let amount = sub_matches.get_one::<String>("amount").unwrap();
            let amount = U256::from_dec_str(amount).unwrap();

            let arpa_contract =
                ArpaContract::new(context.config.arpa_address(), context.signer.clone());

            let trx_hash = ArpaClient::call_contract_transaction(
                "approve-arpa-to-staking",
                arpa_contract.approve(context.config.staking_address(), amount),
                context.config.contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Approve arpa for staking successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("stake", sub_matches)) => {
            let amount = sub_matches.get_one::<String>("amount").unwrap();
            let amount = U256::from_dec_str(amount).unwrap();

            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let arpa_contract =
                ArpaContract::new(context.config.arpa_address(), context.signer.clone());

            let balance = ArpaClient::call_contract_view(
                "balance_of",
                arpa_contract.balance_of(context.signer.address()),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            if balance < amount {
                return Ok(Some(format!(
                    "Insufficient balance, balance: {}, amount: {}",
                    balance, amount
                )));
            }

            let allowance = ArpaClient::call_contract_view(
                "allowance",
                arpa_contract.allowance(context.signer.address(), context.config.staking_address()),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            if allowance < amount {
                return Ok(Some(format!(
                    "Insufficient allowance, allowance: {}, amount: {}",
                    allowance, amount
                )));
            }

            let trx_hash = StakingClient::call_contract_transaction(
                "stake",
                staking_contract.stake(amount),
                context.config.contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Stake arpa successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("unstake", sub_matches)) => {
            let amount = sub_matches.get_one::<String>("amount").unwrap();
            let amount = U256::from_dec_str(amount).unwrap();

            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let staked_amount = StakingClient::call_contract_view(
                "staked_amount",
                staking_contract.get_stake(context.signer.address()),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            if staked_amount < amount {
                return Ok(Some(format!(
                    "Insufficient staked amount, staked amount: {}, amount: {}",
                    staked_amount, amount
                )));
            }

            let trx_hash = StakingClient::call_contract_transaction(
                "unstake",
                staking_contract.unstake(amount),
                context.config.contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Unstake arpa successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("claim", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let trx_hash = StakingClient::call_contract_transaction(
                "claim",
                staking_contract.claim(),
                context.config.contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Claim successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("claim-reward", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let trx_hash = StakingClient::call_contract_transaction(
                "claim_reward",
                staking_contract.claim_reward(),
                context.config.contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Claim reward successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("claim-frozen-principal", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let trx_hash = StakingClient::call_contract_transaction(
                "claim_frozen_principal",
                staking_contract.claim_frozen_principal(),
                context.config.contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Claim frozen principal successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("create-subscription", _sub_matches)) => {
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let trx_hash = AdapterClient::call_contract_transaction(
                "create_subscription",
                adapter_contract.create_subscription(),
                context.config.contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Create subscription successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("add-consumer", sub_matches)) => {
            let sub_id = sub_matches.get_one::<u64>("sub-id").unwrap();
            let consumer = sub_matches.get_one::<String>("consumer").unwrap();

            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let trx_hash = AdapterClient::call_contract_transaction(
                "add_consumer",
                adapter_contract.add_consumer(*sub_id, consumer.parse().unwrap()),
                context.config.contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Add consumer successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("fund-subscription", sub_matches)) => {
            let sub_id = sub_matches.get_one::<u64>("sub-id").unwrap();
            let amount = sub_matches.get_one::<String>("amount").unwrap();
            let amount = U256::from_dec_str(amount).unwrap();

            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let trx_hash = AdapterClient::call_contract_transaction(
                "fund_subscription",
                adapter_contract.fund_subscription(*sub_id).value(amount),
                context.config.contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Fund subscription successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("set-referral", sub_matches)) => {
            let sub_id = sub_matches.get_one::<u64>("sub-id").unwrap();
            let referral_sub_id = sub_matches.get_one::<u64>("referral-sub-id").unwrap();

            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let trx_hash = AdapterClient::call_contract_transaction(
                "set_referral",
                adapter_contract.set_referral(*sub_id, *referral_sub_id),
                context.config.contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Set referral successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("cancel-subscription", sub_matches)) => {
            let sub_id = sub_matches.get_one::<u64>("sub-id").unwrap();
            let recipient = sub_matches.get_one::<String>("recipient").unwrap();

            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let trx_hash = AdapterClient::call_contract_transaction(
                "cancel_subscription",
                adapter_contract.cancel_subscription(*sub_id, recipient.parse().unwrap()),
                context.config.contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Cancel subscription successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("remove-consumer", sub_matches)) => {
            let sub_id = sub_matches.get_one::<u64>("sub-id").unwrap();
            let consumer = sub_matches.get_one::<String>("consumer").unwrap();

            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let trx_hash = AdapterClient::call_contract_transaction(
                "remove_consumer",
                adapter_contract.remove_consumer(*sub_id, consumer.parse().unwrap()),
                context.config.contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Remove consumer successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("set-callback-gas-config", sub_matches)) => {
            let consumer = sub_matches.get_one::<String>("consumer").unwrap();
            let consumer_owner_private_key = sub_matches
                .get_one::<String>("consumer-owner-private-key")
                .unwrap();
            let callback_gas_limit = sub_matches.get_one::<String>("callback-gas-limit").unwrap();
            let callback_max_gas_fee = sub_matches
                .get_one::<String>("callback-max-gas-fee")
                .unwrap();

            let consumer_owner_private_key = if consumer_owner_private_key.starts_with('$') {
                env::var(consumer_owner_private_key.trim_start_matches('$'))?
            } else {
                consumer_owner_private_key.to_owned()
            };

            let set_callback_gas_config_args = vec![
                "send",
                consumer,
                "setCallbackGasConfig(uint32,uint256)",
                callback_gas_limit,
                callback_max_gas_fee,
                "--private-key",
                &consumer_owner_private_key,
            ];
            let cast_res = call_cast(
                &context.config.provider_endpoint,
                &set_callback_gas_config_args,
            );
            let trx_hash = &cast_res[cast_res.find("transactionHash").unwrap()
                + "transactionHash         ".len()
                ..cast_res.find("\ntransactionIndex").unwrap()];
            Ok(Some(format!(
                "Set callback gas config successfully, transaction hash: {}",
                trx_hash
            )))
        }

        _ => panic!("Unknown subcommand {:?}", args.subcommand_name()),
    }
}

async fn call(args: ArgMatches, context: &mut Context) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("current-gas-price", _sub_matches)) => {
            let gas_price = context.provider.get_gas_price().await?;

            Ok(Some(format!("current gas price: {:#?}", gas_price)))
        }
        Some(("block", sub_matches)) => {
            let block_number = sub_matches.get_one::<String>("block-number").unwrap();
            match block_number.as_str() {
                "latest" => {
                    let block: Option<Block> = context
                        .provider
                        .get_block(BlockNumber::Latest)
                        .await?
                        .map(|block| block.into());
                    return Ok(Some(format!("block: {:#?}", block)));
                }
                "earliest" => {
                    let block: Option<Block> = context
                        .provider
                        .get_block(BlockNumber::Earliest)
                        .await?
                        .map(|block| block.into());
                    return Ok(Some(format!("block: {:#?}", block)));
                }
                "pending" => {
                    let block: Option<Block> = context
                        .provider
                        .get_block(BlockNumber::Pending)
                        .await?
                        .map(|block| block.into());
                    return Ok(Some(format!("block: {:#?}", block)));
                }
                _ => {
                    if let Ok(block_number) = block_number.parse::<u64>() {
                        let block: Option<Block> = context
                            .provider
                            .get_block(BlockNumber::Number(block_number.into()))
                            .await?
                            .map(|block| block.into());
                        return Ok(Some(format!("block: {:#?}", block)));
                    }
                }
            }
            panic!("Unknown block number {:?}", block_number);
        }
        Some(("trx-receipt", sub_matches)) => {
            let trx_hash = sub_matches.get_one::<String>("trx-hash").unwrap();

            let receipt = context
                .provider
                .get_transaction_receipt(
                    pad_to_bytes32(&hex::decode(
                        if let Some(trx_hash_without_prefix) = trx_hash.strip_prefix("0x") {
                            trx_hash_without_prefix
                        } else {
                            trx_hash
                        },
                    )?)
                    .unwrap(),
                )
                .await?;

            Ok(Some(format!("trx receipt: {:#?}", receipt)))
        }
        Some(("balance-of-eth", _sub_matches)) => {
            let balance = context
                .provider
                .get_balance(
                    context.signer.address(),
                    Some(BlockId::Number(BlockNumber::Latest)),
                )
                .await?;

            Ok(Some(format!("balance: {:#?}", balance)))
        }
        Some(("balance-of-arpa", _sub_matches)) => {
            let arpa_contract =
                ArpaContract::new(context.config.arpa_address(), context.signer.clone());

            let balance = ArpaClient::call_contract_view(
                "balance_of",
                arpa_contract.balance_of(context.signer.address()),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("balance: {:#?}", balance)))
        }
        _ => panic!("Unknown subcommand {:?}", args.subcommand_name()),
    }
}

fn call_cast(rpc_url: &str, args: &[&str]) -> String {
    let mut cmd = std::process::Command::new("cast");
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit());

    cmd.args(args);
    cmd.arg("--rpc-url");
    cmd.arg(rpc_url);
    let mut child = cmd.spawn().expect("couldnt start cast");

    let stdout = child
        .stdout
        .take()
        .expect("Unable to get stdout for cast child process");

    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_to_string(&mut line)
        .expect("Failed to read line from cast process");
    line.trim_end_matches('\n').to_string()
}

async fn randcast(args: ArgMatches, context: &mut Context) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("nonce", sub_matches)) => {
            let consumer = sub_matches.get_one::<String>("consumer").unwrap();

            let nonce_args = vec!["call", consumer, "nonce()(uint256)"];
            let nonce_res = call_cast(&context.config.provider_endpoint, &nonce_args);

            Ok(Some(format!("consumer_nonce: {}", nonce_res)))
        }
        Some(("callback-gas-limit", sub_matches)) => {
            let consumer = sub_matches.get_one::<String>("consumer").unwrap();

            let callback_gas_limit_args = vec!["call", consumer, "callbackGasLimit()(uint32)"];
            let callback_gas_limit_res =
                call_cast(&context.config.provider_endpoint, &callback_gas_limit_args);

            Ok(Some(format!(
                "callback_gas_limit: {}",
                callback_gas_limit_res
            )))
        }
        Some(("callback-max-gas-fee", sub_matches)) => {
            let consumer = sub_matches.get_one::<String>("consumer").unwrap();

            let callback_max_gas_fee_args = vec!["call", consumer, "callbackMaxGasFee()(uint256)"];
            let callback_max_gas_fee_res = call_cast(
                &context.config.provider_endpoint,
                &callback_max_gas_fee_args,
            );

            Ok(Some(format!(
                "callback_max_gas_fee: {}",
                callback_max_gas_fee_res
            )))
        }
        Some(("estimate-callback-gas", sub_matches)) => {
            let consumer = sub_matches.get_one::<String>("consumer").unwrap();
            let request_sender = sub_matches.get_one::<String>("request-sender").unwrap();
            let request_signature = sub_matches.get_one::<String>("request-signature").unwrap();
            let request_params = sub_matches.get_one::<String>("request-params").unwrap();

            let existed_callback_gas_limit_args =
                vec!["call", consumer, "callbackGasLimit()(uint32)"];
            let existed_callback_gas_limit = call_cast(
                &context.config.provider_endpoint,
                &existed_callback_gas_limit_args,
            );

            let anvil = Anvil::new()
                .chain_id(context.config.chain_id as u64)
                .fork(&context.config.provider_endpoint)
                .port(8544u16)
                .spawn();

            if existed_callback_gas_limit != "0" {
                println!(
                    "callbackGasLimit is already set: {}",
                    existed_callback_gas_limit
                );

                let get_owner_args = vec!["call", consumer, "owner()(address)"];
                let owner = call_cast(&anvil.endpoint(), &get_owner_args);

                let impersonate_account_args = vec!["rpc", "anvil_impersonateAccount", &owner];
                call_cast(&anvil.endpoint(), &impersonate_account_args);

                let reset_callback_gas_args = vec![
                    "send",
                    consumer,
                    "setCallbackGasConfig(uint32,uint256)",
                    "0",
                    "0",
                    "--from",
                    &owner,
                    "--unlocked",
                ];
                call_cast(&anvil.endpoint(), &reset_callback_gas_args);
            }

            let impersonate_account_args = vec!["rpc", "anvil_impersonateAccount", request_sender];
            call_cast(&anvil.endpoint(), &impersonate_account_args);

            // replace adapter code to make sure the request randomness success
            let adapter_address = address_to_string(context.config.adapter_address());
            let set_code_args = vec![
                "rpc",
                "anvil_setCode",
                &adapter_address,
                SIMPLE_ADAPTER_CODE,
            ];
            call_cast(&anvil.endpoint(), &set_code_args);

            let request_randomness_args = vec![
                "send",
                consumer,
                request_signature,
                request_params,
                "--from",
                request_sender,
                "--unlocked",
            ];
            call_cast(&anvil.endpoint(), &request_randomness_args);

            let callback_gas_limit_args = vec!["call", consumer, "callbackGasLimit()(uint32)"];
            let callback_gas_limit_res = call_cast(&anvil.endpoint(), &callback_gas_limit_args);

            Ok(Some(format!(
                "estimate-callback-gas_limit_res: {}",
                callback_gas_limit_res
            )))
        }
        Some(("estimate-payment-amount", sub_matches)) => {
            let callback_gas_limit = sub_matches.get_one::<u32>("callback-gas-limit").unwrap();

            let gas_price = context.provider.get_gas_price().await?;

            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let payment_amount_in_eth = AdapterClient::call_contract_view(
                "estimate_payment_amount",
                adapter_contract.estimate_payment_amount_in_eth(
                    *callback_gas_limit,
                    GAS_EXCEPT_CALLBACK,
                    0,
                    gas_price * 3,
                ),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "payment_amount_in_eth_wei: {:#?} in 3 times of current gas price: 3 * {:#?}",
                payment_amount_in_eth, gas_price
            )))
        }
        Some(("adapter-config", _sub_matches)) => {
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let (
                minimum_request_confirmations,
                max_gas_limit,
                gas_after_payment_calculation,
                gas_except_callback,
                signature_task_exclusive_window,
                reward_per_signature,
                committer_reward_per_signature,
            ) = AdapterClient::call_contract_view(
                "adapter_config",
                adapter_contract.get_adapter_config(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "minimum_request_confirmations: {:#?}, max_gas_limit: {:#?}, gas_after_payment_calculation: {:#?}, gas_except_callback: {:#?}, \
                signature_task_exclusive_window: {:#?}, reward_per_signature: {:#?}, committer_reward_per_signature: {:#?}",
                minimum_request_confirmations,
                max_gas_limit,
                gas_after_payment_calculation,
                gas_except_callback,
                signature_task_exclusive_window,
                reward_per_signature,
                committer_reward_per_signature,
            )))
        }
        Some(("flat-fee-config", _sub_matches)) => {
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let (
                fulfillment_flat_fee_link_ppm_tier1,
                fulfillment_flat_fee_link_ppm_tier2,
                fulfillment_flat_fee_link_ppm_tier3,
                fulfillment_flat_fee_link_ppm_tier4,
                fulfillment_flat_fee_link_ppm_tier5,
                reqs_for_tier2,
                reqs_for_tier3,
                reqs_for_tier4,
                reqs_for_tier5,
                flat_fee_promotion_global_percentage,
                is_flat_fee_promotion_enabled_permanently,
                flat_fee_promotion_start_timestamp,
                flat_fee_promotion_end_timestamp,
            ) = AdapterClient::call_contract_view_without_log(
                adapter_contract.get_flat_fee_config(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "fulfillment_flat_fee_link_ppm_tier1: {:#?}, fulfillment_flat_fee_link_ppm_tier2: {:#?}, fulfillment_flat_fee_link_ppm_tier3: {:#?}, fulfillment_flat_fee_link_ppm_tier4: {:#?}, fulfillment_flat_fee_link_ppm_tier5: {:#?}, \
                reqs_for_tier2: {:#?}, reqs_for_tier3: {:#?}, reqs_for_tier4: {:#?}, reqs_for_tier5: {:#?}, flat_fee_promotion_global_percentage: {:#?}, is_flat_fee_promotion_enabled_permanently: {:#?}, flat_fee_promotion_start_timestamp: {:#?}, flat_fee_promotion_end_timestamp: {:#?}",
                fulfillment_flat_fee_link_ppm_tier1,
                fulfillment_flat_fee_link_ppm_tier2,
                fulfillment_flat_fee_link_ppm_tier3,
                fulfillment_flat_fee_link_ppm_tier4,
                fulfillment_flat_fee_link_ppm_tier5,
                reqs_for_tier2,
                reqs_for_tier3,
                reqs_for_tier4,
                reqs_for_tier5,
                flat_fee_promotion_global_percentage,
                is_flat_fee_promotion_enabled_permanently,
                flat_fee_promotion_start_timestamp,
                flat_fee_promotion_end_timestamp,
            )))
        }
        Some(("referral-config", _sub_matches)) => {
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let (
                is_referral_enabled,
                free_request_count_for_referrer,
                free_request_count_for_referee,
            ) = AdapterClient::call_contract_view(
                "referral_config",
                adapter_contract.get_referral_config(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "is_referral_enabled: {:#?}, free_request_count_for_referrer: {:#?}, free_request_count_for_referee: {:#?}",
                is_referral_enabled,
                free_request_count_for_referrer,
                free_request_count_for_referee,
            )))
        }
        Some(("fee-tier", sub_matches)) => {
            let req_count = sub_matches.get_one::<u64>("req-count").unwrap();
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let fee_ppm = AdapterClient::call_contract_view(
                "fee_tier",
                adapter_contract.get_fee_tier(*req_count),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("fee_ppm: {:#?}", fee_ppm)))
        }
        Some(("subscription", sub_matches)) => {
            let sub_id = sub_matches.get_one::<u64>("sub-id").unwrap();
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let (
                owner,
                consumers,
                balance,
                inflight_cost,
                req_count,
                free_request_count,
                referral_sub_id,
                req_count_in_current_period,
                last_request_timestamp,
            ) = AdapterClient::call_contract_view(
                "get_subscription",
                adapter_contract.get_subscription(*sub_id),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "owner: {:#?}, consumers: {:#?}, balance: {:#?}, inflight_cost: {:#?}, req_count: {:#?}, free_request_count: {:#?}, referral_sub_id: {:#?}, req_count_in_current_period: {:#?}, last_request_timestamp: {:#?}",
                owner,
                consumers,
                balance,
                inflight_cost,
                req_count,
                free_request_count,
                referral_sub_id,
                req_count_in_current_period,
                last_request_timestamp,
            )))
        }
        Some(("my-subscriptions", _sub_matches)) => {
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let created_filter = adapter_contract
                .subscription_created_filter()
                .topic2(context.signer.address())
                .from_block(context.adapter_deployed_block_height)
                .to_block(BlockNumber::Latest);

            let created_logs = created_filter.query().await?;

            let created_subids = created_logs
                .iter()
                .map(|created_log| {
                    H256::from_str(&U256::from(created_log.sub_id).encode_hex()).map(Some)
                })
                .collect::<Result<Vec<_>, _>>()?;

            let canceled_filter = adapter_contract
                .subscription_canceled_filter()
                .topic1(Topic::Array(created_subids))
                .from_block(context.adapter_deployed_block_height)
                .to_block(BlockNumber::Latest);

            let canceled_logs = canceled_filter.query().await?;

            // get existed subscriptions by filtering out canceled subscriptions from created subscriptions
            let existed_subscriptions: Vec<u64> = created_logs
                .into_iter()
                .filter(|created_log| {
                    !canceled_logs
                        .iter()
                        .any(|canceled_log| canceled_log.sub_id == created_log.sub_id)
                })
                .map(|created_log| created_log.sub_id)
                .collect();

            Ok(Some(format!(
                "my subscriptions: {:#?}",
                existed_subscriptions
            )))
        }
        Some(("consumers", sub_matches)) => {
            let sub_id = sub_matches.get_one::<u64>("sub-id").unwrap();
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let (
                _owner,
                consumer_addresses,
                _balance,
                _inflight_cost,
                _req_count,
                _free_request_count,
                _referral_sub_id,
                _req_count_in_current_period,
                _last_request_timestamp,
            ) = AdapterClient::call_contract_view(
                "get_subscription",
                adapter_contract.get_subscription(*sub_id),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            let mut consumers: BTreeMap<Address, Consumer> = consumer_addresses
                .into_iter()
                .map(|consumer_address: Address| {
                    (
                        consumer_address,
                        Consumer {
                            address: consumer_address,
                            added_block: 0,
                            nonce: 1,
                        },
                    )
                })
                .collect();

            let consumer_added_filter = adapter_contract
                .subscription_consumer_added_filter()
                .topic1(H256::from(
                    pad_to_bytes32(&u256_to_vec(&U256::from(*sub_id))).unwrap(),
                ))
                .from_block(context.adapter_deployed_block_height)
                .to_block(BlockNumber::Latest);

            for (log, meta) in consumer_added_filter.query_with_meta().await? {
                let consumer = consumers.get_mut(&log.consumer).unwrap();
                consumer.added_block = meta.block_number.as_u64();
            }

            let filter = adapter_contract
                .randomness_request_filter()
                .topic2(H256::from(
                    pad_to_bytes32(&u256_to_vec(&U256::from(*sub_id))).unwrap(),
                ))
                .from_block(context.adapter_deployed_block_height)
                .to_block(BlockNumber::Latest);

            let logs = filter.query().await?;

            for log in logs {
                let consumer = consumers.get_mut(&log.sender).unwrap();

                consumer.nonce += 1;
            }

            Ok(Some(format!("consumers: {:#?}", consumers)))
        }
        Some(("requests", sub_matches)) => {
            let sub_id = sub_matches.get_one::<u64>("sub-id").unwrap();
            let consumer = sub_matches.get_one::<String>("consumer");
            let is_pending = sub_matches.get_flag("pending");
            let is_success = sub_matches.get_flag("success");
            let is_failed = sub_matches.get_flag("failed");

            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let filter = adapter_contract
                .randomness_request_filter()
                .topic2(H256::from(
                    pad_to_bytes32(&u256_to_vec(&U256::from(*sub_id))).unwrap(),
                ))
                .from_block(context.adapter_deployed_block_height)
                .to_block(BlockNumber::Latest);

            let logs = filter.query().await?;

            let mut results = logs
                .iter()
                .map(|log| RandomnessRequest {
                    request_id: hex::encode(log.request_id),
                    sub_id: log.sub_id,
                    group_index: log.group_index,
                    seed: log.seed,
                    sender: log.sender,
                    request_type: log.request_type.into(),
                    params: log.params.clone(),
                    request_confirmations: log.request_confirmations,
                    callback_gas_limit: log.callback_gas_limit,
                    callback_max_gas_price: log.callback_max_gas_price,
                    estimated_payment: log.estimated_payment,
                    fulfillment_result: None,
                })
                .collect::<Vec<_>>();

            if let Some(consumer) = consumer {
                results = results
                    .into_iter()
                    .filter(|r| r.sender == consumer.parse().unwrap())
                    .collect::<Vec<_>>();
            }

            for result in results.iter_mut() {
                let fulfillment_filter = adapter_contract
                    .randomness_request_result_filter()
                    .topic1(H256::from(
                        pad_to_bytes32(&hex::decode(&result.request_id)?).unwrap(),
                    ))
                    .from_block(context.adapter_deployed_block_height)
                    .to_block(BlockNumber::Latest);

                let fulfillments = fulfillment_filter.query().await?;
                fulfillments.iter().for_each(|fulfillment| {
                    result.fulfillment_result = Some(RandomnessRequestResult {
                        request_id: hex::encode(fulfillment.request_id),
                        group_index: fulfillment.group_index,
                        committer: fulfillment.committer,
                        participant_members: fulfillment.participant_members.clone(),
                        randommness: fulfillment.randommness,
                        payment: fulfillment.payment,
                        flat_fee: fulfillment.flat_fee,
                        success: fulfillment.success,
                    });
                });
            }

            // filter results if fulfillment_result is_pending, is_success, or is_failed
            if is_pending {
                results = results
                    .into_iter()
                    .filter(|r| r.fulfillment_result.is_none())
                    .collect::<Vec<_>>();
            } else if is_success {
                results = results
                    .into_iter()
                    .filter(|r| {
                        r.fulfillment_result
                            .as_ref()
                            .map(|fr| fr.success)
                            .unwrap_or(false)
                    })
                    .collect::<Vec<_>>();
            } else if is_failed {
                results = results
                    .into_iter()
                    .filter(|r| {
                        r.fulfillment_result
                            .as_ref()
                            .map(|fr| !fr.success)
                            .unwrap_or(false)
                    })
                    .collect::<Vec<_>>();
            }

            println!("{} request(s) found!", results.iter().len());

            Ok(Some(format!("requests: {:#?}", results)))
        }
        Some(("last-assigned-group-index", _sub_matches)) => {
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let last_assigned_group_index = AdapterClient::call_contract_view(
                "get_last_assigned_group_index",
                adapter_contract.get_last_assigned_group_index(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "last_assigned_group_index: {:#?}",
                last_assigned_group_index
            )))
        }
        Some(("randomness-count", _sub_matches)) => {
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let randomness_count = AdapterClient::call_contract_view(
                "get_randomness_count",
                adapter_contract.get_randomness_count(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("randomness_count: {:#?}", randomness_count)))
        }
        Some(("cumulative-data", _sub_matches)) => {
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let (
                cumulative_flat_fee,
                cumulative_committer_reward,
                cumulative_partial_signature_reward,
            ) = AdapterClient::call_contract_view(
                "cumulative_data",
                adapter_contract.get_cumulative_data(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("cumulativeFlatFee: {:#?}, cumulativeCommitterReward: {:#?}, cumulativePartialSignatureReward: {:#?}", 
            cumulative_flat_fee, cumulative_committer_reward, cumulative_partial_signature_reward)))
        }
        Some(("last-randomness", _sub_matches)) => {
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let last_randomness = AdapterClient::call_contract_view(
                "get_last_randomness",
                adapter_contract.get_last_randomness(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(last_randomness.to_string()))
        }
        Some(("pending-request-commitment", sub_matches)) => {
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let r_id = sub_matches.get_one::<String>("request-id").unwrap();

            let pending_request_commitment = adapter_contract
                .get_pending_request_commitment(pad_to_bytes32(&hex::decode(r_id)?).unwrap())
                .await?;

            Ok(Some(hex::encode(pending_request_commitment)))
        }

        _ => panic!("Unknown subcommand {:?}", args.subcommand_name()),
    }
}

async fn stake(args: ArgMatches, context: &mut Context) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("stake", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let amount = StakingClient::call_contract_view(
                "get_stake",
                staking_contract.get_stake(context.signer.address()),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("staked amount: {:#?}", amount)))
        }
        Some(("base-reward", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let base_reward = StakingClient::call_contract_view(
                "get_base_reward",
                staking_contract.get_base_reward(context.signer.address()),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("base reward: {:#?}", base_reward)))
        }
        Some(("delegation-reward", sub_matches)) => {
            let delegator_address = sub_matches
                .get_one::<String>("delegator-address")
                .unwrap()
                .parse::<Address>()?;

            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let delegation_reward = StakingClient::call_contract_view(
                "get_delegation_reward",
                staking_contract.get_delegation_reward(delegator_address),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("delegation reward: {:#?}", delegation_reward)))
        }
        Some(("total-delegated-amount", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let total_delegated_amount = StakingClient::call_contract_view(
                "get_total_delegated_amount",
                staking_contract.get_total_delegated_amount(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "total delegated amount: {:#?}",
                total_delegated_amount
            )))
        }
        Some(("delegates-count", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let delegates_count = StakingClient::call_contract_view(
                "get_delegates_count",
                staking_contract.get_delegates_count(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("delegates count: {:#?}", delegates_count)))
        }
        Some(("community-stakers-count", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let community_stakers_count = StakingClient::call_contract_view(
                "get_community_stakers_count",
                staking_contract.get_community_stakers_count(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "community stakers count: {:#?}",
                community_stakers_count
            )))
        }
        Some(("total-staked-amount", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let total_staked_amount = StakingClient::call_contract_view(
                "get_total_staked_amount",
                staking_contract.get_total_staked_amount(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "total staked amount: {:#?}",
                total_staked_amount
            )))
        }
        Some(("total-community-staked-amount", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let total_community_staked_amount = StakingClient::call_contract_view(
                "get_total_community_staked_amount",
                staking_contract.get_total_community_staked_amount(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "total community staked amount: {:#?}",
                total_community_staked_amount
            )))
        }
        Some(("total-frozen-amount", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let total_frozen_amount = StakingClient::call_contract_view(
                "get_total_frozen_amount",
                staking_contract.get_total_frozen_amount(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "total frozen amount: {:#?}",
                total_frozen_amount
            )))
        }
        Some(("max-pool-size", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let max_pool_size = StakingClient::call_contract_view(
                "get_max_pool_size",
                staking_contract.get_max_pool_size(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("max pool size: {:#?}", max_pool_size)))
        }
        Some(("community-staker-limits", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let (min, max) = StakingClient::call_contract_view(
                "get_community_staker_limits",
                staking_contract.get_community_staker_limits(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("min: {:#?}, max: {:#?}", min, max)))
        }
        Some(("operator-limit", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let limit = StakingClient::call_contract_view(
                "get_operator_limit",
                staking_contract.get_operator_limit(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("operator limit: {:#?}", limit)))
        }
        Some(("reward-timestamps", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let (init, expiry) = StakingClient::call_contract_view(
                "get_reward_timestamps",
                staking_contract.get_reward_timestamps(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("init: {:#?}, expiry: {:#?}", init, expiry)))
        }
        Some(("reward-rate", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let rate = StakingClient::call_contract_view(
                "get_reward_rate",
                staking_contract.get_reward_rate(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("reward rate: {:#?}", rate)))
        }
        Some(("reward-apy", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let rate = StakingClient::call_contract_view(
                "get_reward_rate",
                staking_contract.get_reward_rate(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            let total_community_staked_amount = StakingClient::call_contract_view(
                "get_total_community_staked_amount",
                staking_contract.get_total_community_staked_amount(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            let apy_with_precision: U256 =
                rate * 3600 * 24 * 365 * 95 * 10_000 / total_community_staked_amount;

            Ok(Some(format!(
                "reward APY: {}%",
                apy_with_precision.as_u64() as f64 / 10_000.0
            )))
        }
        Some(("delegation-rate-denominator", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let rate = StakingClient::call_contract_view(
                "get_delegation_rate_denominator",
                staking_contract.get_delegation_rate_denominator(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("delegation rate denominator: {:#?}", rate)))
        }
        Some(("frozen-principal", _sub_matches)) => {
            let staking_contract =
                StakingContract::new(context.config.staking_address(), context.signer.clone());

            let (amounts, timestamps) = StakingClient::call_contract_view(
                "frozen_principal",
                staking_contract.get_frozen_principal(context.signer.address()),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "amounts: {:#?}, unfreeze timestamps: {:#?}",
                amounts, timestamps
            )))
        }

        _ => panic!("Unknown subcommand {:?}", args.subcommand_name()),
    }
}

async fn show(args: ArgMatches, context: &mut Context) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("address", _sub_matches)) => {
            context.show_address = true;
            Ok(Some(address_to_string(context.signer.address())))
        }
        Some(("config", _sub_matches)) => Ok(Some(format!("{:#?}", context.config))),

        _ => panic!("Unknown subcommand {:?}", args.subcommand_name()),
    }
}

fn history(_args: ArgMatches, context: &mut Context) -> anyhow::Result<Option<String>> {
    Ok(Some(read_file_line_by_line(
        context.history_file_path.clone(),
    )?))
}

// Called after successful command execution, updates prompt with returned Option
async fn update_prompt(context: &mut Context) -> anyhow::Result<Option<String>> {
    Ok(Some(if context.show_address {
        address_to_string(context.signer.address())
    } else {
        DEFAULT_PROMPT.to_owned()
    }))
}

fn read_file_line_by_line(filepath: PathBuf) -> anyhow::Result<String> {
    let mut res: String = Default::default();
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);

    for (number, line) in reader.lines().enumerate() {
        res += &format!("{}    {}\n", number + 1, line?);
    }

    Ok(res)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    let config = Config::load(opt.config_path);

    let wallet = build_wallet_from_config(&config.account)?.with_chain_id(config.chain_id as u32);

    let provider = Arc::new(Provider::<Http>::try_from(config.provider_endpoint.clone()).unwrap());

    let nonce_manager = NonceManagerMiddleware::new(provider.clone(), wallet.address());

    let signer = Arc::new(SignerMiddleware::new(nonce_manager, wallet));

    let context = Context {
        config,
        provider,
        signer,
        adapter_deployed_block_height: opt.adapter_deployed_block_height,
        show_address: false,
        history_file_path: opt.history_file_path.clone(),
    };

    let mut repl = Repl::new(context)
        .with_name("ARPA User CLI")
        .with_history(opt.history_file_path, MAX_HISTORY_CAPACITY)
        .with_version("v0.0.1")
        .with_description("ARPA User CLI is a utilitarian REPL for users of ARPA Network.")
        .with_banner("Welcome, Tip: Search history with CTRL+R, clear input with CTRL+C, exit repl with CTRL+D")
        .with_prompt(DEFAULT_PROMPT)
        .with_command(Command::new("history").about("Show command history"), history)
        .with_command_async(
            Command::new("call")
                .subcommand(
                    Command::new("block").visible_alias("b").about("Get block information")
                        .arg(Arg::new("block-number").required(true).help("block number in latest/ earliest/ pending/ decimal number"))
                ).subcommand(
                    Command::new("current-gas-price").visible_alias("cgp").about("Get current gas price")
                ).subcommand(
                    Command::new("trx-receipt").visible_alias("tr").about("Get transaction receipt")
                        .arg(Arg::new("trx-hash").required(true).help("transaction hash in hex format"))
                ).subcommand(
                    Command::new("balance-of-eth").visible_alias("boe").about("Get balance of eth")
                ).subcommand(
                    Command::new("balance-of-arpa").visible_alias("boa")
                        .about("Get balance of arpa")
                )
                .about("Get information from blockchain"),
                |args, context| Box::pin(call(args, context)),
        ).with_command_async(
            Command::new("randcast")
                .subcommand(
                    Command::new("subscription").visible_alias("s").about("Get subscription by subscription id")
                        .arg(Arg::new("sub-id").required(true).value_parser(value_parser!(u64)).help("subscription id in decimal"))
                ).subcommand(
                    Command::new("my-subscriptions").visible_alias("mss").about("Get my subscriptions")
                ).subcommand(
                    Command::new("consumers").visible_alias("cs").about("Get consumer contracts by subscription id")
                        .arg(Arg::new("sub-id").required(true).value_parser(value_parser!(u64)).help("subscription id in decimal"))
                ).subcommand(
                    Command::new("requests").visible_alias("rs").about("Get requests by subscription id, filter by consumer address, pending/ success/ failed")
                        .arg(Arg::new("sub-id").required(true).value_parser(value_parser!(u64)).help("subscription id in decimal"))
                        .arg(Arg::new("consumer").required(false).help("sent by consumer address in hex format"))
                        .arg(Arg::new("pending").long("pending").required(false).action(ArgAction::SetTrue).help("only pending requests"))
                        .arg(Arg::new("success").long("success").required(false).action(ArgAction::SetTrue).help("only success requests"))
                        .arg(Arg::new("failed").long("failed").required(false).action(ArgAction::SetTrue).help("only failed requests, which means the callback function in consumer contract reverts due to business logic or gas limit"))
                ).subcommand(
                    Command::new("estimate-callback-gas").visible_alias("ecg")
                        .about("Estimate callback gas for any consumer contract extends GeneralRandcastConsumerBase under the current circumstances. \
                                This can be used before the first request to estimate how much eth is needed for subscription funding, \
                                or at any time to compare gas cost with the estimated one to adjust the callback gas config in the consumer contract. \
                                This also can be used as a dry run to see if the callback function in consumer contract reverts due to business logic or gas limit. \
                                An error will be returned if callback in the consumer contract reverts.")
                        .arg(Arg::new("consumer").required(true).help("address of your customized consumer contract in hex format"))
                        .arg(Arg::new("request-sender").required(true).help("sender address(depending on your business logic, don't have to be the owner of the consumer contract) to request randomness(don't have to be the function in consumer contract) in hex format"))
                        .arg(Arg::new("request-signature").required(true).help("function signature of request randomness with a pair of quotation marks"))
                        .arg(Arg::new("request-params").required(true).help("request params split by space"))
                ).subcommand(
                    Command::new("estimate-payment-amount").visible_alias("epa").about("Estimate the amount of gas used for a fulfillment of randomness in 3 times of current gas price, for calculating how much eth is needed for subscription funding")
                        .arg(Arg::new("callback-gas-limit").required(true).value_parser(value_parser!(u32)).help("callback gas limit by calling estimate-callback-gas"))
                ).subcommand(
                    Command::new("callback-gas-limit").visible_alias("cgl").about("Get callback gas limit of consumer contract")
                        .arg(Arg::new("consumer").required(true).help("consumer address in hex format"))
                ).subcommand(
                    Command::new("callback-max-gas-fee").visible_alias("cmgf").about("Get callback max gas fee of consumer contract. 0 means auto-estimating CallbackMaxGasFee as 3 times tx.gasprice of the request call, also user can set it manually by calling set-callback-gas-config")
                        .arg(Arg::new("consumer").required(true).help("consumer address in hex format"))
                ).subcommand(
                    Command::new("nonce").visible_alias("n").about("Get nonce(counting from 1, as there was no request) of consumer contract")
                        .arg(Arg::new("consumer").required(true).help("consumer address in hex format"))
                ).subcommand(
                    Command::new("last-randomness").visible_alias("lr").about("Get last randomness")
                ).subcommand(
                    Command::new("pending-request-commitment").visible_alias("prc")
                        .arg(Arg::new("request-id").required(true).help("request id in hex format"))
                        .about("Get pending commitment by request id")
                ).subcommand(
                    Command::new("adapter-config").visible_alias("ac").about("Get adapter config")
                ).subcommand(
                    Command::new("flat-fee-config").visible_alias("ffc").about("Get flat fee info about \
                    fee tiers, if global flat fee promotion is enabled and flat fee promotion global percentage and duration")
                ).subcommand(
                    Command::new("referral-config").visible_alias("rcfg").about("Get info about if referral activity is enabled and free request count for referrer and referee")
                ).subcommand(
                    Command::new("fee-tier").visible_alias("ft").about("Get fee tier based on the request count")
                        .arg(Arg::new("req-count").required(true).value_parser(value_parser!(u64)).help("request count in decimal"))
                ).subcommand(
                    Command::new("last-assigned-group-index").visible_alias("lagi").about("Get last assigned group index in randomness generation")
                ).subcommand(
                    Command::new("randomness-count").visible_alias("rc").about("Get randomness count")
                ).subcommand(
                    Command::new("cumulative-data").visible_alias("cd")
                    .about("Get cumulative data(FlatFee, CommitterReward and PartialSignatureReward) of randomness generation")
                )
                .about("Get views and events from adapter contract"),
            |args, context| Box::pin(randcast(args, context)),
        ).with_command_async(
            Command::new("stake")
                .subcommand(
                    Command::new("stake").visible_alias("s")
                        .about("Get staked arpa amount")
                ).subcommand(
                    Command::new("base-reward").visible_alias("br")
                        .about("Get amount of base rewards earned in ARPA wei")
                ).subcommand(
                    Command::new("delegation-reward").visible_alias("dr")
                    .arg(Arg::new("delegator-address").required(true).help("delegator address in hex format"))
                        .about("Get amount of delegation rewards earned by an operator in ARPA wei")
                ).subcommand(
                    Command::new("total-delegated-amount").visible_alias("tda")
                        .about("Get total delegated amount, calculated by dividing the total \
                         community staker staked amount by the delegation rate, i.e. \
                         totalDelegatedAmount = pool.totalCommunityStakedAmount / delegationRateDenominator")
                ).subcommand(
                    Command::new("delegates-count").visible_alias("dc")
                        .about("Delegates count increases after an operator is added to the list \
                         of operators and stakes the required amount.")
                ).subcommand(
                    Command::new("community-stakersCount").visible_alias("cs")
                        .about("Count all community stakers that have a staking balance greater than 0")
                ).subcommand(
                    Command::new("getTotalStakedAmount").visible_alias("tsa")
                        .about("Total amount staked by community stakers and operators in ARPA wei")
                ).subcommand(
                    Command::new("total-community-staked-amount").visible_alias("tcsa")
                        .about("Total amount staked by community stakers in ARPA wei")
                ).subcommand(
                    Command::new("total-frozen-amount").visible_alias("tfa")
                        .about("The sum of frozen operator principals that have not been \
                     withdrawn from the staking pool in ARPA wei.")
                ).subcommand(
                    Command::new("delegation-rate-denominator").visible_alias("drd")
                        .about("Get current delegation rate")
                ).subcommand(
                    Command::new("reward-rate").visible_alias("rr")
                        .about("Get current reward rate, expressed in arpa weis per second")
                ).subcommand(
                    Command::new("reward-apy").visible_alias("ra")
                        .about("Get current reward APY, expressed in percentage")
                ).subcommand(
                    Command::new("reward-timestamps").visible_alias("rt")
                        .about("Get reward initialization timestamp and reward expiry timestamp")
                ).subcommand(
                    Command::new("operator-limit").visible_alias("ol")
                        .about("Get amount that should be staked by an operator")
                ).subcommand(
                    Command::new("community-staker-limits").visible_alias("csl")
                        .about("Get minimum amount and maximum amount that can be staked by a community staker")
                ).subcommand(
                    Command::new("max-pool-size").visible_alias("mps")
                        .about("Get current maximum staking pool size")
                ).subcommand(
                    Command::new("frozen-principal").visible_alias("fp")
                        .about("Get frozen principal and unfreeze time")
                )
                .about("Get views and events from staking contract"),
            |args, context| Box::pin(stake(args, context)),
        ).with_command_async(
            Command::new("send")
                .subcommand(
                    Command::new("approve-arpa-to-staking").visible_alias("aats")
                        .arg(Arg::new("amount").required(true).help("amount of arpa to approve"))
                        .about("Approve arpa to staking contract")
                ).subcommand(
                    Command::new("stake").visible_alias("s").about("Stake arpa to staking contract")
                        .arg(Arg::new("amount").required(true).help("amount of arpa to stake"))
                ).subcommand(
                    Command::new("unstake").visible_alias("u").about("Unstake(then freeze) arpa from staking contract and claim delegation rewards instantly after exit")
                        .arg(Arg::new("amount").required(true).help("amount of arpa to unstake"))
                ).subcommand(
                    Command::new("claim-frozen-principal").visible_alias("cfp").about("Claim frozen principal from staking after unstake")
                ).subcommand(
                    Command::new("claim").visible_alias("c").about("Claim rewards as well as frozen principal(if any) from staking")
                ).subcommand(
                    Command::new("claim-reward").visible_alias("cr").about("Claim rewards from staking")
                ).subcommand(
                    Command::new("create-subscription").visible_alias("cs")
                        .about("Create a new subscription as owner")
                ).subcommand(
                    Command::new("add-consumer").visible_alias("ac")
                        .arg(Arg::new("sub-id").required(true).value_parser(value_parser!(u64)).help("subscription id in decimal format"))
                        .arg(Arg::new("consumer").required(true).help("consumer address in hex format"))
                        .about("Add consumer contract to subscription")
                ).subcommand(
                    Command::new("fund-subscription").visible_alias("fs")
                        .arg(Arg::new("sub-id").required(true).value_parser(value_parser!(u64)).help("subscription id in decimal format"))
                        .arg(Arg::new("amount").required(true).help("amount of ETH(wei) to fund"))
                        .about("Fund subscription with ETH")
                ).subcommand(
                    Command::new("set-referral").visible_alias("sr")
                        .arg(Arg::new("sub-id").required(true).value_parser(value_parser!(u64)).help("subscription id in decimal format"))
                        .arg(Arg::new("referral-sub-id").required(true).value_parser(value_parser!(u64)).help("referral subscription id in decimal format"))
                        .about("Set referral subscription id for your subscription to get referral rewards")
                ).subcommand(
                    Command::new("cancel-subscription").visible_alias("ccs")
                        .arg(Arg::new("sub-id").required(true).value_parser(value_parser!(u64)).help("subscription id in decimal format"))
                        .arg(Arg::new("recipient").required(true).help("address to send ETH left"))
                        .about("Cancel subscription and redeem ETH left to receiver address")
                ).subcommand(
                    Command::new("remove-consumer").visible_alias("rc")
                        .arg(Arg::new("sub-id").required(true).value_parser(value_parser!(u64)).help("subscription id in decimal format"))
                        .arg(Arg::new("consumer").required(true).help("consumer contract address in hex format"))
                        .about("Remove consumer contract from subscription")
                ).subcommand(
                    Command::new("set-callback-gas-config").visible_alias("scgc")
                        .arg(Arg::new("consumer").required(true).help("consumer contract address in hex format"))
                        .arg(Arg::new("consumer-owner-private-key").required(true).help("consumer contract owner private key in plain hex format, or a env var starts with $"))
                        .arg(Arg::new("callback-gas-limit").required(true).help("callback gas limit"))
                        .arg(Arg::new("callback-max-gas-fee").required(true).help("callback max gas fee"))
                        .about("Set callback gas config for consumer contract")
                )
                .about("*** Be careful this will change on-chain state and cost gas as well as block time***\nSend trxs to on-chain contracts"),
            |args, context| Box::pin(send(args, context)),
        ).with_command_async(
            Command::new("show")
                .subcommand(
                    Command::new("address").visible_alias("a")
                    .about("Show address of the wallet")
                ).subcommand(
                    Command::new("config").visible_alias("c")
                    .about("Print config")
                )
                .about("Show information of the config file"),
                |args, context| Box::pin(show(args, context)),
        ).with_on_after_command_async(|context| Box::pin(update_prompt(context)));

    repl.run_async().await?;

    Ok(())
}
