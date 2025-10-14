// Fetch contract from github releases

use engine_standalone_storage::Storage;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::storage_ext;

#[derive(Debug, Error)]
enum FetchError {
    #[error("Network error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("HTTP status error: {0}")]
    Status(reqwest::StatusCode),
}

pub struct ContractKey {
    pub version: String,
    pub height: u64,
    pub pos: u16,
}

pub async fn all(storage: &RwLock<Storage>, link: &str) {
    // TODO(vlad): fetch all known versions
    let _ = (storage, link);
}

/// the source must be TLS protected
pub async fn fetch_and_store_contract(
    storage: &RwLock<Storage>,
    link: &str,
    version: &str,
    height: u64,
    pos: u16,
) {
    match network_fetch(version, &link).await {
        Ok(code) => {
            // should be fine because of TLS
            tracing::info!(version = version, "Fetched contract");
            let storage = storage.read().await;
            if let Err(err) = storage_ext::store_contract(&storage, height, pos, &code) {
                tracing::error!(
                    version = &version,
                    err = format!("{err:?}"),
                    "Failed to store contract in storage"
                );
            } else {
                tracing::info!(version = version, "Stored contract in storage");
            }
        }
        Err(err) => {
            tracing::error!(
                version = version,
                err = format!("{err:?}"),
                "Failed to fetch contract from github"
            );
        }
    }
}

// https://github.com/aurora-is-near/aurora-engine/releases/download/%version%/aurora-compat.wasm
async fn network_fetch(version: &str, link: &str) -> Result<Vec<u8>, FetchError> {
    let url = link.replace("%version%", version);
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        return Err(FetchError::Status(response.status()));
    }
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}
