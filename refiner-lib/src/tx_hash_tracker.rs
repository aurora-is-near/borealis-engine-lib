use aurora_refiner_types::{near_block::NEARBlock, near_primitives::hash::CryptoHash};
use std::path::Path;

/// A helper object for tracking the NEAR transaction hash that caused each NEAR receipt
/// to be produced (potentially indirectly via a number of other receipts). The main interface
/// includes the `get_tx_hash` function to query the helper for the transaction hash associated
/// with a given receipt hash, and two functions to mutate the tracker's state:
/// `consume_near_block` and `on_block_end`. The purpose of `consume_near_block` is to update
/// the tracker's state with the new receipts that are created in that block. The purpose of
/// `on_block_end` is to give the tracker an opportunity to prune away old state that is no
/// longer needed since the block at the given height is finished processing.
pub struct TxHashTracker {
    inner: TxHashTrackerImpl,
}

impl TxHashTracker {
    pub fn new<P: AsRef<Path>>(storage_path: P, start_height: u64) -> anyhow::Result<Self> {
        let inner = TxHashTrackerImpl::new(storage_path, start_height)?;
        Ok(Self { inner })
    }

    pub fn get_tx_hash(&mut self, rx_hash: &CryptoHash) -> Option<CryptoHash> {
        self.inner.get_tx_hash(rx_hash)
    }

    pub fn consume_near_block(&mut self, near_block: &NEARBlock) -> anyhow::Result<()> {
        let block_height = near_block.block.header.height;

        let tx_iter = near_block
            .shards
            .iter()
            .filter_map(|s| s.chunk.as_ref())
            .flat_map(|c| c.transactions.iter());

        // Track receipts created from transactions
        for tx in tx_iter {
            let tx_hash = tx.transaction.hash;
            for rx_hash in tx.outcome.execution_outcome.outcome.receipt_ids.iter() {
                self.inner.record_rx(*rx_hash, tx_hash, block_height)?;
            }
        }

        let rx_iter = near_block
            .shards
            .iter()
            .flat_map(|s| s.receipt_execution_outcomes.iter());

        // Track receipts created from other receipts
        for rx in rx_iter {
            let rx_hash = &rx.receipt.receipt_id;
            let tx_hash = match self.get_tx_hash(rx_hash) {
                Some(tx_hash) => tx_hash,
                None => {
                    tracing::warn!("Transaction provenance unknown for receipt {}", rx_hash);
                    continue;
                }
            };
            for rx_hash in rx.execution_outcome.outcome.receipt_ids.iter() {
                self.inner.record_rx(*rx_hash, tx_hash, block_height)?;
            }
        }

        Ok(())
    }

    pub fn on_block_end(&mut self, block_height: u64) -> anyhow::Result<()> {
        self.inner.prune_state(block_height)
    }
}

/// This struct is intentionally private and contains the core state-management logic of the
/// transaction hash tracker. The reason to separate this from the public struct above is to
/// allow the implementation details of the state to change without impacting the public
/// interface.
/// The cache in the implementation allows for fast (in-memory) look-ups, while the rocksdb
/// storage layer enables crash recovery without data loss.
struct TxHashTrackerImpl {
    cache: lru::LruCache<CryptoHash, CryptoHash>,
    persistent_storage: rocksdb::DB,
}

/// At 64 bytes per entry (two 32-byte hashes), this caps the memory footprint of the tracker
/// at around 70 MB, which seems reasonable. One million entries should also be sufficient to
/// ensure the cache never miss under normal conditions; it would require a receipt to be
/// created and then not included in a block before one million other receipts we created first.
/// With the maximum daily number of transactions ever observed on NEAR at just over two million,
/// this means the receipt would not have been included in a block for at least 8 hours; an
/// extremely unlikely event (typically receipts are included in the next block after they are
/// created, less than 2 seconds later).
const CACHE_SIZE: usize = 1_000_000;

