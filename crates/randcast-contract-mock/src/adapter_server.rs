use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Root};
use log4rs::encode::json::JsonEncoder;
use log4rs::Config as LogConfig;
use parking_lot::RwLock;
use randcast_contract_mock::{
    contract::{adapter::Adapter, controller::Controller},
    server::adapter_server::start_adapter_server,
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

    let adapter_rpc_endpoint = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get an adapter rpc endpoint string"),
    };

    let initial_entropy = 0x2222_2222_2222_2222_u64.into();

    println!(
        "adapter is deploying... initial entropy: {}",
        initial_entropy
    );

    let adapter = Adapter::new(initial_entropy);

    let adapter = Controller::new(adapter);

    let adapter = Arc::new(RwLock::new(adapter));

    start_adapter_server(adapter_rpc_endpoint, adapter).await?;

    Ok(())
}
