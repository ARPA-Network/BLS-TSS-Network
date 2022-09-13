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
        .out_dir("stub")
        .compile_with_config(prost_build, protos, &["proto"])?;

    Ok(())
}
