use std::path::PathBuf;

use log::{info, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::filter::threshold::ThresholdFilter;
use log4rs::Config as LogConfig;
use randcast_node::node::context::chain::types::GeneralMainChain;
use randcast_node::node::context::types::{Config, GeneralContext};
use randcast_node::node::context::{Context, TaskWaiter};
use randcast_node::node::contract_client::controller::ControllerTransactions;
use randcast_node::node::contract_client::rpc_mock::controller::MockControllerClient;
use randcast_node::node::dal::cache::{
    InMemoryBLSTasksQueue, InMemoryGroupInfoCache, InMemoryNodeInfoCache,
};
use randcast_node::node::dal::sqlite::{
    init_tables, BLSTasksDBClient, GroupInfoDBClient, NodeInfoDBClient,
};
use randcast_node::node::dal::types::{ChainIdentity, RandomnessTask};
use randcast_node::node::dal::utils::format_now_date;
use randcast_node::node::dal::{NodeInfoFetcher, NodeInfoUpdater};
use std::fs::{self, read_to_string};
use structopt::StructOpt;
use threshold_bls::schemes::bls12_381::G1Scheme;
use threshold_bls::sig::Scheme;

#[derive(StructOpt, Debug)]
#[structopt(name = "Randcast Node")]
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
    #[structopt(short = "c", long, parse(from_os_str), default_value = "config.yml")]
    config_path: PathBuf,

    /// Data file for persistence
    #[structopt(short, long, parse(from_os_str), default_value = "data.sqlite")]
    data_path: PathBuf,
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

    let stdout = ConsoleAppender::builder().build();

    let file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} {l} - {m}{n}")))
        .build(format!("log/{}/node.log", node_id))
        .unwrap();

    let err_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} {l} {t} - {m}{n}")))
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

    // log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();

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
        "1" => include_str!("../config_test_1.yml"),
        "2" => include_str!("../config_test_2.yml"),
        "3" => include_str!("../config_test_3.yml"),
        "4" => include_str!("../config_test_4.yml"),
        "5" => include_str!("../config_test_5.yml"),
        "6" => include_str!("../config_test_6.yml"),
        _ => config_str,
    };

    let config: Config = serde_yaml::from_str(yaml_str).expect("Error loading configuration file");

    info!("{:?}", config);

    match opt.mode.as_str() {
        "new-run" => {
            if opt.data_path.exists() {
                fs::rename(
                    opt.data_path.clone(),
                    opt.data_path
                        .parent()
                        .unwrap()
                        .join(format!("bak_{}.sqlite", format_now_date())),
                )?;
                info!("Existing data file found. Renamed to the directory of data_path.",);
            }
            init_tables(opt.data_path.clone())?;

            let rng = &mut rand::thread_rng();

            let (dkg_private_key, dkg_public_key) = G1Scheme::keypair(rng);

            info!("dkg private_key: {}", dkg_private_key);
            info!("dkg public_key: {}", dkg_public_key);
            info!("-------------------------------------------------------");

            let mut node_cache = NodeInfoDBClient::new(opt.data_path.clone());

            node_cache
                .save_node_info(config.id_address.clone(), config.node_rpc_endpoint.clone())?;

            node_cache
                .set_dkg_key_pair(dkg_private_key, dkg_public_key)
                .unwrap();

            let group_cache = GroupInfoDBClient::new(opt.data_path.clone());

            let randomness_tasks_cache = BLSTasksDBClient::<RandomnessTask>::new(opt.data_path);

            let main_chain_identity = ChainIdentity::new(
                0,
                vec![],
                config.id_address.clone(),
                config.controller_endpoint.clone(),
            );

            let main_chain = GeneralMainChain::<
                NodeInfoDBClient,
                GroupInfoDBClient,
                BLSTasksDBClient<RandomnessTask>,
            >::new(
                0,
                "main chain".to_string(),
                main_chain_identity,
                node_cache,
                group_cache,
                randomness_tasks_cache,
            );

            let context = GeneralContext::new(main_chain);

            let handle = context.deploy();

            // TODO register node to randcast network, this should be moved to node_cmd_client(triggering manully to avoid accidental operation) in prod
            let client = MockControllerClient::new(config.controller_endpoint, config.id_address);

            client
                .node_register(bincode::serialize(&dkg_public_key).unwrap())
                .await?;

            handle.wait_task().await;
        }
        "re-run" => {
            let mut node_cache = NodeInfoDBClient::new(opt.data_path.clone());

            node_cache.refresh_current_node_info().expect(
                "It seems there is no existing node record. Please execute in new-run mode.",
            );

            node_cache.get_node_rpc_endpoint()?;
            node_cache.get_dkg_public_key()?;

            let mut group_cache = GroupInfoDBClient::new(opt.data_path.clone());

            group_cache.refresh_current_group_info().expect(
                "It seems there is no existing group record. Please execute in new-run mode.",
            );

            let randomness_tasks_cache = BLSTasksDBClient::<RandomnessTask>::new(opt.data_path);

            let main_chain_identity = ChainIdentity::new(
                0,
                vec![],
                config.id_address.clone(),
                config.controller_endpoint.clone(),
            );

            let main_chain = GeneralMainChain::<
                NodeInfoDBClient,
                GroupInfoDBClient,
                BLSTasksDBClient<RandomnessTask>,
            >::new(
                0,
                "main chain".to_string(),
                main_chain_identity,
                node_cache,
                group_cache,
                randomness_tasks_cache,
            );

            let context = GeneralContext::new(main_chain);

            let handle = context.deploy();

            handle.wait_task().await;
        }
        "demo" => {
            let rng = &mut rand::thread_rng();

            let (dkg_private_key, dkg_public_key) = G1Scheme::keypair(rng);

            info!("dkg private_key: {}", dkg_private_key);
            info!("dkg public_key: {}", dkg_public_key);
            info!("-------------------------------------------------------");

            let mut node_cache = InMemoryNodeInfoCache::new(config.id_address.clone());

            node_cache
                .set_node_rpc_endpoint(config.node_rpc_endpoint.clone())
                .unwrap();

            node_cache
                .set_dkg_key_pair(dkg_private_key, dkg_public_key)
                .unwrap();

            let group_cache = InMemoryGroupInfoCache::new();

            let main_chain_identity = ChainIdentity::new(
                0,
                vec![],
                config.id_address.clone(),
                config.controller_endpoint.clone(),
            );

            let randomness_tasks_cache = InMemoryBLSTasksQueue::<RandomnessTask>::new();

            let main_chain = GeneralMainChain::<
                InMemoryNodeInfoCache,
                InMemoryGroupInfoCache,
                InMemoryBLSTasksQueue<RandomnessTask>,
            >::new(
                0,
                "main chain".to_string(),
                main_chain_identity,
                node_cache,
                group_cache,
                randomness_tasks_cache,
            );

            let context = GeneralContext::new(main_chain);

            // suspend handling adapters
            // for adapter in config.adapters {
            //     let chain_identity =
            //         ChainIdentity::new(adapter.id, vec![], adapter.id_address, adapter.endpoint);

            //     let randomness_tasks_cache = InMemoryBLSTasksQueue::<RandomnessTask>::new();

            //     let chain = InMemoryAdapterChain::<
            //         InMemoryNodeInfoCache,
            //         InMemoryGroupInfoCache,
            //         InMemoryBLSTasksQueue<RandomnessTask>,
            //     >::new(
            //         adapter.id,
            //         adapter.name,
            //         chain_identity,
            //         randomness_tasks_cache,
            //     );

            //     context.add_adapter_chain(chain)?;
            // }

            let handle = context.deploy();

            // register node to randcast network
            let client = MockControllerClient::new(config.controller_endpoint, config.id_address);

            client
                .node_register(bincode::serialize(&dkg_public_key).unwrap())
                .await?;

            handle.wait_task().await;
        }
        _ => panic!("unimplemented mode"),
    }

    Ok(())
}
