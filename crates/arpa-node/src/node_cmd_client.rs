use arpa_node_contract_client::{adapter::AdapterViews, rpc_mock::adapter::MockAdapterClient};
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

    let contract_rpc_endpoint = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get an adapter rpc endpoint string"),
    };

    let contract_type = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get a contract type string"),
    };

    let instruction = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get an instruction string"),
    };

    let id_address: Address = id_address.parse().unwrap();

    if contract_type == "1" {
        // Controller
        let client = MockAdapterClient::new(contract_rpc_endpoint, id_address);

        if instruction == "get_group" {
            let group_index = match args.next() {
                Some(arg) => arg,
                None => panic!("Didn't get a seed string"),
            };

            let group =
                AdapterViews::<BN254>::get_group(&client, group_index.parse::<usize>().unwrap())
                    .await?;

            println!("{:?}", group);
        }
    } else if contract_type == "2" {
        // Adapter
        let client = MockAdapterClient::new(contract_rpc_endpoint, id_address);

        if instruction == "get_group" {
            let group_index = match args.next() {
                Some(arg) => arg,
                None => panic!("Didn't get a seed string"),
            };

            let group =
                AdapterViews::<BN254>::get_group(&client, group_index.parse::<usize>().unwrap())
                    .await?;

            println!("{:?}", group);
        }
    }

    Ok(())
}
