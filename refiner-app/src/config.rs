use serde::Deserialize;
use std::path::PathBuf;

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
    pub engine_path: PathBuf,
    pub engine_account_id: String,
    #[serde(default)]
    pub tx_tracker_path: Option<PathBuf>,
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
    pub path: PathBuf,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SocketServer {
    pub path: PathBuf,
}

#[derive(Deserialize, Clone, Debug)]
pub enum Network {
    Mainnet,
    Testnet,
}
