use arpa_contract_client::adapter::{AdapterClientBuilder, AdapterViews};
use arpa_contract_client::contract_stub::adapter::Adapter as AdapterContract;
use arpa_contract_client::contract_stub::controller::Controller as ControllerContract;
use arpa_contract_client::contract_stub::ierc20::IERC20 as ArpaContract;
use arpa_contract_client::contract_stub::staking::Staking as StakingContract;
use arpa_contract_client::controller::{
    ControllerClientBuilder, ControllerTransactions, ControllerViews,
};
use arpa_contract_client::ethers::adapter::AdapterClient;
use arpa_contract_client::ethers::controller::ControllerClient;
use arpa_contract_client::{ServiceClient, TransactionCaller, ViewCaller};
use arpa_core::{
    address_to_string, build_wallet_from_config, pad_to_bytes32, ChainIdentity, Config,
    GeneralMainChainIdentity, WalletSigner,
};
use arpa_dal::NodeInfoFetcher;
use arpa_node::management::client::GeneralManagementClient;
use arpa_sqlite_db::SqliteDB;
use ethers::prelude::k256::ecdsa::SigningKey;
use ethers::providers::Middleware;
use ethers::signers::coins_bip39::English;
use ethers::signers::Signer;
use ethers::signers::{LocalWallet, MnemonicBuilder};
use ethers::types::{Address, BlockId, BlockNumber, H256, U256, U64};
use reedline_repl_rs::clap::{self, Arg, ArgAction, ArgMatches, Command};
use reedline_repl_rs::Repl;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use structopt::StructOpt;
use threshold_bls::curve::bn254::G2Curve;
use threshold_bls::serialize::scalar_to_hex;

pub const MAX_HISTORY_CAPACITY: usize = 1000;
pub const DEFAULT_PROMPT: &str = "ARPA Node CLI";

#[derive(StructOpt, Debug)]
#[structopt(name = DEFAULT_PROMPT)]
pub struct Opt {
    /// Set the config path
    #[structopt(
        short = "c",
        long,
        parse(from_os_str),
        default_value = "conf/config.yml"
    )]
    config_path: PathBuf,

    /// Set the history file path
    #[structopt(
        short = "H",
        long,
        parse(from_os_str),
        default_value = "node-shell.history"
    )]
    history_file_path: PathBuf,

    /// Set the block height when adapter contract deployed to accelerate the query of events
    #[structopt(short = "d", long, default_value = "0")]
    adapter_deployed_block_height: u64,
}

struct Context {
    config: Config,
    wallet: LocalWallet,
    main_chain_identity: GeneralMainChainIdentity,
    db: SqliteDB,
    staking_contract_address: Address,
    arpa_contract_address: Address,
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

            let arpa_contract = ArpaContract::new(
                context.arpa_contract_address,
                context.main_chain_identity.get_signer(),
            );

