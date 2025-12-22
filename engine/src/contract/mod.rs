mod fetch;
mod github;
pub mod version;

#[cfg(test)]
mod tests;

use std::io;

use futures::{Stream, StreamExt};

use aurora_refiner_types::source_config::ContractSource;
use engine_standalone_storage::{Error, Storage, WasmInitError};

/// Fetch all versions of the contract from the given source and store in the storage.
/// The source must be TLS protected.
pub async fn fetch_all(storage: &Storage, source: &ContractSource) -> Result<(), fetch::Error> {
    match source {
        ContractSource::Github { base_url, repo } => {
            let releases = github::fetch_releases(base_url, repo).await?;
            for release in releases {
                if let Some(asset) = release
                    .assets
                    .into_iter()
                    .find(|x| x.name == "aurora-compat.wasm")
                {
                    let code = github::fetch_asset(&asset).await?;
                    tracing::info!(version = release.tag_name, "Fetched contract");
                    if let Err(err) = store_contract_by_version(storage, &release.tag_name, &code) {
                        tracing::error!(
                            version = &release.tag_name,
                            err = format!("{err:?}"),
                            "Failed to store contract in storage"
                        );
                    } else {
                        tracing::info!(version = &release.tag_name, "Stored contract in storage");
                    }
                } else {
                    tracing::warn!(
                        version = release.tag_name,
                        "No aurora-compat.wasm asset found"
                    );
                    continue;
                }
            }
        }
        ContractSource::Mock => {
            let list = [
                ("3.6.4", bundled::CONTRACT_3_6_4),
                ("3.7.0", bundled::CONTRACT_3_7_0),
                ("3.9.0", bundled::CONTRACT_3_9_0),
                ("3.9.1", bundled::CONTRACT_3_9_1),
            ];
            for (version, code) in list {
                if let Err(err) = store_contract_by_version(storage, version, code) {
                    tracing::error!(
                        version = version,
                        err = format!("{err:?}"),
                        "Failed to store contract in storage"
                    );
                } else {
                    tracing::info!(version = version, "Stored contract in storage");
                }
            }
        }
    }

    Ok(())
}

/// Fetch the specific version of the contract from the given source and store in the storage.
/// `height` and `tx_pos` refer to the block height and the transaction position
/// where the contract was deployed and hence all transaction starting from this position
/// must run against it. The source must be TLS protected.
pub async fn fetch_version(
    storage: &Storage,
    link: &ContractSource,
    version: &str,
    height: u64,
    tx_pos: u16,
) -> Option<Vec<u8>> {
    match fetch::run(version, link).await {
        Ok(code) => {
            tracing::info!(version = version, "Fetched contract");
            if let Err(err) = store_contract(storage, height, tx_pos, &code) {
                tracing::error!(
                    version = &version,
                    err = format!("{err:?}"),
                    "Failed to store contract in storage"
                );
            } else {
                tracing::info!(version = version, "Stored contract in storage");
            }
            Some(code)
        }
        Err(err) => {
            tracing::error!(
                version = version,
                err = format!("{err:?}"),
                "Failed to fetch contract from github"
            );
            None
        }
    }
}

