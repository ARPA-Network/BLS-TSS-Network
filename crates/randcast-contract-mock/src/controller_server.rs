use std::{env, sync::Arc};

use parking_lot::RwLock;
use randcast_contract_mock::{
    contract::{adapter::Adapter, controller::Controller},
    server::controller_server::start_controller_server,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args();

    args.next();

    let controller_rpc_endpoint = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get a controller rpc endpoint string"),
    };

    let initial_entropy = 0x1111_1111_1111_1111;

    println!(
        "controller is deploying... initial entropy: {}",
        initial_entropy
    );

    let adapter = Adapter::new(initial_entropy, controller_rpc_endpoint.clone());

    let controller = Controller::new(adapter);

    let controller = Arc::new(RwLock::new(controller));

    start_controller_server(controller_rpc_endpoint, controller).await?;

    Ok(())
}
