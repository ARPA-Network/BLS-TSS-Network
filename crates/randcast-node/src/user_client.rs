use std::env;

use randcast_node::node::contract_client::adapter_client::{
    AdapterTransactions, AdapterViews, MockAdapterClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args();

    args.next();

    let id_address = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get an id_address string"),
    };

    let adapter_rpc_endpoint = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get an adapter rpc endpoint string"),
    };

    let instruction = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get an instruction string"),
    };

    let mut client = MockAdapterClient::new(adapter_rpc_endpoint, id_address).await?;

    if instruction == "request" {
        let seed = match args.next() {
            Some(arg) => arg,
            None => panic!("Didn't get a seed string"),
        };
        client.request_randomness(&seed).await?;

        println!("request randomness successfully");
    } else if instruction == "last_output" {
        let res = client.get_last_output().await?;

        println!("last_randomness_output: {}", res);
    }

    Ok(())
}
