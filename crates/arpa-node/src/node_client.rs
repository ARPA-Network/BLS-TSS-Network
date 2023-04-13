use arpa_node::node::context::chain::types::GeneralMainChain;
use arpa_node::node::context::types::GeneralContext;
use arpa_node::node::context::{Context, TaskWaiter};
use arpa_node_contract_client::controller::{ControllerClientBuilder, ControllerTransactions};
use arpa_node_core::log::encoder::JsonEncoder;
use arpa_node_core::{build_wallet_from_config, RandomnessTask};
use arpa_node_core::{format_now_date, Config};
use arpa_node_core::{GeneralChainIdentity, CONFIG};
use arpa_node_dal::NodeInfoFetcher;
use arpa_node_sqlite_db::BLSTasksDBClient;
use arpa_node_sqlite_db::GroupInfoDBClient;
use arpa_node_sqlite_db::NodeInfoDBClient;
use arpa_node_sqlite_db::SqliteDB;
use ethers::signers::Signer;
use log::{info, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Root};
use log4rs::filter::threshold::ThresholdFilter;
use log4rs::Config as LogConfig;
use std::fs::{self, read_to_string};
use std::path::PathBuf;
use structopt::StructOpt;
use threshold_bls::curve::bn254::PairingCurve as BN254;
use threshold_bls::schemes::bn254::G2Scheme;
use threshold_bls::serialize::point_to_hex;
use threshold_bls::sig::Scheme;

#[derive(StructOpt, Debug)]
#[structopt(name = "Arpa Node")]
pub struct Opt {
    /// Mode to run.
    /// 1) new-run: First run on Randcast client. Loading data from config.yml settings.
    /// 2) re-run: Continue to run Randcast client from some kind of breakdown. Config in existing database data.sqlite will be used.
    #[structopt(short = "m", long, possible_values = &["new-run", "re-run"])]
    mode: String,

    /// Set the config path
    #[structopt(
        short = "c",
        long,
        parse(from_os_str),
        default_value = "conf/config.yml"
    )]
    config_path: PathBuf,
}

fn load_config(opt: Opt) -> String {
    println!("{:#?}", opt);

    let config_str = &read_to_string(opt.config_path).unwrap_or_else(|e| {
        panic!(
            "Error loading configuration file: {:?}, please check the configuration!",
            e
        )
    });

    let config: Config =
        serde_yaml::from_str(config_str).expect("Error loading configuration file");

    config.initialize();

    opt.mode
}

fn init_log(node_id: &str, context_logging: bool) {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(
            JsonEncoder::default().context_logging(context_logging),
        ))
        .build();

    let file = FileAppender::builder()
        .encoder(Box::new(
            JsonEncoder::default().context_logging(context_logging),
        ))
        .build(format!("log/{}/node.log", node_id))
        .unwrap();

    let err_file = FileAppender::builder()
        .encoder(Box::new(
            JsonEncoder::default().context_logging(context_logging),
        ))
        .build(format!("log/{}/node_err.log", node_id))
        .unwrap();

    let log_config = LogConfig::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LevelFilter::Error)))
                .build("err_file", Box::new(err_file)),
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

    let mode = load_config(opt);

    init_log(
        CONFIG.get().unwrap().node_id.as_ref().unwrap(),
        CONFIG.get().unwrap().context_logging,
    );

    let config = CONFIG.get().unwrap();

    info!("{:?}", config);

    let data_path = PathBuf::from(config.data_path.clone().unwrap());

    match mode.as_str() {
        "new-run" => {
            let wallet = build_wallet_from_config(&config.account)?;

            let id_address = wallet.address();

            if data_path.exists() {
                fs::rename(
                    data_path.clone(),
                    data_path.parent().unwrap().join(format!(
                        "bak_{}_{}",
                        format_now_date(),
                        data_path.file_name().unwrap().to_str().unwrap(),
                    )),
                )?;
                info!("Existing data file found. Renamed to the directory of data_path.",);
            }

            let db = SqliteDB::build(
                data_path.as_os_str().to_str().unwrap(),
                &wallet.signer().to_bytes(),
            )
            .await?;

            let rng = &mut rand::thread_rng();

            let (dkg_private_key, dkg_public_key) = G2Scheme::keypair(rng);

            info!("dkg public_key: {}", point_to_hex(&dkg_public_key));

            let mut node_cache = db.get_node_info_client();

            node_cache
                .save_node_info(
                    id_address,
                    config.node_committer_rpc_endpoint.clone(),
                    dkg_private_key,
                    dkg_public_key,
                )
                .await?;

            let group_cache = db.get_group_info_client();

            let randomness_tasks_cache = db.get_bls_tasks_client::<RandomnessTask>();

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
            );

            let main_chain = GeneralMainChain::<
                NodeInfoDBClient<BN254>,
                GroupInfoDBClient<BN254>,
                BLSTasksDBClient<RandomnessTask, BN254>,
                GeneralChainIdentity,
                BN254,
            >::new(
                "main chain".to_string(),
                main_chain_identity.clone(),
                node_cache,
                group_cache,
                randomness_tasks_cache,
            );

            let context = GeneralContext::new(main_chain);

            let handle = context.deploy().await?;

            // TODO register node to randcast network, this should be moved to node_cmd_client(triggering manully to avoid accidental operation) in prod
            let client =
                ControllerClientBuilder::<BN254>::build_controller_client(&main_chain_identity);

            client
                .node_register(bincode::serialize(&dkg_public_key).unwrap())
                .await?;

            handle.wait_task().await;
        }
        "re-run" => {
            let wallet = build_wallet_from_config(&config.account)?;

            let id_address = wallet.address();

            let db = SqliteDB::build(
                data_path.as_os_str().to_str().unwrap(),
                &wallet.signer().to_bytes(),
            )
            .await?;

            let mut node_cache = db.get_node_info_client();

            node_cache.refresh_current_node_info().await.expect(
                "It seems there is no existing node record. Please execute in new-run mode.",
            );

            assert_eq!(node_cache.get_id_address()?, id_address,"Node identity is different from the database, please check or execute in new-run mode.");

            node_cache.get_node_rpc_endpoint()?;
            node_cache.get_dkg_public_key()?;

            let mut group_cache = db.get_group_info_client();

            group_cache.refresh_current_group_info().await.expect(
                "It seems there is no existing group record. Please execute in new-run mode.",
            );

            let randomness_tasks_cache = db.get_bls_tasks_client::<RandomnessTask>();

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
            );

            let main_chain = GeneralMainChain::<
                NodeInfoDBClient<BN254>,
                GroupInfoDBClient<BN254>,
                BLSTasksDBClient<RandomnessTask, BN254>,
                GeneralChainIdentity,
                BN254,
            >::new(
                "main chain".to_string(),
                main_chain_identity,
                node_cache,
                group_cache,
                randomness_tasks_cache,
            );

            let context = GeneralContext::new(main_chain);

            let handle = context.deploy().await?;

            handle.wait_task().await;
        }
        _ => panic!("unimplemented mode"),
    }

    Ok(())
}
