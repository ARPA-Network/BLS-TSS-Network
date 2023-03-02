use std::fs;

use ethers_contract_abigen::MultiAbigen;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto");
    println!("cargo:rerun-if-changed=abi");

    let mut prost_build = prost_build::Config::new();

    prost_build.btree_map(["members"]);

    fs::create_dir_all("./src/rpc_stub")?;

    let protos = &[
        "proto/adapter.proto",
        "proto/controller.proto",
        "proto/coordinator.proto",
    ];

    tonic_build::configure()
        .out_dir("./src/rpc_stub")
        .compile_with_config(prost_build, protos, &["proto"])?;

    MultiAbigen::from_json_files("./abi")
        .unwrap()
        .build()?
        .write_to_module("./src/contract_stub", false)?;

    Ok(())
}