/// This is the number of block heights into the past the DB will remember receipt hashes.
/// With a 1 second block time, this corresponds to 5 days of transactions, or 10 "epochs"
/// since NEAR epochs tend to be around 12 hours. Keeping data for 5 epochs is the standard for
/// non-archival nearcore nodes, so 10 epochs should be more than enough for us. At NEAR's peak of
/// two million transactions per day, and assuming each transaction has 5 receipts on average
/// (likely an overestimate), then this will mean fifty million entries in the DB at most.
/// With 72 bytes per entry (two 32-byte hashes plus one 8-byte height), this will cap the
/// storage used by this DB at under 4 GB.
const PERSISTENT_HISTORY_SIZE: u64 = 432_000;

impl TxHashTrackerImpl {
    fn new<P: AsRef<Path>>(storage_path: P, start_height: u64) -> anyhow::Result<Self> {
        let mut cache = lru::LruCache::new(CACHE_SIZE.try_into()?);
        let persistent_storage = rocksdb::DB::open_default(storage_path)?;

        // Read out enough data from the DB to fill the cache
        let opts = {
            let mut tmp = rocksdb::ReadOptions::default();
            let db_key = [start_height.to_be_bytes().as_slice(), &[0xff_u8; 32]].concat();
            tmp.set_iterate_upper_bound(db_key);
            tmp
        };
        let iter = persistent_storage
            .iterator_opt(rocksdb::IteratorMode::End, opts)
            .take(CACHE_SIZE);
        let mut cache_data = iter.collect::<Vec<_>>();
        // Consume the data from the DB in reverse order to get right LRU structure
        cache_data.reverse();
        for entry in cache_data {
            let (k, v) = entry?;
            if k.len() < 8 {
                return Err(anyhow::anyhow!(
                    "Invalid transaction tracker DB key: {}",
                    hex::encode(k)
                ));
            }
            cache.put(slice_to_crypto_hash(&k[8..])?, slice_to_crypto_hash(&v)?);
        }

        Ok(Self {
            cache,
            persistent_storage,
        })
    }

    /// Uses the LRU cache to get the transaction hash associated with the given receipt hash.
    ///
    /// We intentionally do not fall back on the rocksdb storage layer in the event of a cache
    /// miss. This is because the rocksdb storage layer is optimized for fast pruning by
    /// prepending each receipt hash with the block height where it was created (this means the
    /// keys are chronologically ordered and thus pruning old keys acts on a contiguous section
    /// of the DB). However, this optimization means we cannot look up a transaction hash from
    /// a receipt hash alone, we need the block height the receipt was created at as well, but
    /// that information is not easily available. Note that fast pruning is required to ensure
    /// the size of the state needed to run the refiner stays bounded.
    ///
    /// Therefore, the cache must be large enough such that misses never happen under normal
    /// conditions and the cache must be populated eagerly from the rocksdb storage layer
    /// on start-up.
    fn get_tx_hash(&mut self, rx_hash: &CryptoHash) -> Option<CryptoHash> {
        self.cache.get(rx_hash).copied()
    }

    fn record_rx(
        &mut self,
        rx_hash: CryptoHash,
        tx_hash: CryptoHash,
        block_height: u64,
    ) -> anyhow::Result<()> {
        self.cache.put(rx_hash, tx_hash);

        let db_key = [&block_height.to_be_bytes(), rx_hash.as_ref()].concat();
        self.persistent_storage.put(db_key, tx_hash)?;
        Ok(())
    }

    fn prune_state(&mut self, completed_block_height: u64) -> anyhow::Result<()> {
        let prune_height = completed_block_height.saturating_sub(PERSISTENT_HISTORY_SIZE);

        let start_key = vec![0u8; 40];
        let end_key = [prune_height.to_be_bytes().as_slice(), &[0xff_u8; 32]].concat();
        let mut batch = rocksdb::WriteBatch::default();
        batch.delete_range(start_key, end_key);
        self.persistent_storage.write(batch)?;

        Ok(())
    }
}