            let trx_hash = ArpaClient::call_contract_transaction(
                context.config.chain_id,
                "approve-arpa-to-staking",
                arpa_contract.approve(context.staking_contract_address, amount),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_transaction_retry_descriptor,
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

            let staking_contract = StakingContract::new(
                context.staking_contract_address,
                context.main_chain_identity.get_signer(),
            );

            let is_operator = StakingClient::call_contract_view(
                context.config.chain_id,
                "is_operator",
                staking_contract.is_operator(context.main_chain_identity.get_id_address()),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            if !is_operator {
                return Ok(Some(
                    "Node is not operator, please contact us first".to_string(),
                ));
            }

            let arpa_contract = ArpaContract::new(
                context.arpa_contract_address,
                context.main_chain_identity.get_signer(),
            );

            let balance = ArpaClient::call_contract_view(
                context.config.chain_id,
                "balance_of",
                arpa_contract.balance_of(context.main_chain_identity.get_id_address()),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            if balance < amount {
                return Ok(Some(format!(
                    "Insufficient balance, balance: {}, amount: {}",
                    balance, amount
                )));
            }

            let allowance = ArpaClient::call_contract_view(
                context.config.chain_id,
                "allowance",
                arpa_contract.allowance(
                    context.main_chain_identity.get_id_address(),
                    context.staking_contract_address,
                ),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            if allowance < amount {
                return Ok(Some(format!(
                    "Insufficient allowance, allowance: {}, amount: {}",
                    allowance, amount
                )));
            }

            let trx_hash = StakingClient::call_contract_transaction(
                context.config.chain_id,
                "stake",
                staking_contract.stake(amount),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_transaction_retry_descriptor,
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

            let staking_contract = StakingContract::new(
                context.staking_contract_address,
                context.main_chain_identity.get_signer(),
            );

            let staked_amount = StakingClient::call_contract_view(
                context.config.chain_id,
                "staked_amount",
                staking_contract.get_stake(context.main_chain_identity.get_id_address()),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            if staked_amount < amount {
                return Ok(Some(format!(
                    "Insufficient staked amount, staked amount: {}, amount: {}",
                    staked_amount, amount
                )));
            }

            let trx_hash = StakingClient::call_contract_transaction(
                context.config.chain_id,
                "unstake",
                staking_contract.unstake(amount),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Unstake arpa successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("claim-frozen-principal", _sub_matches)) => {
            let staking_contract = StakingContract::new(
                context.staking_contract_address,
                context.main_chain_identity.get_signer(),
            );

            let trx_hash = StakingClient::call_contract_transaction(
                context.config.chain_id,
                "claim_frozen_principal",
                staking_contract.claim_frozen_principal(),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Claim frozen principal successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("register", _sub_matches)) => {
            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let node =
                ControllerViews::<G2Curve>::get_node(&client, context.wallet.address()).await?;

            if node.id_address != Address::zero() {
                return Ok(Some("Node already registered".to_string()));
            }

            let mut node_cache = context.db.get_node_info_client::<G2Curve>();

            node_cache.refresh_current_node_info().await?;

            let dkg_public_key = node_cache.get_dkg_public_key()?;

            let trx_hash = client
                .node_register(bincode::serialize(&dkg_public_key)?)
                .await?;

            Ok(Some(format!(
                "Register node successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("activate", _sub_matches)) => {
            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let node =
                ControllerViews::<G2Curve>::get_node(&client, context.wallet.address()).await?;

            if node.id_address == Address::zero() {
                return Ok(Some("Node has not registered".to_string()));
            }

            if node.state {
                return Ok(Some("Node already activated".to_string()));
            }

            let controller_contract =
                ServiceClient::<ControllerContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let trx_hash = ControllerClient::call_contract_transaction(
                context.config.chain_id,
                "node_activate",
                controller_contract.node_activate(),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Activate node successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("quit", _sub_matches)) => {
            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let node =
                ControllerViews::<G2Curve>::get_node(&client, context.wallet.address()).await?;

            if node.id_address == Address::zero() {
                return Ok(Some("Node has not registered".to_string()));
            }

            let controller_contract =
                ServiceClient::<ControllerContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let trx_hash = ControllerClient::call_contract_transaction(
                context.config.chain_id,
                "node_quit",
                controller_contract.node_quit(),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Quit node successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("change-dkg-public-key", _sub_matches)) => {
            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let node =
                ControllerViews::<G2Curve>::get_node(&client, context.wallet.address()).await?;

            if node.id_address == Address::zero() {
                return Ok(Some("Node has not registered".to_string()));
            }

            if node.state {
                return Ok(Some("Node already activated".to_string()));
            }

            let mut node_cache = context.db.get_node_info_client::<G2Curve>();

            node_cache.refresh_current_node_info().await?;

            let dkg_public_key = node_cache.get_dkg_public_key()?;

            let controller_contract =
                ServiceClient::<ControllerContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let trx_hash = ControllerClient::call_contract_transaction(
                context.config.chain_id,
                "change_dkg_public_key",
                controller_contract
                    .change_dkg_public_key(bincode::serialize(&dkg_public_key)?.into()),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Change dkg public key of the node successfully, transaction hash: {:?}",
                trx_hash
            )))
        }
        Some(("withdraw", sub_matches)) => {
            let recipient = sub_matches.get_one::<String>("recipient").unwrap();

            let recipient = recipient.parse::<Address>()?;

            if recipient == Address::zero() {
                return Ok(Some("Invalid recipient address".to_string()));
            }

            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let node =
                ControllerViews::<G2Curve>::get_node(&client, context.wallet.address()).await?;

            if node.id_address == Address::zero() {
                return Ok(Some("Node has not registered".to_string()));
            }

            let controller_contract =
                ServiceClient::<ControllerContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let trx_hash = ControllerClient::call_contract_transaction(
                context.config.chain_id,
                "node_withdraw",
                controller_contract.node_withdraw(recipient),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_transaction_retry_descriptor,
                true,
            )
            .await?;

            Ok(Some(format!(
                "Withdraw node balance successfully, transaction hash: {:?}",
                trx_hash
            )))
        }

        _ => panic!("Unknown subcommand {:?}", args.subcommand_name()),
    }
}

async fn call(args: ArgMatches, context: &mut Context) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        // getNode
        Some(("node", sub_matches)) => {
            let id_address = sub_matches.get_one::<String>("id-address").unwrap();

            let id_address = id_address.parse::<Address>()?;

            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let node = ControllerViews::<G2Curve>::get_node(&client, id_address).await?;

            Ok(Some(format!("{:#?}", node)))
        }
        // getGroup
        Some(("group", sub_matches)) => {
            let group_index = sub_matches.get_one::<usize>("group-index").unwrap();

            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let group = ControllerViews::<G2Curve>::get_group(&client, *group_index).await?;

            Ok(Some(format!("{:#?}", group)))
        }
        // getValidGroupIndices
        Some(("valid-group-indices", _sub_matches)) => {
            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let controller_contract =
                ServiceClient::<ControllerContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let valid_group_indices = ControllerClient::call_contract_view(
                context.config.chain_id,
                "valid_group_indices",
                controller_contract.get_valid_group_indices(),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("{:#?}", valid_group_indices)))
        }
        // getGroupEpoch
        Some(("group-epoch", _sub_matches)) => {
            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let controller_contract =
                ServiceClient::<ControllerContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let group_epoch = ControllerClient::call_contract_view(
                context.config.chain_id,
                "group_epoch",
                controller_contract.get_group_epoch(),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("{:#?}", group_epoch)))
        }
        // getGroupCount
        Some(("group-count", _sub_matches)) => {
            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let controller_contract =
                ServiceClient::<ControllerContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let group_count = ControllerClient::call_contract_view(
                context.config.chain_id,
                "group_count",
                controller_contract.get_group_count(),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("{:#?}", group_count)))
        }
        // getBelongingGroup
        Some(("belonging-group", sub_matches)) => {
            let node_address = sub_matches.get_one::<String>("id-address").unwrap();

            let node_address = node_address.parse::<Address>()?;

            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let controller_contract =
                ServiceClient::<ControllerContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let (belonging_group_index, member_index) = ControllerClient::call_contract_view(
                context.config.chain_id,
                "belonging_group",
                controller_contract.get_belonging_group(node_address),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "belonging_group_index: {:#?}, member_index: {:#?}",
                belonging_group_index, member_index
            )))
        }
        // getMember
        Some(("member", sub_matches)) => {
            let group_index = sub_matches.get_one::<usize>("group-index").unwrap();

            let member_index = sub_matches.get_one::<usize>("member-index").unwrap();

            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let controller_contract =
                ServiceClient::<ControllerContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let member = ControllerClient::call_contract_view(
                context.config.chain_id,
                "member",
                controller_contract.get_member((*group_index).into(), (*member_index).into()),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("{:#?}", member)))
        }
        // getCoordinator
        Some(("coordinator", sub_matches)) => {
            let group_index = sub_matches.get_one::<usize>("group-index").unwrap();

            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let controller_contract =
                ServiceClient::<ControllerContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let coordinator = ControllerClient::call_contract_view(
                context.config.chain_id,
                "coordinator",
                controller_contract.get_coordinator((*group_index).into()),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("{:#?}", coordinator)))
        }
        // getNodeWithdrawableTokens
        Some(("node-withdrawable-tokens", sub_matches)) => {
            let node_address = sub_matches.get_one::<String>("id-address").unwrap();

            let node_address = node_address.parse::<Address>()?;

            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let controller_contract =
                ServiceClient::<ControllerContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let (node_withdrawable_eth, node_withdrawable_arpa) =
                ControllerClient::call_contract_view(
                    context.config.chain_id,
                    "node_withdrawable_tokens",
                    controller_contract.get_node_withdrawable_tokens(node_address),
                    context
                        .config
                        .time_limits
                        .unwrap()
                        .contract_view_retry_descriptor,
                )
                .await?;

            Ok(Some(format!(
                "node_withdrawable_eth: {:#?}, node_withdrawable_arpa: {:#?}",
                node_withdrawable_eth, node_withdrawable_arpa
            )))
        }
        // getControllerConfig
        Some(("controller-config", _sub_matches)) => {
            let client = ControllerClientBuilder::<G2Curve>::build_controller_client(
                &context.main_chain_identity,
            );

            let controller_contract =
                ServiceClient::<ControllerContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let (
                staking_contract_address,
                adapter_contract_address,
                node_staking_amount,
                disqualified_node_penalty_amount,
                default_number_of_committers,
                default_dkg_phase_duration,
                group_max_capacity,
                ideal_number_of_groups,
                pending_block_after_quit,
                dkg_post_process_reward,
            ) = ControllerClient::call_contract_view(
                context.config.chain_id,
                "controller_config",
                controller_contract.get_controller_config(),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("staking_contract_address: {:#?}, adapter_contract_address: {:#?}, node_staking_amount: {:#?}, \
            disqualified_node_penalty_amount: {:#?}, default_number_of_committers: {:#?}, default_dkg_phase_duration: {:#?}, \
            group_max_capacity: {:#?}, ideal_number_of_groups: {:#?}, pending_block_after_quit: {:#?}, dkg_post_process_reward: {:#?}",  
            staking_contract_address,
            adapter_contract_address,
            node_staking_amount,
            disqualified_node_penalty_amount,
            default_number_of_committers,
            default_dkg_phase_duration,
            group_max_capacity,
            ideal_number_of_groups,
            pending_block_after_quit,
            dkg_post_process_reward,)))
        }
        Some(("fulfillments-as-committer", _sub_matches)) => {
            let client = AdapterClientBuilder::build_adapter_client(
                &context.main_chain_identity,
                context.main_chain_identity.get_id_address(),
            );

            let adapter_contract =
                ServiceClient::<AdapterContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let filter = adapter_contract
                .randomness_request_result_filter()
                .address(ethers::types::ValueOrArray::Value(
                    context.main_chain_identity.get_adapter_address(),
                ))
                .topic3(context.main_chain_identity.get_id_address())
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
        Some(("fulfillments-as-participant", _sub_matches)) => {
            let client = AdapterClientBuilder::build_adapter_client(
                &context.main_chain_identity,
                context.main_chain_identity.get_id_address(),
            );

            let adapter_contract =
                ServiceClient::<AdapterContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let filter = adapter_contract
                .randomness_request_result_filter()
                .address(ethers::types::ValueOrArray::Value(
                    context.main_chain_identity.get_adapter_address(),
                ))
                .from_block(context.adapter_deployed_block_height)
                .to_block(BlockNumber::Latest);

            let logs = filter.query().await?;

            let logs = logs
                .iter()
                .filter(|log| {
                    log.participant_members
                        .contains(&context.main_chain_identity.get_id_address())
                })
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

            println!("{} fulfillment(s) found!", logs.len());

            Ok(Some(format!("log: {:#?}", logs)))
        }
        Some(("delegation-reward", _sub_matches)) => {
            let staking_contract = StakingContract::new(
                context.staking_contract_address,
                context.main_chain_identity.get_signer(),
            );

            let delegation_reward = StakingClient::call_contract_view(
                context.config.chain_id,
                "delegation_reward",
                staking_contract
                    .get_delegation_reward(context.main_chain_identity.get_id_address()),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("delegation_reward: {:#?}", delegation_reward)))
        }
        Some(("delegates-count", _sub_matches)) => {
            let staking_contract = StakingContract::new(
                context.staking_contract_address,
                context.main_chain_identity.get_signer(),
            );

            let delegates_count = StakingClient::call_contract_view(
                context.config.chain_id,
                "delegates_count",
                staking_contract.get_delegates_count(),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("delegation_count: {:#?}", delegates_count)))
        }
        Some(("stake", _sub_matches)) => {
            let staking_contract = StakingContract::new(
                context.staking_contract_address,
                context.main_chain_identity.get_signer(),
            );

            let amount = StakingClient::call_contract_view(
                context.config.chain_id,
                "get_stake",
                staking_contract.get_stake(context.main_chain_identity.get_id_address()),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("staked amount: {:#?}", amount)))
        }
        Some(("frozen-principal", _sub_matches)) => {
            let staking_contract = StakingContract::new(
                context.staking_contract_address,
                context.main_chain_identity.get_signer(),
            );

            let (amounts, timestamps) = StakingClient::call_contract_view(
                context.config.chain_id,
                "frozen_principal",
                staking_contract.get_frozen_principal(context.main_chain_identity.get_id_address()),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "amounts: {:#?}, unfreeze timestamps: {:#?}",
                amounts, timestamps
            )))
        }
        Some(("balance-of-eth", _sub_matches)) => {
            let provider = context.main_chain_identity.get_provider();

            let balance = provider
                .get_balance(
                    context.main_chain_identity.get_id_address(),
                    Some(BlockId::Number(BlockNumber::Latest)),
                )
                .await?;

            Ok(Some(format!("balance: {:#?}", balance)))
        }
        Some(("balance-of-arpa", _sub_matches)) => {
            let arpa_contract = ArpaContract::new(
                context.arpa_contract_address,
                context.main_chain_identity.get_signer(),
            );

            let balance = ArpaClient::call_contract_view(
                context.config.chain_id,
                "balance_of",
                arpa_contract.balance_of(context.main_chain_identity.get_id_address()),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("balance: {:#?}", balance)))
        }
        // getAdapterConfig
        Some(("adapter-config", _sub_matches)) => {
            let client = AdapterClientBuilder::build_adapter_client(
                &context.main_chain_identity,
                context.main_chain_identity.get_id_address(),
            );

            let adapter_contract =
                ServiceClient::<AdapterContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let (
                minimum_request_confirmations,
                max_gas_limit,
                gas_after_payment_calculation,
                gas_except_callback,
                signature_task_exclusive_window,
                reward_per_signature,
                committer_reward_per_signature,
            ) = AdapterClient::call_contract_view(
                context.config.chain_id,
                "adapter_config",
                adapter_contract.get_adapter_config(),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
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
            let client = AdapterClientBuilder::build_adapter_client(
                &context.main_chain_identity,
                context.main_chain_identity.get_id_address(),
            );

            let adapter_contract =
                ServiceClient::<AdapterContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let last_assigned_group_index = AdapterClient::call_contract_view(
                context.config.chain_id,
                "last_assigned_group_index",
                adapter_contract.get_last_assigned_group_index(),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!(
                "last_assigned_group_index: {:#?}",
                last_assigned_group_index
            )))
        }
        // getRandomnessCount
        Some(("randomness-count", _sub_matches)) => {
            let client = AdapterClientBuilder::build_adapter_client(
                &context.main_chain_identity,
                context.main_chain_identity.get_id_address(),
            );

            let adapter_contract =
                ServiceClient::<AdapterContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let randomness_count = AdapterClient::call_contract_view(
                context.config.chain_id,
                "randomness_count",
                adapter_contract.get_randomness_count(),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("randomness_count: {:#?}", randomness_count)))
        }
        Some(("block", sub_matches)) => {
            let block_number = sub_matches.get_one::<String>("block-number").unwrap();
            match block_number.as_str() {
                "latest" => {
                    let block: Option<Block> = context
                        .main_chain_identity
                        .get_provider()
                        .get_block(BlockNumber::Latest)
                        .await?
                        .map(|block| block.into());
                    return Ok(Some(format!("block: {:#?}", block)));
                }
                "earliest" => {
                    let block: Option<Block> = context
                        .main_chain_identity
                        .get_provider()
                        .get_block(BlockNumber::Earliest)
                        .await?
                        .map(|block| block.into());
                    return Ok(Some(format!("block: {:#?}", block)));
                }
                "pending" => {
                    let block: Option<Block> = context
                        .main_chain_identity
                        .get_provider()
                        .get_block(BlockNumber::Pending)
                        .await?
                        .map(|block| block.into());
                    return Ok(Some(format!("block: {:#?}", block)));
                }
                _ => {
                    if let Ok(block_number) = block_number.parse::<u64>() {
                        let block: Option<Block> = context
                            .main_chain_identity
                            .get_provider()
                            .get_block(BlockNumber::Number(block_number.into()))
                            .await?
                            .map(|block| block.into());
                        return Ok(Some(format!("block: {:#?}", block)));
                    }
                }
            }
            panic!("Unknown block number {:?}", block_number);
        }
        // current-gas-price
        Some(("current-gas-price", _sub_matches)) => {
            let gas_price = context
                .main_chain_identity
                .get_provider()
                .get_gas_price()
                .await?;

            Ok(Some(format!("current gas price: {:#?}", gas_price)))
        }
        // trx-receipt
        Some(("trx-receipt", sub_matches)) => {
            let trx_hash = sub_matches.get_one::<String>("trx-hash").unwrap();

            let receipt = context
                .main_chain_identity
                .get_provider()
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
        // getCumulativeData
        Some(("cumulative-data", _sub_matches)) => {
            let client = AdapterClientBuilder::build_adapter_client(
                &context.main_chain_identity,
                context.main_chain_identity.get_id_address(),
            );

            let adapter_contract =
                ServiceClient::<AdapterContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let (
                cumulative_flat_fee,
                cumulative_committer_reward,
                cumulative_partial_signature_reward,
            ) = AdapterClient::call_contract_view(
                context.config.chain_id,
                "cumulative_data",
                adapter_contract.get_cumulative_data(),
                context
                    .config
                    .time_limits
                    .unwrap()
                    .contract_view_retry_descriptor,
            )
            .await?;

            Ok(Some(format!("cumulativeFlatFee: {:#?}, cumulativeCommitterReward: {:#?}, cumulativePartialSignatureReward: {:#?}", 
            cumulative_flat_fee, cumulative_committer_reward, cumulative_partial_signature_reward)))
        }
        Some(("last-randomness", _sub_matches)) => {
            let client = AdapterClientBuilder::build_adapter_client(
                &context.main_chain_identity,
                context.main_chain_identity.get_id_address(),
            );

            let last_randomness = client.get_last_randomness().await?;

            Ok(Some(last_randomness.to_string()))
        }
        Some(("pending-request-commitment", sub_matches)) => {
            let client = AdapterClientBuilder::build_adapter_client(
                &context.main_chain_identity,
                context.main_chain_identity.get_id_address(),
            );

            let adapter_contract =
                ServiceClient::<AdapterContract<WalletSigner>>::prepare_service_client(&client)
                    .await?;

            let r_id = sub_matches.get_one::<String>("request-id").unwrap();

            let pending_request_commitment = adapter_contract
                .get_pending_request_commitment(pad_to_bytes32(&hex::decode(r_id)?).unwrap())
                .await?;

            Ok(Some(hex::encode(pending_request_commitment)))
        }

        _ => panic!("Unknown subcommand {:?}", args.subcommand_name()),
    }
}

