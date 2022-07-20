use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub id_address: String,
    pub node_rpc_endpoint: String,
    pub controller_endpoint: String,
    pub adapters: Vec<Adapter>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Adapter {
    pub id: usize,
    pub id_address: String,
    pub name: String,
    pub endpoint: String,
}
