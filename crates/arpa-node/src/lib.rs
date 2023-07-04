#![allow(incomplete_features)]
#![feature(async_fn_in_trait)]

use arpa_node_core::Config;
use std::{fs::read_to_string, path::PathBuf};

pub mod node;
pub mod rpc_stub;

pub fn load_config(config_path: PathBuf) -> Config {
    let config_str = &read_to_string(config_path).unwrap_or_else(|e| {
        panic!(
            "Error loading configuration file: {:?}, please check the configuration!",
            e
        )
    });

    let config: Config =
        serde_yaml::from_str(config_str).expect("Error loading configuration file");

    config.initialize()
}
