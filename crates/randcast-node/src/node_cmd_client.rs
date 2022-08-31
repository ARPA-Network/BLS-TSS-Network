use randcast_node::node::contract_client::adapter::{AdapterTransactions, AdapterViews};
use randcast_node::node::contract_client::controller::ControllerViews;
use randcast_node::node::contract_client::types::Group as ContractGroup;
use randcast_node::node::contract_client::{
    rpc_mock::adapter::MockAdapterClient, rpc_mock::controller::MockControllerClient,
};
use std::env;

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

    if contract_type == "1" {
        // Controller
        let client = MockControllerClient::new(contract_rpc_endpoint, id_address.clone());

        if instruction == "set_initial_group" {
            let group = client.get_group(1).await?;

            let group: ContractGroup = group.into();

            let group_as_bytes = bincode::serialize(&group).unwrap();

            let adapter_rpc_endpoint = match args.next() {
                Some(arg) => arg,
                None => panic!("Didn't get a seed string"),
            };

            let client = MockAdapterClient::new(adapter_rpc_endpoint, id_address);

            if let Err(e) = client.set_initial_group(group_as_bytes).await {
                println!("{:?}", e);
            }

            println!("set_initial_group successfully");
        } else if instruction == "get_group" {
            let group_index = match args.next() {
                Some(arg) => arg,
                None => panic!("Didn't get a seed string"),
            };

            let group = client
                .get_group(group_index.parse::<usize>().unwrap())
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

            let group = client
                .get_group(group_index.parse::<usize>().unwrap())
                .await?;

            println!("{:?}", group);
        }
    }

    Ok(())
}
