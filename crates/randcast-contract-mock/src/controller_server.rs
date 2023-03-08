use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Root};
use log4rs::encode::json::JsonEncoder;
use log4rs::Config as LogConfig;
use parking_lot::RwLock;
use randcast_contract_mock::{
    contract::{adapter::Adapter, controller::Controller},
    server::controller_server::start_controller_server,
};
use std::{env, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(JsonEncoder::new()))
        .build();

    let log_config = LogConfig::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .unwrap();

    log4rs::init_config(log_config).unwrap();

    let mut args = env::args();

    args.next();

    let controller_rpc_endpoint = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get a controller rpc endpoint string"),
    };

    let initial_entropy = 0x1111_1111_1111_1111_u64.into();

    println!(
        "controller is deploying... initial entropy: {}",
        initial_entropy
    );

    let adapter = Adapter::new(initial_entropy);

    let controller = Controller::new(adapter);

    let controller = Arc::new(RwLock::new(controller));

    start_controller_server(controller_rpc_endpoint, controller).await?;

    Ok(())
}
