use ethers_contract_abigen::MultiAbigen;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protos = &[
        "proto/adapter.proto",
        "proto/controller.proto",
        "proto/coordinator.proto",
        "proto/committer.proto",
    ];

    for proto in protos {
        println!("cargo:rerun-if-changed={}", proto);
    }

    let mut prost_build = prost_build::Config::new();

    prost_build.btree_map(&["members"]);

    tonic_build::configure()
        .out_dir("rpc_stub")
        .compile_with_config(prost_build, protos, &["proto"])?;

    MultiAbigen::from_json_files("./abi")
        .unwrap()
        .build()?
        .write_to_module("contract_stub", false)?;

    Ok(())
}
