use arpa_contract_client::controller::{ControllerClientBuilder, ControllerTransactions};
use arpa_core::build_wallet_from_config;
use arpa_core::log::encoder::JsonEncoder;
use arpa_core::Config;
use arpa_core::GeneralMainChainIdentity;
use arpa_core::GeneralRelayedChainIdentity;
use arpa_core::DEFAULT_WEBSOCKET_PROVIDER_RECONNECT_TIMES;
use arpa_dal::GroupInfoHandler;
use arpa_dal::NodeInfoHandler;
use arpa_dal::{NodeInfoFetcher, NodeInfoUpdater};
use arpa_node::context::chain::types::GeneralMainChain;
use arpa_node::context::chain::types::GeneralRelayedChain;
use arpa_node::context::types::GeneralContext;
use arpa_node::context::{Context, TaskWaiter};
use arpa_sqlite_db::SqliteDB;
use ethers::providers::Provider;
use ethers::providers::Ws;
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
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use structopt::StructOpt;
use threshold_bls::schemes::bn254::G2Curve;
use threshold_bls::schemes::bn254::G2Scheme;
use threshold_bls::serialize::point_to_hex;
use threshold_bls::sig::Scheme;
use tokio::sync::RwLock;

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

    let logger_descriptor = config.get_logger_descriptor();

    init_logger(
        logger_descriptor.get_node_id(),
        logger_descriptor.get_context_logging(),
        logger_descriptor.get_log_file_path(),
        logger_descriptor.get_rolling_file_size(),
    );

    info!("{:?}", config);

    let data_path = PathBuf::from(config.get_data_path());

    let wallet = build_wallet_from_config(config.get_account())?;

    let id_address = wallet.address();

    let is_new_run = !data_path.exists();

    if let Some(parent) = data_path.parent() {
        fs::create_dir_all(parent)?;
    }

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
                    .get_node_advertised_committer_rpc_endpoint()
                    .to_string(),
                dkg_private_key,
                dkg_public_key,
            )
            .await?;

        dkg_public_key_to_register = Some(bincode::serialize(&dkg_public_key)?);
    } else {
        if let Ok(false) = node_cache.refresh_current_node_info().await {
            panic!("It seems there is no existing node record. Please check the database or remove it.");
        }

        assert_eq!(node_cache.get_id_address()?, id_address,
        "Node identity is different from the database, please check config or remove the existed database.");

        // update committer rpc endpoint according to config
        node_cache
            .set_node_rpc_endpoint(
                config
                    .get_node_advertised_committer_rpc_endpoint()
                    .to_string(),
            )
            .await?;

        group_cache.refresh_current_group_info().await?;
    }

    let node_cache: Arc<RwLock<Box<dyn NodeInfoHandler<G2Curve>>>> =
        Arc::new(RwLock::new(Box::new(node_cache)));

    let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> =
        Arc::new(RwLock::new(Box::new(group_cache)));

    let randomness_tasks_cache = Arc::new(RwLock::new(db.build_randomness_tasks_cache(0)?));

    let randomness_result_cache = Arc::new(RwLock::new(db.build_randomness_result_cache(0).await?));

    let provider = Arc::new(
        Provider::<Ws>::connect_with_reconnects(
            config.get_provider_endpoint(),
            DEFAULT_WEBSOCKET_PROVIDER_RECONNECT_TIMES,
        )
        .await?
        .interval(Duration::from_millis(
            config.get_time_limits().provider_polling_interval_millis,
        )),
    );

    let main_chain_identity = GeneralMainChainIdentity::new(
        config.get_main_chain_id(),
        wallet.clone(),
        provider,
        config.get_provider_endpoint().to_string(),
        config
            .get_controller_address()
            .parse()
            .expect("bad format of controller_address"),
        config
            .get_controller_relayer_address()
            .parse()
            .expect("bad format of controller_relayer_address"),
        config
            .get_adapter_address()
            .parse()
            .expect("bad format of adapter_address"),
        config
            .get_time_limits()
            .contract_transaction_retry_descriptor,
        config.get_time_limits().contract_view_retry_descriptor,
    );

    let main_chain = GeneralMainChain::<G2Curve, G2Scheme>::new(
        "main chain".to_string(),
        main_chain_identity.clone(),
        node_cache.clone(),
        group_cache.clone(),
        randomness_tasks_cache,
        randomness_result_cache,
        *config.get_time_limits(),
        config.get_listeners().clone(),
    );

    let relayed_chains_config = config.get_relayed_chains().clone();

    let mut context = GeneralContext::new(main_chain, config);

    for relayed_chain_config in relayed_chains_config {
        let provider = Arc::new(
            Provider::<Ws>::connect_with_reconnects(
                relayed_chain_config.get_provider_endpoint(),
                DEFAULT_WEBSOCKET_PROVIDER_RECONNECT_TIMES,
            )
            .await?
            .interval(Duration::from_millis(
                relayed_chain_config
                    .get_time_limits()
                    .provider_polling_interval_millis,
            )),
        );

        let relayed_chain_id = relayed_chain_config.get_chain_id();

        let relayed_chain_identity = GeneralRelayedChainIdentity::new(
            relayed_chain_id,
            wallet.clone(),
            provider,
            relayed_chain_config.get_provider_endpoint().to_string(),
            relayed_chain_config
                .get_controller_oracle_address()
                .parse()
                .expect("bad format of controller_oracle_address"),
            relayed_chain_config
                .get_adapter_address()
                .parse()
                .expect("bad format of adapter_address"),
            relayed_chain_config
                .get_time_limits()
                .contract_transaction_retry_descriptor,
            relayed_chain_config
                .get_time_limits()
                .contract_view_retry_descriptor,
        );

        let randomness_tasks_cache = Arc::new(RwLock::new(
            db.build_randomness_tasks_cache(relayed_chain_id)?,
        ));

        let randomness_result_cache = Arc::new(RwLock::new(
            db.build_randomness_result_cache(relayed_chain_id).await?,
        ));

        let relayed_chain = GeneralRelayedChain::<G2Curve, G2Scheme>::new(
            relayed_chain_config.get_description().to_string(),
            relayed_chain_identity,
            node_cache.clone(),
            group_cache.clone(),
            randomness_tasks_cache,
            randomness_result_cache,
            *relayed_chain_config.get_time_limits(),
            relayed_chain_config.get_listeners().to_vec(),
        );

        context.add_relayed_chain(Box::new(relayed_chain))?;
    }

    let handle = context.deploy().await?;

    if is_new_run {
        let client =
            ControllerClientBuilder::<G2Curve>::build_controller_client(&main_chain_identity);

        client
            .node_register(dkg_public_key_to_register.unwrap())
            .await?;
    }

    handle.wait_task().await;

    Ok(())
}
