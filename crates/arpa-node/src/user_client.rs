use arpa_node_contract_client::{
    adapter::{AdapterTransactions, AdapterViews},
    rpc_mock::adapter::MockAdapterClient,
};
use ethers::types::Address;
use std::env;
use threshold_bls::curve::bn254::PairingCurve as BN254;

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

    let id_address: Address = id_address.parse().unwrap();

    let client = MockAdapterClient::new(adapter_rpc_endpoint, id_address);

    if instruction == "request" {
        let seed = match args.next() {
            Some(arg) => arg,
            None => panic!("Didn't get a seed string"),
        };
        client.request_randomness(&seed).await?;

        println!("request randomness successfully");
    } else if instruction == "last_output" {
        let res = AdapterViews::<BN254>::get_last_output(&client).await?;

        println!("last_randomness_output: {}", res);
    }

    Ok(())
}
