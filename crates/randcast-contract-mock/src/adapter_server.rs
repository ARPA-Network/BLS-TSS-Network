use std::{env, sync::Arc};

use parking_lot::RwLock;
use randcast_contract_mock::{
    contract::{adapter::Adapter, controller::Controller},
    server::adapter_server::start_adapter_server,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args();

    args.next();

    let adapter_rpc_endpoint = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get an adapter rpc endpoint string"),
    };

    let initial_entropy = 0x2222_2222_2222_2222;

    println!(
        "adapter is deploying... initial entropy: {}",
        initial_entropy
    );

    let adapter = Adapter::new(initial_entropy, adapter_rpc_endpoint.clone());

    let adapter = Controller::new(adapter);

    let adapter = Arc::new(RwLock::new(adapter));

    start_adapter_server(adapter_rpc_endpoint, adapter).await?;

    Ok(())
}
