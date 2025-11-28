use std::{fs, io, path::PathBuf};

use engine_standalone_storage::{Error, Storage, WasmInitError};
use near_primitives_core::hash::CryptoHash;

const CONTRACT_KEY: &[u8] = b"\0";

pub fn store_contract(storage: &Storage, height: u64, pos: u16, value: &[u8]) -> Result<(), Error> {
    storage
        .set_custom_data_at(CONTRACT_KEY, height, pos, &value)
        .map_err(Error::Rocksdb)
}

pub fn store_contract_by_version(
    storage: &Storage,
    version: &str,
    value: &[u8],
) -> Result<(), Error> {
    storage
        .set_custom_data(&[&CONTRACT_KEY[..], version.as_bytes()].concat(), &value)
        .map_err(Error::Rocksdb)
}

#[derive(Debug, thiserror::Error)]
pub enum ContractApplyError {
    #[error("WASM initialization error: {0}")]
    BadCode(#[from] WasmInitError),
    #[error("database error: {0:?}")]
    Db(Error),
    #[error("I/O error: {0}")]
    LoadError(#[from] io::Error),
    #[error("Contract not found by height/pos and no version provided")]
    NotFound,
}

pub fn apply_contract(
    storage: &mut Storage,
    height: u64,
    pos: u16,
    version: Option<&str>,
    override_prefix: Option<PathBuf>,
) -> Result<(), ContractApplyError> {
    if let Some(bytes) =
        get_contract(storage, height, pos, version).map_err(ContractApplyError::Db)?
    {
        storage.runner_mut().set_code(bytes)?;
        return Ok(());
    }
    if let Some(version) = version {
        let (bytes, _) = load_from_file(&version, override_prefix)?;
        storage.runner_mut().set_code(bytes)?;
        Ok(())
    } else {
        Err(ContractApplyError::NotFound)
    }
}

pub fn get_contract(
    storage: &Storage,
    height: u64,
    pos: u16,
    version: Option<&str>,
) -> Result<Option<Vec<u8>>, Error> {
    if let Some(data) = storage
        .get_custom_data_at(CONTRACT_KEY, height, pos)
        .map_err(Error::Rocksdb)?
    {
        return Ok(Some(data));
    }

    if let Some(version) = version {
        let key = [&CONTRACT_KEY[..], version.as_bytes()].concat();
        if let Some(data) = storage.get_custom_data(&key).map_err(Error::Rocksdb)? {
            store_contract(storage, height, pos, &data)?;
            // TODO(vlad): consider removing versioned data after storing it in height/pos
            // storage.remove_custom_data(&key)?;
            return Ok(Some(data));
        }
    }

    Ok(None)
}

pub fn load_from_file(
    version: &str,
    override_prefix: Option<PathBuf>,
) -> io::Result<(Vec<u8>, Option<CryptoHash>)> {
    let prefix = override_prefix.clone().unwrap_or_else(|| "etc/res".into());
    let path = prefix.join(format!("aurora-engine-{}.wasm", version));
    fs::read(&path)
        .map(|code| (code, None))
        .map_err(|e| {
            let err = format!("Failed to read `{}`: {e}", path.display());
            io::Error::new(e.kind(), err)
        })
        .or_else(|err| {
            if override_prefix.is_none() {
                // tests are run from the crate root, not from workspace root
                load_from_file(version, Some(PathBuf::from("../etc/res")))
            } else {
                Err(err)
            }
        })
}
