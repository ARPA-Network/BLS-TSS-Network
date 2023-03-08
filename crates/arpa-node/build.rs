use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto");

    let mut prost_build = prost_build::Config::new();

    prost_build.btree_map(["members"]);

    fs::create_dir_all("./src/rpc_stub")?;

    let protos = &["proto/committer.proto", "proto/management.proto"];

    tonic_build::configure()
        .out_dir("./src/rpc_stub")
        .compile_with_config(prost_build, protos, &["proto"])?;

    Ok(())
}
