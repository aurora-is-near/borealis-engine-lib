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
}

#[derive(Deserialize, Clone, Debug)]
pub struct Refiner {
    pub chain_id: u64,
    pub engine_path: PathBuf,
    pub engine_account_id: ValidatedAccountId,
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

#[derive(Clone, Debug)]
pub struct ValidatedAccountId(AccountId);

impl From<ValidatedAccountId> for AccountId {
    fn from(value: ValidatedAccountId) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for ValidatedAccountId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let account_id = AccountId::deserialize(deserializer)
            .map_err(|v| Error::custom(format!("invalid Near account ID, error code: {v}")))?;

        Ok(ValidatedAccountId(account_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_near_account_id_does_not_pass_deserialization() {
        let error =
            serde_json::from_str::<ValidatedAccountId>("\"invalid id because it contains spaces\"")
                .unwrap_err();

        let actual_error = format!("{error}");
        let expected_error = "invalid Near account ID, error code: ERR_ACCOUNT_ID_TO_INVALID";

        assert_eq!(actual_error, expected_error);
    }

    #[test]
    fn test_near_account_id_passes_deserialization() {
        let account_id = serde_json::from_str::<ValidatedAccountId>("\"some_near_id\"").unwrap();

        let actual_near_account_id = account_id.0.to_string();
        let expected_near_account_id = "some_near_id";

        assert_eq!(actual_near_account_id, expected_near_account_id);
    }
}
