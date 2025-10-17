use engine_standalone_storage::{Error, Storage};

const CONTRACT_KEY: &[u8] = b"\0";

// TODO(vlad): receive contract from nats
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

pub fn get_contract(
    storage: &Storage,
    height: u64,
    pos: u16,
    version: &str,
) -> Result<Option<Vec<u8>>, Error> {
    if let Some(data) = storage
        .get_custom_data_at(CONTRACT_KEY, height, pos)
        .map_err(Error::Rocksdb)?
    {
        return Ok(Some(data));
    }

    let key = [&CONTRACT_KEY[..], version.as_bytes()].concat();
    if let Some(data) = storage.get_custom_data(&key).map_err(Error::Rocksdb)? {
        store_contract(storage, height, pos, &data)?;
        // TODO(vlad): consider removing versioned data after storing it in height/pos
        // storage.remove_custom_data(&key)?;
        return Ok(Some(data));
    }

    Ok(None)
}
