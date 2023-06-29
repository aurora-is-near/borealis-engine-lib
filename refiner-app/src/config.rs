use serde::Deserialize;

use crate::store::OutputStoreConfig;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub refiner: Refiner,
    pub output_storage: OutputStoreConfig,
    pub input_mode: InputMode,
    pub socket_server: Option<SocketServer>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Refiner {
    pub chain_id: u64,
    pub engine_path: String,
    pub engine_account_id: String,
    #[serde(default)]
    pub tx_tracker_path: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub enum InputMode {
    DataLake(DataLakeConfig),
    Nearcore(NearcoreConfig),
}

#[derive(Deserialize, Clone, Debug)]
pub struct DataLakeConfig {
    pub network: Network,
}

#[derive(Deserialize, Clone, Debug)]
pub struct NearcoreConfig {
    pub path: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SocketServer {
    pub path: String,
}

#[derive(Deserialize, Clone, Debug)]
pub enum Network {
    Mainnet,
    Testnet,
}