/// Receive contract code from the stream and store.
/// The stream item is a tuple `(version, code)`
pub async fn update<S>(mut contract_update: S, storage: &Storage)
where
    S: Stream<Item = (String, Vec<u8>)> + Unpin,
{
    while let Some((version, code)) = contract_update.next().await {
        if let Err(err) = store_contract_by_version(storage, &version, &code) {
            tracing::error!(
                err = format!("{err:?}"),
                new_version = &version,
                "Failed to store updated contract",
            );
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ContractApplyError {
    /// Wasm runner cannot initialize from the code.
    #[error("WASM initialization error: {0}")]
    BadCode(#[from] WasmInitError),
    /// Cannot fetch the contract by a key. The database is broken.
    #[error("database error: {0:?}")]
    Db(Error),
    #[error("I/O error: {0}")]
    /// Cannot fallback to fatch the code from filesystem.
    LoadError(#[from] io::Error),
    /// Must either provide a version string or the contract must be already deployed and
    /// height/pos is known.
    #[error("Contract not found at {height}.{pos} and no version provided")]
    NotFound { height: u64, pos: u16 },
}

/// Apply the contract so the standalone storage will execute this version when call
/// `engine_standalone_storage::sync::*` methods
pub fn apply(
    storage: &mut Storage,
    height: u64,
    pos: u16,
    version: Option<&str>,
) -> Result<(), ContractApplyError> {
    if let Some(version) = version {
        let key = [&CONTRACT_KEY[..], version.as_bytes()].concat();
        return if let Some(data) = storage
            .get_custom_data(&key)
            .map_err(Error::Rocksdb)
            .map_err(ContractApplyError::Db)?
        {
            tracing::debug!(
                height = height,
                version = version,
                "apply contract code stored by version"
            );
            store_contract(storage, height, pos, &data).map_err(ContractApplyError::Db)?;
            // TODO(vlad): consider removing versioned data after storing it in height/pos
            // storage.remove_custom_data(&key)?;
            storage.runner_mut().set_code(data)?;
            Ok(())
        } else {
            if version::ver_cmp(version, "3.6.4").is_lt() {
                tracing::debug!(
                    height = height,
                    version = version,
                    "skip update because the version is bellow 3.6.4"
                );
                Ok(())
            } else {
                tracing::debug!(
                    height = height,
                    version = version,
                    "apply contract code bundled in the library"
                );

                let bytes = bundled::get(&version)
                    .ok_or_else(|| ContractApplyError::NotFound { height, pos })?;
                store_contract(storage, height, pos, bytes).map_err(ContractApplyError::Db)?;
                storage.runner_mut().set_code(bytes.to_vec())?;
                Ok(())
            }
        };
    } else if let Some(data) = storage
        .get_custom_data_at(CONTRACT_KEY, height, pos)
        .map_err(Error::Rocksdb)
        .map_err(ContractApplyError::Db)?
    {
        tracing::debug!(
            height = height,
            "apply contract code stored by block height"
        );
        storage.runner_mut().set_code(data)?;
        return Ok(());
    } else {
        // TODO(vlad): initialize latest available wasm code
        Err(ContractApplyError::NotFound { height, pos })
    }
}

const CONTRACT_KEY: &[u8] = b"\0";

fn store_contract(storage: &Storage, height: u64, pos: u16, value: &[u8]) -> Result<(), Error> {
    storage
        .set_custom_data_at(CONTRACT_KEY, height, pos, &value)
        .map_err(Error::Rocksdb)
}

fn store_contract_by_version(storage: &Storage, version: &str, value: &[u8]) -> Result<(), Error> {
    storage
        .set_custom_data(&[&CONTRACT_KEY[..], version.as_bytes()].concat(), &value)
        .map_err(Error::Rocksdb)
}

pub mod bundled {
    pub static CONTRACT_3_6_4: &[u8] = include_bytes!("../../../etc/res/aurora-engine-3.6.4.wasm");
    pub static CONTRACT_3_7_0: &[u8] = include_bytes!("../../../etc/res/aurora-engine-3.7.0.wasm");
    pub static CONTRACT_3_9_0: &[u8] = include_bytes!("../../../etc/res/aurora-engine-3.9.0.wasm");
    pub static CONTRACT_3_9_1: &[u8] = include_bytes!("../../../etc/res/aurora-engine-3.9.1.wasm");
    pub static CONTRACT_3_9_2: &[u8] = include_bytes!("../../../etc/res/aurora-engine-3.9.2.wasm");

    pub fn get(version: &str) -> Option<&'static [u8]> {
        match version {
            "3.6.4" => Some(CONTRACT_3_6_4),
            "3.7.0" => Some(CONTRACT_3_7_0),
            "3.9.0" => Some(CONTRACT_3_9_0),
            "3.9.1" => Some(CONTRACT_3_9_1),
            "3.9.2" => Some(CONTRACT_3_9_2),
            _ => None,
        }
    }
}
