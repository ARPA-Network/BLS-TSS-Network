use arpa_node::node::context::chain::types::GeneralMainChain;
use arpa_node::node::context::types::{build_wallet_from_config, Config, GeneralContext};
use arpa_node::node::context::{Context, TaskWaiter};
use arpa_node_contract_client::controller::{ControllerClientBuilder, ControllerTransactions};
use arpa_node_core::format_now_date;
use arpa_node_core::GeneralChainIdentity;
use arpa_node_core::MockChainIdentity;
use arpa_node_core::RandomnessTask;
use arpa_node_dal::cache::{InMemoryBLSTasksQueue, InMemoryGroupInfoCache, InMemoryNodeInfoCache};
use arpa_node_dal::{NodeInfoFetcher, NodeInfoUpdater};
use arpa_node_sqlite_db::BLSTasksDBClient;
use arpa_node_sqlite_db::GroupInfoDBClient;
use arpa_node_sqlite_db::NodeInfoDBClient;
use arpa_node_sqlite_db::SqliteDB;
use ethers::signers::Signer;
use log::{info, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Root};
use log4rs::encode::json::JsonEncoder;
use log4rs::filter::threshold::ThresholdFilter;
use log4rs::Config as LogConfig;
use std::fs::{self, read_to_string};
use std::path::PathBuf;
use structopt::StructOpt;
use threshold_bls::curve::bn254::PairingCurve as BN254;
use threshold_bls::schemes::bn254::G2Scheme;
use threshold_bls::sig::Scheme;

#[derive(StructOpt, Debug)]
#[structopt(name = "Arpa Node")]
pub struct Opt {
    /// Mode to run.
    /// 1) new-run: First run on Randcast client. Loading data from config.yml settings.
    /// 2) re-run: Continue to run Randcast client from some kind of breakdown. Config in existing database data.sqlite will be used.
    /// 3) demo: Run a demo with data in memory only.
    #[structopt(short = "m", long, possible_values = &["new-run", "re-run", "demo"])]
    mode: String,

    /// Set the index of the config file when running the node demo
    #[structopt(short = "i", long, possible_values = &["1", "2", "3", "4", "5", "6"],required_if("mode", "demo"))]
    demo_config_index: Option<u32>,