fn generate(args: ArgMatches, _context: &mut Context) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("private-key", _sub_matches)) => {
            let mut rng = rand::thread_rng();

            let pk = SigningKey::random(&mut rng).to_bytes();

            Ok(Some(hex::encode(pk)))
        }
        Some(("keystore", sub_matches)) => {
            let path = sub_matches.get_one::<PathBuf>("path").unwrap();
            let password = sub_matches.get_one::<String>("password").unwrap();
            let name = sub_matches.get_one::<String>("name");

            let mut rng = rand::thread_rng();
            LocalWallet::new_keystore(path, &mut rng, password, name.map(|x| &**x))?;

            Ok(Some("keystore generated successfully.".to_owned()))
        }
        Some(("hd-wallet", sub_matches)) => {
            let path = sub_matches.get_one::<PathBuf>("path").unwrap();
            let derivation_path: &str = sub_matches
                .get_one::<String>("derivation-path")
                .map_or("m/44'/60'/0'/0/0", |s| s);
            let password = sub_matches.get_one::<String>("password").unwrap();

            let mut rng = rand::thread_rng();
            MnemonicBuilder::<English>::default()
                .word_count(12)
                .derivation_path(derivation_path)?
                .write_to(path)
                .password(password)
                .build_random(&mut rng)?;

            Ok(Some("Mnemonic generated successfully.".to_owned()))
        }

        _ => panic!("Unknown subcommand {:?}", args.subcommand_name()),
    }
}

