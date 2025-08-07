use aurora_engine_types::account_id::AccountId;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::path::PathBuf;

use crate::store::OutputStoreConfig;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub refiner: Refiner,
    pub output_storage: OutputStoreConfig,
    pub input_mode: InputMode,
    pub socket_server: Option<SocketServer>,
    pub contract_wasm_code_path: PathBuf,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Refiner {
    pub chain_id: u64,
    pub engine_path: PathBuf,
    #[serde(deserialize_with = "deserialize_account_id_with_descriptive_error_message")]
    pub engine_account_id: AccountId,
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

fn deserialize_account_id_with_descriptive_error_message<'de, D>(
    deserializer: D,
) -> Result<AccountId, D::Error>
where
    D: Deserializer<'de>,
{
    AccountId::deserialize(deserializer)
        .map_err(|v| Error::custom(format!("invalid Near account ID, error code: {v}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_near_account_id_does_not_pass_deserialization() {
        let error = serde_json::from_str::<Refiner>(
            r#"{
            "chain_id": 0,
            "engine_path": "test",
            "engine_account_id": "invalid id because it contains spaces"
        }"#,
        )
        .unwrap_err();

        let actual_error = format!("{error}");
        let expected_error =
            "invalid Near account ID, error code: ERR_ACCOUNT_ID_TO_INVALID at line 5 column 9";

        assert_eq!(actual_error, expected_error);
    }

    #[test]
    fn test_near_account_id_passes_deserialization() {
        let refiner = serde_json::from_str::<Refiner>(
            r#"{
            "chain_id": 0,
            "engine_path": "test",
            "engine_account_id": "some_near_id"
        }"#,
        )
        .unwrap();

        let actual_near_account_id = refiner.engine_account_id.to_string();
        let expected_near_account_id = "some_near_id";

        assert_eq!(actual_near_account_id, expected_near_account_id);
    }
}
