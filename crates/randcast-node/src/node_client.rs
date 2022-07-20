use randcast_node::node::context::chain::InMemoryAdapterChain;
use randcast_node::node::context::context::{Context, InMemoryContext, TaskWaiter};
use randcast_node::node::context::types::Config;
use randcast_node::node::contract_client::controller_client::{
    ControllerTransactions, MockControllerClient,
};
use randcast_node::node::dao::types::ChainIdentity;
use std::env;
use threshold_bls::schemes::bls12_381::G1Scheme;
use threshold_bls::sig::Scheme;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args();

    args.next();

    let node_index = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get an id_address string"),
    };

    let yaml_str = match node_index.parse::<usize>().unwrap() {
        1 => include_str!("../config_test_1.yml"),
        2 => include_str!("../config_test_2.yml"),
        3 => include_str!("../config_test_3.yml"),
        4 => include_str!("../config_test_4.yml"),
        5 => include_str!("../config_test_5.yml"),
        6 => include_str!("../config_test_6.yml"),
        _ => panic!("This test config is not exist!"),
    };

    let config: Config = serde_yaml::from_str(yaml_str).expect("config.yml read failed!");

    println!("{:?}", config);

    let rng = &mut rand::thread_rng();

    let (private_key, public_key) = G1Scheme::keypair(rng);

    println!("dkg private_key: {}", private_key);
    println!("dkg public_key: {}", public_key);
    println!("-------------------------------------------------------");

    let main_chain_identity = ChainIdentity::new(
        0,
        vec![],
        config.id_address.clone(),
        config.controller_endpoint.clone(),
    );

    let mut context = InMemoryContext::new(
        main_chain_identity,
        config.node_rpc_endpoint,
        private_key,
        public_key,
    );

    for adapter in config.adapters {
        let chain_identity =
            ChainIdentity::new(adapter.id, vec![], adapter.id_address, adapter.endpoint);

        let chain = InMemoryAdapterChain::new(adapter.id, adapter.name, chain_identity);

        context.add_adapter_chain(chain)?;
    }

    let handle = context.deploy();

    // register node to randcast network
    let mut client =
        MockControllerClient::new(config.controller_endpoint, config.id_address).await?;

    client
        .node_register(bincode::serialize(&public_key).unwrap())
        .await?;

    handle.wait_task().await;

    Ok(())
}