fn slice_to_crypto_hash(slice: &[u8]) -> anyhow::Result<CryptoHash> {
    slice
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid hash: {}", hex::encode(slice)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_refiner_types::near_primitives::serialize::BaseDecode;

    #[test]
    fn test_transaction_hash_tracker() {
        let db_dir = tempfile::tempdir().unwrap();
        let mut tracker = TxHashTracker::new(db_dir.path(), 0).unwrap();

        let block_1 = read_block("blocks/block-34834053.json");
        let block_2 = read_block("blocks/block-51188689.json");
        let block_3 = read_block("blocks/block-51188690.json");

        tracker.consume_near_block(&block_1).unwrap();

        // Receipt <-> Transaction mapping should be present after consuming a block
        let rx_hash_1 =
            CryptoHash::from_base("EkdeeCvozyK5zXZSd3tk2VpakH4kiGfLHCBzfXZVAVvd").unwrap();
        let expected_tx_hash_1 =
            CryptoHash::from_base("uQ6hiGNnVW371JKwnLLQM4iMdBNE7xJqVjPGecbmL4D").unwrap();
        assert_eq!(tracker.get_tx_hash(&rx_hash_1).unwrap(), expected_tx_hash_1,);

        tracker.on_block_end(34834053).unwrap();

        // Mapping should still be present on restart
        drop(tracker);
        let mut tracker = TxHashTracker::new(db_dir.path(), 34834053).unwrap();
        assert_eq!(tracker.get_tx_hash(&rx_hash_1).unwrap(), expected_tx_hash_1,);

        // Try consuming another block
        tracker.consume_near_block(&block_2).unwrap();

        let rx_hash_2 =
            CryptoHash::from_base("9qNqNxE6LenxsFMFmzf9RdQwH6MqhU7Hfqnq7GoibYK8").unwrap();
        let expected_tx_hash_2 =
            CryptoHash::from_base("DEtAE5d6M8NtBMsCaZVCzjg8C2a5wqduhwVkioseUhT4").unwrap();
        assert_eq!(tracker.get_tx_hash(&rx_hash_2).unwrap(), expected_tx_hash_2,);

        tracker.on_block_end(51188689).unwrap();

        // After restart the first block should have been pruned since it is a much lower height
        drop(tracker);
        let mut tracker = TxHashTracker::new(db_dir.path(), 51188689).unwrap();
        assert_eq!(tracker.get_tx_hash(&rx_hash_1), None,);
        // But the just consumed block is still present
        assert_eq!(tracker.get_tx_hash(&rx_hash_2).unwrap(), expected_tx_hash_2,);

        // Consume the next block
        tracker.consume_near_block(&block_3).unwrap();

        // This receipt comes from receipt `rx_hash_2`, which in turn came from transaction `expected_tx_hash_2`.
        // Therefore, we expect this receipt also should be associated with `expected_tx_hash_2`.
        let rx_hash_3 =
            CryptoHash::from_base("3d43nGKmmbXbCCtt12NAAPLfEoaRo3j31CEKaiQCK3Bt").unwrap();
        assert_eq!(tracker.get_tx_hash(&rx_hash_3).unwrap(), expected_tx_hash_2,);

        tracker.on_block_end(51188690).unwrap();

        // And both receipts are still present after restart
        drop(tracker);
        let mut tracker = TxHashTracker::new(db_dir.path(), 51188690).unwrap();
        assert_eq!(tracker.get_tx_hash(&rx_hash_2).unwrap(), expected_tx_hash_2,);
        assert_eq!(tracker.get_tx_hash(&rx_hash_3).unwrap(), expected_tx_hash_2,);
    }

    fn read_block(path: &str) -> NEARBlock {
        let data = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&data).unwrap()
    }
}
