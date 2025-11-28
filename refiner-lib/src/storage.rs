use std::path::Path;

use aurora_engine_types::{H256, U256, account_id::AccountId};
use engine_standalone_storage::{Storage, StoragePrefix};

/// Must match the VERSION in `engine_standalone_storage`
const VERSION: u8 = 0;
/// Write to the DB in batches of 100k heights at a time
const BATCH_SIZE: usize = 100_000;

pub fn init_storage<P: AsRef<Path>>(storage_path: P, account_id: &AccountId, chain_id: u64) {
    migrate_block_hash(storage_path, account_id, chain_id);
}

fn migrate_block_hash<P: AsRef<Path>>(
    storage_path: P,
    account_id: &AccountId,
    chain_id: u64,
) -> Storage {
    let chain_id = aurora_engine_types::types::u256_to_arr(&U256::from(chain_id));
    let mut storage = Storage::open_ensure_account_id(&storage_path, account_id).unwrap();
    let (block_hash, block_height) = match storage.get_latest_block() {
        Ok(x) => x,
        // If there are no blocks then there is nothing to migrate
        Err(_) => return storage,
    };
    let computed_block_hash =
        aurora_engine::engine::compute_block_hash(chain_id, block_height, account_id.as_bytes());
    if computed_block_hash != block_hash {
        // Block hash is not what is expected, so we need to migrate
        tracing::info!("Detected incorrect blockhash. Performing migration");

        // Close the current storage instance because we're going to need low-level access to the DB.
        let (_, mut block_height) = storage.get_earliest_block().unwrap();
        drop(storage);
        let db = rocksdb::DB::open_default(&storage_path).unwrap();

        while let MigrationStatus::Continue(height) =
            block_hash_migration_batch(&db, block_height, account_id.as_bytes(), chain_id)
        {
            block_height = height;
            tracing::debug!("Migrated up to height {}", block_height);
        }

        // Close low-level access, and open new Storage instance
        drop(db);
        tracing::info!("Migration complete.");
        storage = Storage::open_ensure_account_id(storage_path, account_id).unwrap();
    }
    storage
}

fn block_hash_migration_batch(
    db: &rocksdb::DB,
    start_height: u64,
    account_id: &[u8],
    chain_id: [u8; 32],
) -> MigrationStatus {
    let mut batch: Vec<(u64, H256, H256)> = Vec::with_capacity(BATCH_SIZE);
    let start_key = construct_storage_key(StoragePrefix::BlockHash, &start_height.to_be_bytes());
    let mut iter = db.prefix_iterator(&start_key);
    let mut return_status = MigrationStatus::Continue(start_height);

    // Collect heights and hashes to migrate in this batch
    while batch.len() < BATCH_SIZE {
        match iter.next().map(Result::unwrap) {
            None => {
                return_status = MigrationStatus::Complete;
                break;
            }
            Some((key, value)) => {
                let is_block_hash_key = key
                    .get(1)
                    .map(|b| b == &(StoragePrefix::BlockHash as u8))
                    .unwrap_or(false);
                if !is_block_hash_key {
                    return_status = MigrationStatus::Complete;
                    break;
                }

                let block_height = {
                    let mut buf = [0u8; 8];
                    // First two bytes are VERSION and StoragePrefix::BlockHash, remaining 8 bytes are the height
                    buf.copy_from_slice(&key[2..10]);
                    u64::from_be_bytes(buf)
                };
                let block_hash = H256::from_slice(&value);
                let new_hash =
                    aurora_engine::engine::compute_block_hash(chain_id, block_height, account_id);
                if new_hash != block_hash {
                    batch.push((block_height, block_hash, new_hash));
                }
                return_status = MigrationStatus::Continue(block_height + 1);
            }
        }
    }

    let mut write_batch = rocksdb::WriteBatch::default();
    for (block_height, old_hash, new_hash) in batch {
        let old_metadata_key =
            construct_storage_key(StoragePrefix::BlockMetadata, old_hash.as_bytes());
        let metadata = db.get_pinned(old_metadata_key).unwrap().unwrap();
        let block_height_bytes = block_height.to_be_bytes();
        write_batch.put(
            construct_storage_key(StoragePrefix::BlockHash, &block_height_bytes),
            new_hash,
        );
        write_batch.put(
            construct_storage_key(StoragePrefix::BlockHeight, new_hash.as_bytes()),
            block_height_bytes,
        );
        write_batch.put(
            construct_storage_key(StoragePrefix::BlockMetadata, new_hash.as_bytes()),
            metadata,
        );
    }
    db.write(write_batch).unwrap();

    return_status
}

enum MigrationStatus {
    Continue(u64),
    Complete,
}

fn construct_storage_key(prefix: StoragePrefix, key: &[u8]) -> Vec<u8> {
    [&[VERSION], &[prefix as u8], key].concat()
}
