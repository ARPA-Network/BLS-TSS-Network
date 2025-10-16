use std::fs;

const RPC_STUB_DIR: &str = "./src/rpc_stub";
const PROTO_DIR: &str = "proto";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto");
    println!("cargo:rerun-if-changed=src/mock_contracts");

    let mut prost_build = tonic_prost_build::Config::new();
    prost_build.btree_map(["members"]);
    fs::create_dir_all(RPC_STUB_DIR)?;
    let protos = &["proto/committer.proto", "proto/management.proto"];

    tonic_prost_build::configure()
        .out_dir(RPC_STUB_DIR)
        .compile_with_config(prost_build, protos, &[PROTO_DIR])?;

    Ok(())
}