async fn show(args: ArgMatches, context: &mut Context) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("address", _sub_matches)) => {
            context.show_address = true;
            Ok(Some(address_to_string(context.wallet.address())))
        }
        Some(("config", _sub_matches)) => Ok(Some(format!("{:#?}", context.config))),
        Some(("node", sub_matches)) => {
            let display_sensitive = sub_matches.get_flag("display-sensitive");

            let mut node_cache = context.db.get_node_info_client::<G2Curve>();

            node_cache.refresh_current_node_info().await?;

            Ok(Some(if display_sensitive {
                format!(
                    "{:#?} \n dkg_private_key: {:#?}",
                    node_cache,
                    scalar_to_hex(node_cache.get_dkg_private_key()?)
                )
            } else {
                format!("{:#?}", node_cache)
            }))
        }

        _ => panic!("Unknown subcommand {:?}", args.subcommand_name()),
    }
}

async fn inspect(args: ArgMatches, context: &mut Context) -> anyhow::Result<Option<String>> {
    match args.subcommand() {
        Some(("list-fixed-tasks", _sub_matches)) => {
            let management_client = GeneralManagementClient::new(
                context.config.node_management_rpc_endpoint.to_owned(),
                context.config.node_management_rpc_token.clone(),
            );
            Ok(Some(format!(
                "fixed-tasks: {:#?}",
                management_client.list_fixed_tasks().await?
            )))
        }
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
        address_to_string(context.main_chain_identity.get_id_address())
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

    let wallet = build_wallet_from_config(&config.account)?;

    let main_chain_identity = GeneralMainChainIdentity::new(
        config.chain_id,
        wallet.clone(),
        config.provider_endpoint.clone(),
        config.time_limits.unwrap().provider_polling_interval_millis,
        config
            .controller_address
            .parse()
            .expect("bad format of controller_address"),
        config
            .controller_relayer_address
            .parse()
            .expect("bad format of controller_relayer_address"),
        config
            .adapter_address
            .parse()
            .expect("bad format of adapter_address"),
        config
            .time_limits
            .unwrap()
            .contract_transaction_retry_descriptor,
        config.time_limits.unwrap().contract_view_retry_descriptor,
    );

    let db = SqliteDB::build(
        PathBuf::from(config.data_path.clone().unwrap())
            .as_os_str()
            .to_str()
            .unwrap(),
        &wallet.signer().to_bytes(),
    )
    .await
    .unwrap();

    let client = ControllerClientBuilder::<G2Curve>::build_controller_client(&main_chain_identity);

    let controller_contract =
        ServiceClient::<ControllerContract<WalletSigner>>::prepare_service_client(&client).await?;

    let staking_contract_address = ControllerClient::call_contract_view(
        config.chain_id,
        "controller_config",
        controller_contract.get_controller_config(),
        config.time_limits.unwrap().contract_view_retry_descriptor,
    )
    .await?
    .0;

    let staking_contract =
        StakingContract::new(staking_contract_address, main_chain_identity.get_signer());
    let arpa_contract_address = StakingClient::call_contract_view(
        config.chain_id,
        "get_arpa_token",
        staking_contract.get_arpa_token(),
        config.time_limits.unwrap().contract_view_retry_descriptor,
    )
    .await?;

    let context = Context {
        config,
        wallet,
        main_chain_identity,
        db,
        staking_contract_address,
        arpa_contract_address,
        adapter_deployed_block_height: opt.adapter_deployed_block_height,
        show_address: false,
        history_file_path: opt.history_file_path.clone(),
    };

    let mut repl = Repl::new(context)
        .with_name("ARPA Node CLI")
        .with_history(opt.history_file_path, MAX_HISTORY_CAPACITY)
        .with_version("v0.0.1")
        .with_description("ARPA Node CLI is a fast and verbose REPL for the operator of a ARPA node.")
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
                    Command::new("last-randomness").visible_alias("lr").about("Get last randomness")
                ).subcommand(
                    Command::new("pending-request-commitment").visible_alias("prc")
                        .arg(Arg::new("request-id").required(true).help("request id in hex format"))
                        .about("Get pending commitment by request id")
                ).subcommand(
                    Command::new("controller-config").visible_alias("cc").about("Get controller config")
                ).subcommand(
                    Command::new("adapter-config").visible_alias("ac").about("Get adapter config")
                ).subcommand(
                    Command::new("last-assigned-group-index").visible_alias("lagi").about("Get last assigned group index in randomness generation")
                ).subcommand(
                    Command::new("randomness-count").visible_alias("rc").about("Get randomness count")
                ).subcommand(
                    Command::new("cumulative-data").visible_alias("cd")
                    .about("Get cumulative data(FlatFee, CommitterReward and PartialSignatureReward) of randomness generation")
                ).subcommand(
                    Command::new("fulfillments-as-committer").visible_alias("fac")
                        .about("Get all fulfillment events as committer in history")
                ).subcommand(
                    Command::new("fulfillments-as-participant").visible_alias("fap")
                        .about("Get all fulfillment events as participant in history")
                ).subcommand(
                    Command::new("node").visible_alias("n")
                        .arg(Arg::new("id-address").required(true).help("node id address in hex format"))
                        .about("Get node info by id address")
                ).subcommand(
                    Command::new("group").visible_alias("g")
                        .arg(Arg::new("group-index").required(true).help("group index")
                          .value_parser(clap::value_parser!(usize)))
                        .about("Get group info by index")
                ).subcommand(
                    Command::new("valid-group-indices").visible_alias("vgi")
                        .about("Get valid group indices which are ready for randomness generation")
                ).subcommand(
                    Command::new("group-epoch").visible_alias("ge")
                        .about("Get global group epoch")
                ).subcommand(
                    Command::new("group-count").visible_alias("gc")
                        .about("Get global group count")
                ).subcommand(
                    Command::new("belonging-group").visible_alias("bg")
                        .arg(Arg::new("id-address").required(true).help("node id address in hex format"))
                        .about("Get the group index and member index of a given node")
                ).subcommand(
                    Command::new("member").visible_alias("m")
                        .arg(Arg::new("group-index").required(true).help("group index")
                          .value_parser(clap::value_parser!(usize)))
                        .arg(Arg::new("member-index").required(true).help("member index")
                          .value_parser(clap::value_parser!(usize)))
                        .about("Get group member info by group index and member index")
                ).subcommand(
                    Command::new("coordinator").visible_alias("c")
                        .arg(Arg::new("group-index").required(true).help("group index")
                          .value_parser(clap::value_parser!(usize)))
                        .about("Get group coordinator during a running dkg process by group index")
                ).subcommand(
                    Command::new("node-withdrawable-tokens").visible_alias("nwt")
                        .arg(Arg::new("id-address").required(true).help("node id-address in hex format"))
                        .about("Get node withdrawable tokens(eth and arpa rewards) by id-address")
                ).subcommand(
                    Command::new("stake").visible_alias("s")
                        .about("Get node staked arpa amount")
                ).subcommand(
                    Command::new("delegation-reward").visible_alias("dr")
                        .about("Get node delegation reward")
                ).subcommand(
                    Command::new("delegates-count").visible_alias("dc")
                        .about("Get eligible nodes count")
                ).subcommand(
                    Command::new("balance-of-arpa").visible_alias("boa")
                        .about("Get balance of arpa")
                ).subcommand(
                    Command::new("frozen-principal").visible_alias("fp")
                        .about("Get frozen principal and unfreeze time")
                )
                .about("Get views and events from on-chain contracts"),
            |args, context| Box::pin(call(args, context)),
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
                    Command::new("register").visible_alias("r").about("Register node to Randcast network")
                ).subcommand(
                    Command::new("activate").visible_alias("a").about("Activate node after exit or slashing")
                ).subcommand(
                    Command::new("quit").visible_alias("q").about("Quit node from Randcast network")
                ).subcommand(
                    Command::new("change-dkg-public-key").visible_alias("cdpk")
                    .about("Change dkg public key(recorded in node database) after exit or slashing")
                ).subcommand(
                    Command::new("withdraw").visible_alias("w")
                    .arg(Arg::new("recipient").required(true).help("path to keystore file"))
                    .about("Withdraw node reward to any address"))
                .about("*** Be careful this will change on-chain state and cost gas as well as block time***\nSend trxs to on-chain contracts"),
            |args, context| Box::pin(send(args, context)),
        ).with_command(
            Command::new("generate")
                .subcommand(
                    Command::new("private-key").visible_alias("pk")
                    .about("Generate private key(not recommended)")
                )
                .subcommand(
                    Command::new("keystore").visible_alias("k")
                    .arg(Arg::new("path").required(true).help("path to keystore file")
                      .value_parser(clap::value_parser!(PathBuf)))
                    .arg(Arg::new("password").required(true).help("password to encrypt keystore file"))
                    .arg(Arg::new("name").required(false).help("file name"))
                    .about("Generate keystore file")
                ).subcommand(
                    Command::new("hd-wallet").visible_alias("hw") 
                    .arg(Arg::new("path").required(true).help("path to mnemonic file")
                      .value_parser(clap::value_parser!(PathBuf)))
                    .arg(Arg::new("password").required(true).help("password to encrypt hd-wallet"))
                    .arg(Arg::new("derivation-path").required(false).help("derivation path, default is m/44'/60'/0'/0/0"))
                    .about("Generate hierarchical deterministic wallet and save the mnemonic to a file")
                )
                .about("Generate node identity(wallet) corresponding to ARPA node format"),
            generate
        ).with_command_async(
            Command::new("show")
                .subcommand(
                    Command::new("address").visible_alias("a")
                    .about("Show address of the node identity(wallet)")
                ).subcommand(
                    Command::new("config").visible_alias("c")
                    .about("Print node config")
                ).subcommand(
                    Command::new("node").visible_alias("n")
                    .arg(Arg::new("display-sensitive").short('s').long("display-sensitive")
                      .value_parser(clap::value_parser!(bool)).action(ArgAction::SetTrue).required(false).help("display sensitive info"))
                    .about("Print node info from node database")
                )
                .about("Show information of the config file and node database"),
                |args, context| Box::pin(show(args, context)),
        ).with_command_async(
            Command::new("inspect")
                .subcommand(
                    Command::new("list-fixed-tasks").visible_alias("lft")
                    .about("List fixed tasks of the node")
                )
                .about("Connect to the node client and inspect the node status"),
                |args, context| Box::pin(inspect(args, context)),
        ).with_on_after_command_async(|context| Box::pin(update_prompt(context)));

    repl.run_async().await?;

    Ok(())
}
