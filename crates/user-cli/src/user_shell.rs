use arpa_contract_client::contract_stub::adapter::Adapter as AdapterContract;
use arpa_contract_client::contract_stub::ierc20::IERC20 as ArpaContract;
use arpa_contract_client::contract_stub::staking::Staking as StakingContract;
use arpa_contract_client::ethers::adapter::AdapterClient;
use arpa_contract_client::{TransactionCaller, ViewCaller};
use arpa_core::{address_to_string, pad_to_bytes32, WalletSigner};
use arpa_user_cli::config::{build_wallet_from_config, Config};
use ethers::prelude::{NonceManagerMiddleware, SignerMiddleware};
use ethers::providers::{Http, Middleware, Provider};
use ethers::signers::Signer;
use ethers::types::{Address, BlockId, BlockNumber, U256};
use reedline_repl_rs::clap::{Arg, ArgMatches, Command};
use reedline_repl_rs::Repl;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::Arc;
use structopt::StructOpt;

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

    /// Set the block height when adapter contract deployed
    #[structopt(short = "d", long, default_value = "3632525")]
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
                Some(trx_hash)
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
                Some(trx_hash)
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
                Some(trx_hash)
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
                Some(trx_hash)
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
                Some(trx_hash)
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
                Some(trx_hash)
            )))
        }

        // let controller_contract = Controller::new(self.controller_address, self.signer.clone());
        _ => panic!("Unknown subcommand {:?}", args.subcommand_name()),
    }
}

async fn call(args: ArgMatches, context: &mut Context) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
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

async fn randcast(args: ArgMatches, context: &mut Context) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("fulfillments", _sub_matches)) => {
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let filter = adapter_contract
                .randomness_request_result_filter()
                // .topic3(context.signer.address())
                .from_block(context.adapter_deployed_block_height)
                .to_block(BlockNumber::Latest);

            let logs = filter.query().await?;

            let logs = logs
                .iter()
                .map(|log| RandomnessRequestResult {
                    request_id: hex::encode(log.request_id),
                    group_index: log.group_index,
                    committer: log.committer,
                    participant_members: log.participant_members.clone(),
                    randommness: log.randommness,
                    payment: log.payment,
                    flat_fee: log.flat_fee,
                    success: log.success,
                })
                .collect::<Vec<_>>();

            println!("{} fulfillment(s) found!", logs.iter().len());

            Ok(Some(format!("log: {:#?}", logs)))
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

        // getLastAssignedGroupIndex
        Some(("last-assigned-group-index", _sub_matches)) => {
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let last_assigned_group_index = AdapterClient::call_contract_view(
                "last_assigned_group_index",
                adapter_contract.get_last_assigned_group_index(),
                context.config.contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "last_assigned_group_index: {:#?}",
                last_assigned_group_index
            )))
        }
        // getRandomnessCount
        Some(("randomness-count", _sub_matches)) => {
            let adapter_contract =
                AdapterContract::new(context.config.adapter_address(), context.signer.clone());

            let randomness_count = AdapterClient::call_contract_view(
                "randomness_count",
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
                    Command::new("last-randomness").visible_alias("lr").about("Get last randomness")
                ).subcommand(
                    Command::new("pending-request-commitment").visible_alias("prc")
                        .arg(Arg::new("request-id").required(true).help("request id in hex format"))
                        .about("Get pending commitment by request id")
                ).subcommand(
                    Command::new("adapter-config").visible_alias("ac").about("Get adapter config")
                ).subcommand(
                    Command::new("last-assigned-group-index").visible_alias("lagi").about("Get last assigned group index in randomness generation")
                ).subcommand(
                    Command::new("randomness-count").visible_alias("rc").about("Get randomness count")
                ).subcommand(
                    Command::new("cumulative-data").visible_alias("cd")
                    .about("Get cumulative data(FlatFee, CommitterReward and PartialSignatureReward) of randomness generation")
                )
                .about("Get views from adapter contract"),
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
                .about("Get views from staking contract"),
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
