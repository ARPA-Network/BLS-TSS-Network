use arpa_contract_client::controller::{ControllerClientBuilder, ControllerTransactions};
use arpa_core::log::encoder::JsonEncoder;
use arpa_core::Config;
use arpa_core::GeneralChainIdentity;
use arpa_core::{build_wallet_from_config, RandomnessTask};
use arpa_dal::cache::RandomnessResultCache;
use arpa_dal::{NodeInfoFetcher, NodeInfoUpdater};
use arpa_node::context::chain::types::GeneralMainChain;
use arpa_node::context::types::GeneralContext;
use arpa_node::context::{Context, TaskWaiter};
use arpa_sqlite_db::GroupInfoDBClient;
use arpa_sqlite_db::NodeInfoDBClient;
use arpa_sqlite_db::SqliteDB;
use arpa_sqlite_db::{BLSTasksDBClient, SignatureResultDBClient};
use ethers::signers::Signer;
use log::{info, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::rolling_file::policy::compound::roll::delete::DeleteRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::{Appender, Root};
use log4rs::filter::threshold::ThresholdFilter;
use log4rs::Config as LogConfig;
use std::path::PathBuf;
use structopt::StructOpt;
use threshold_bls::curve::bn254::PairingCurve as BN254;
use threshold_bls::schemes::bn254::G2Scheme;
use threshold_bls::serialize::point_to_hex;
use threshold_bls::sig::Scheme;

#[derive(StructOpt, Debug)]
#[structopt(name = "Arpa Node")]
pub struct Opt {
    /// Set the config path
    #[structopt(
        short = "c",
        long,
        parse(from_os_str),
        default_value = "conf/config.yml"
    )]
    config_path: PathBuf,
}

fn init_logger(node_id: &str, context_logging: bool, log_file_path: &str, rolling_file_size: u64) {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(
            JsonEncoder::new(node_id.to_string()).context_logging(context_logging),
        ))
        .build();

    let rolling_file = RollingFileAppender::builder()
        .encoder(Box::new(
            JsonEncoder::new(node_id.to_string()).context_logging(context_logging),
        ))
        .build(
            format!(
                "{}/node.log",
                if let Some(path_without_slash) = log_file_path.strip_suffix('/') {
                    path_without_slash
                } else {
                    log_file_path
                }
            ),
            Box::new(CompoundPolicy::new(
                Box::new(SizeTrigger::new(rolling_file_size)),
                Box::new(DeleteRoller::new()),
            )),
        )
        .unwrap();

    let rolling_err_file = RollingFileAppender::builder()
        .encoder(Box::new(
            JsonEncoder::new(node_id.to_string()).context_logging(context_logging),
        ))
        .build(
            format!("{}/node_err.log", log_file_path),
            Box::new(CompoundPolicy::new(
                Box::new(SizeTrigger::new(rolling_file_size)),
                Box::new(DeleteRoller::new()),
            )),
        )
        .unwrap();

    let log_config = LogConfig::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(rolling_file)))
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LevelFilter::Error)))
                .build("err_file", Box::new(rolling_err_file)),
        )
        .build(
            Root::builder()
                .appender("stdout")
                .appender("file")
                .appender("err_file")
                .build(LevelFilter::Info),
        )
        .unwrap();

    log4rs::init_config(log_config).unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    println!("{:#?}", opt);

    let config = Config::load(opt.config_path);

    init_logger(
        &config.logger.as_ref().unwrap().node_id,
        config.logger.as_ref().unwrap().context_logging,
        &config.logger.as_ref().unwrap().log_file_path,
        config.logger.as_ref().unwrap().rolling_file_size,
    );

    info!("{:?}", config);

    let data_path = PathBuf::from(config.data_path.clone().unwrap());

    let wallet = build_wallet_from_config(&config.account)?;

    let id_address = wallet.address();

    let is_new_run = !data_path.exists();

    let db = SqliteDB::build(
        data_path.as_os_str().to_str().unwrap(),
        &wallet.signer().to_bytes(),
    )
    .await?;

    let mut node_cache = db.get_node_info_client();

    let mut group_cache = db.get_group_info_client();

    let mut dkg_public_key_to_register: Option<Vec<u8>> = None;

    if is_new_run {
        let rng = &mut rand::thread_rng();

        let (dkg_private_key, dkg_public_key) = G2Scheme::keypair(rng);

        info!("dkg public_key: {}", point_to_hex(&dkg_public_key));

        node_cache
            .save_node_info(
                id_address,
                config
                    .node_advertised_committer_rpc_endpoint
                    .clone()
                    .unwrap(),
                dkg_private_key,
                dkg_public_key,
            )
            .await?;

        dkg_public_key_to_register = Some(bincode::serialize(&dkg_public_key)?);
    } else {
        if let Ok(false) = node_cache.refresh_current_node_info().await {
            panic!("It seems there is no existing node record. Please check the database or remove it.");
        }

        assert_eq!(node_cache.get_id_address()?, id_address,"Node identity is different from the database, please check config or remove the existed database.");

        // update committer rpc endpoint according to config
        node_cache
            .set_node_rpc_endpoint(
                config
                    .node_advertised_committer_rpc_endpoint
                    .clone()
                    .unwrap(),
            )
            .await?;

        group_cache.refresh_current_group_info().await?;
    }

    let randomness_tasks_cache = db.get_bls_tasks_client::<RandomnessTask>();

    let randomness_result_cache = db.get_randomness_result_client().await?;

    let main_chain_identity = GeneralChainIdentity::new(
        config.chain_id,
        wallet,
        config.provider_endpoint.clone(),
        config.time_limits.unwrap().provider_polling_interval_millis,
        config
            .controller_address
            .parse()
            .expect("bad format of controller_address"),
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

    let main_chain = GeneralMainChain::<
        NodeInfoDBClient<BN254>,
        GroupInfoDBClient<BN254>,
        BLSTasksDBClient<RandomnessTask>,
        SignatureResultDBClient<RandomnessResultCache>,
        GeneralChainIdentity,
        BN254,
    >::new(
        "main chain".to_string(),
        main_chain_identity.clone(),
        node_cache,
        group_cache,
        randomness_tasks_cache,
        randomness_result_cache,
        config.time_limits.unwrap(),
        config.listeners.clone(),
    );

    let context = GeneralContext::new(main_chain, config);

    let handle = context.deploy().await?;

    if is_new_run {
        let client =
            ControllerClientBuilder::<BN254>::build_controller_client(&main_chain_identity);

        client
            .node_register(dkg_public_key_to_register.unwrap())
            .await?;
    }

    handle.wait_task().await;

    Ok(())
}
