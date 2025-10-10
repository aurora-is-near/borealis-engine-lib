use engine_standalone_storage::{Error, Storage};

const CONTRACT_KEY: &[u8] = b"\0";

// TODO(vlad): receive contract from nats
#[allow(dead_code)]
pub fn store_contract(storage: &Storage, height: u64, pos: u16, value: &[u8]) -> Result<(), Error> {
    storage
        .set_custom_data_at(CONTRACT_KEY, height, pos, &value)
        .map_err(Error::Rocksdb)
}

pub fn get_contract(storage: &Storage, height: u64, pos: u16) -> Result<Option<Vec<u8>>, Error> {
    storage
        .get_custom_data_at(CONTRACT_KEY, height, pos)
        .map_err(Error::Rocksdb)
}