    /// Set the config path
    #[structopt(
        short = "c",
        long,
        parse(from_os_str),
        default_value = "conf/config.yml"
    )]
    config_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    println!("{:#?}", opt);

    let node_id = if let Some(id) = opt.demo_config_index {
        id.to_string()
    } else {
        String::from("running")
    };

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(JsonEncoder::new()))
        .build();

    let file = FileAppender::builder()
        .encoder(Box::new(JsonEncoder::new())) //PatternEncoder::new("{d} {l} - {m}{n}"))
        .build(format!("log/{}/node.log", node_id))
        .unwrap();

    let err_file = FileAppender::builder()
        .encoder(Box::new(JsonEncoder::new()))
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
                .build(LevelFilter::Debug),
        )
        .unwrap();

    log4rs::init_config(log_config).unwrap();

    let config_path_str = opt
        .config_path
        .clone()
        .into_os_string()
        .into_string()
        .unwrap();

    let config_str = &read_to_string(opt.config_path).unwrap_or_else(|_| {
        panic!(
            "Error loading configuration file {}, please check the configuration!",
            config_path_str
        )
    });

    let yaml_str = match node_id.as_str() {
        "1" => include_str!("../conf/config_test_1.yml"),
        "2" => include_str!("../conf/config_test_2.yml"),
        "3" => include_str!("../conf/config_test_3.yml"),
        "4" => include_str!("../conf/config_test_4.yml"),
        "5" => include_str!("../conf/config_test_5.yml"),
        "6" => include_str!("../conf/config_test_6.yml"),
        _ => config_str,
    };

    let mut config: Config =
        serde_yaml::from_str(yaml_str).expect("Error loading configuration file");
    if config.data_path.is_none() {
        config.data_path = Some(String::from("data.sqlite"));
    }

    info!(target: "config", "{:?}", config);

    let data_path = PathBuf::from(config.data_path.clone().unwrap());

    match opt.mode.as_str() {
        "new-run" => {
            let wallet = build_wallet_from_config(&config.account)?;

            let id_address = wallet.address();

            if data_path.exists() {
                fs::rename(
                    data_path.clone(),
                    data_path
                        .parent()
                        .unwrap()
                        .join(format!("bak_{}.sqlite", format_now_date())),
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

            info!("dkg private_key: {}", dkg_private_key);
            info!("dkg public_key: {}", dkg_public_key);
            info!("-------------------------------------------------------");

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
                0,
                0,
                wallet,
                config.provider_endpoint.clone(),
                config
                    .controller_address
                    .parse()
                    .expect("bad format of controller_address"),
            );

            let main_chain = GeneralMainChain::<
                NodeInfoDBClient<BN254>,
                GroupInfoDBClient<BN254>,
                BLSTasksDBClient<RandomnessTask, BN254>,
                GeneralChainIdentity,
                BN254,
            >::new(
                0,
                "main chain".to_string(),
                main_chain_identity.clone(),
                node_cache,
                group_cache,
                randomness_tasks_cache,
            );

            let context = GeneralContext::new(config, main_chain);

            let handle = context.deploy().await?;

            // TODO register node to randcast network, this should be moved to node_cmd_client(triggering manully to avoid accidental operation) in prod
            let client = main_chain_identity.build_controller_client();

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
                0,
                0,
                wallet,
                config.provider_endpoint.clone(),
                config
                    .controller_address
                    .parse()
                    .expect("bad format of controller_address"),
            );

            let main_chain = GeneralMainChain::<
                NodeInfoDBClient<BN254>,
                GroupInfoDBClient<BN254>,
                BLSTasksDBClient<RandomnessTask, BN254>,
                GeneralChainIdentity,
                BN254,
            >::new(
                0,
                "main chain".to_string(),
                main_chain_identity,
                node_cache,
                group_cache,
                randomness_tasks_cache,
            );

            let context = GeneralContext::new(config, main_chain);

            let handle = context.deploy().await?;

            handle.wait_task().await;
        }
        "demo" => {
            let id_address = format!("0x000000000000000000000000000000000000000{}", node_id)
                .parse()
                .unwrap();

            let rng = &mut rand::thread_rng();

            let (dkg_private_key, dkg_public_key) = G2Scheme::keypair(rng);

            info!("dkg private_key: {}", dkg_private_key);
            info!("dkg public_key: {}", dkg_public_key);
            info!("-------------------------------------------------------");

            let mut node_cache = InMemoryNodeInfoCache::new(id_address);

            node_cache
                .set_node_rpc_endpoint(config.node_committer_rpc_endpoint.clone())
                .await
                .unwrap();

            node_cache
                .set_dkg_key_pair(dkg_private_key, dkg_public_key)
                .await
                .unwrap();

            let group_cache = InMemoryGroupInfoCache::new();

            let main_chain_identity =
                MockChainIdentity::new(0, 0, id_address, config.provider_endpoint.clone());

            let randomness_tasks_cache = InMemoryBLSTasksQueue::<RandomnessTask>::new();

            let main_chain = GeneralMainChain::<
                InMemoryNodeInfoCache<BN254>,
                InMemoryGroupInfoCache<BN254>,
                InMemoryBLSTasksQueue<RandomnessTask>,
                MockChainIdentity,
                BN254,
            >::new(
                0,
                "main chain".to_string(),
                main_chain_identity.clone(),
                node_cache,
                group_cache,
                randomness_tasks_cache,
            );

            let context = GeneralContext::new(config, main_chain);

            let handle = context.deploy().await?;

            println!("finished!");

            // register node to randcast network
            let client = main_chain_identity.build_controller_client();

            client
                .node_register(bincode::serialize(&dkg_public_key).unwrap())
                .await?;

            handle.wait_task().await;
        }
        _ => panic!("unimplemented mode"),
    }

    Ok(())
}
