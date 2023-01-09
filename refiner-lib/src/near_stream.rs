use crate::metrics::{PROCESSED_BLOCKS, SKIP_BLOCKS};
use crate::refiner_inner::Refiner;
use crate::tx_hash_tracker::TxHashTracker;
use aurora_refiner_types::aurora_block::AuroraBlock;
use aurora_refiner_types::near_block::NEARBlock;
use aurora_standalone_engine::EngineContext;

pub struct NearStream {
    /// Keep track of last block seen, to report empty blocks
    last_block_height: Option<u64>,
    /// Pass the filtered information to the handler
    handler: Refiner,
    /// Context used to access engine
    context: EngineContext,
    /// Helper to track the NEAR transaction hash associated with each NEAR receipt.
    tx_tracker: TxHashTracker,
}

impl NearStream {
    pub fn new(
        chain_id: u64,
        last_block_height: Option<u64>,
        context: EngineContext,
        tx_tracker: TxHashTracker,
    ) -> Self {
        Self {
            last_block_height,
            handler: Refiner::new(chain_id),
            context,
            tx_tracker,
        }
    }

    fn handle_block(&mut self, near_block: &NEARBlock) -> AuroraBlock {
        self.handler.on_block_start(near_block);

        let mut txs = Default::default();

        // Panic if engine can't consume this block
        aurora_standalone_engine::consume_near_block(near_block, &mut self.context, Some(&mut txs))
            .unwrap();

        // Panic if transaction hash tracker cannot consume the block
        self.tx_tracker
            .consume_near_block(near_block)
            .expect("Transaction tracker consume_near_block error");

        near_block
            .shards
            .iter()
            .flat_map(|shard| shard.receipt_execution_outcomes.as_slice())
            .filter(|outcome| {
                outcome.receipt.receiver_id.as_bytes() == self.context.engine_account_id.as_bytes()
            })
            .for_each(|outcome| {
                let rx_hash = &outcome.receipt.receipt_id;
                let near_tx_hash = self.tx_tracker.get_tx_hash(rx_hash);
                if near_tx_hash.is_none() {
                    tracing::warn!("Transaction provenance unknown for receipt {}", rx_hash);
                    crate::metrics::UNKNOWN_TX_FOR_RECEIPT.inc();
                }
                self.handler.on_execution_outcome(
                    near_block,
                    near_tx_hash,
                    outcome,
                    &txs,
                    &self.context.storage,
                );
            });

        let aurora_block = self.handler.on_block_end(near_block);
        self.tx_tracker
            .on_block_end(near_block.block.header.height)
            .expect("Transaction tracker on_block_end error");
        aurora_block
    }

    pub fn next_block(&mut self, near_block: &NEARBlock) -> Vec<AuroraBlock> {
        let mut blocks = vec![];

        let height = near_block.block.header.height;

        // Emit events for all skip blocks
        let mut last_height = self.last_block_height.unwrap_or(height);
        while last_height + 1 < height {
            last_height += 1;
            let skip_block = self.handler.on_block_skip(last_height, near_block);
            blocks.push(skip_block);
            SKIP_BLOCKS.inc();
        }

        self.last_block_height = Some(height);
        let block = self.handle_block(near_block);
        blocks.push(block);
        PROCESSED_BLOCKS.inc();

        blocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_81206675() {
        let db_dir = tempfile::tempdir().unwrap();
        let engine_path = db_dir.path().join("engine");
        let tracker_path = db_dir.path().join("tracker");
        let chain_id = 1313161554_u64;
        crate::storage::init_storage(engine_path.clone(), "aurora".into(), chain_id);
        let ctx = EngineContext::new(&engine_path, "aurora".parse().unwrap(), chain_id).unwrap();
        let tx_tracker = TxHashTracker::new(tracker_path, 0).unwrap();
        let mut stream = NearStream::new(chain_id, None, ctx, tx_tracker);
        let block: NEARBlock = {
            let data = std::fs::read_to_string("blocks/block-81206675.json").unwrap();
            serde_json::from_str(&data).unwrap()
        };

        let mut aurora_blocks = stream.next_block(&block);

        assert_eq!(aurora_blocks.len(), 1);
        let aurora_block = aurora_blocks.pop().unwrap();

        assert_eq!(
            hex::encode(aurora_block.hash),
            "0a007345d45f931532063ff5bb0d267b5af940e8ca2ccb0cdc81e37664c82ba4"
        );
        assert_eq!(
            hex::encode(aurora_block.transactions_root),
            "c467fc63b0524d8896f235f1a1af975dcf5f2b5c1353270db9637c4f902d1d5b"
        );
        assert_eq!(
            hex::encode(aurora_block.state_root),
            "49d90ec7938074f982813e8e0186085bda029c6579ac50c836622860251fd696"
        );

        let tx_1 = &aurora_block.transactions[0];
        assert_eq!(
            hex::encode(tx_1.hash),
            "172794dc3ee343c1fc7cdf5170e2aa61372a3d947fe042b106286f99454ab6ff"
        );
        assert_eq!(
            hex::encode(tx_1.from.as_bytes()),
            "c4fe580eabe347a7be55a2976bcd75293b837753"
        );
        assert_eq!(
            hex::encode(tx_1.to.unwrap().as_bytes()),
            "713e400b032b89db9f68105e501ff13260398490"
        );
        assert_eq!(tx_1.logs.len(), 3);

        let tx_2 = &aurora_block.transactions[1];
        assert_eq!(
            hex::encode(tx_2.hash),
            "a341c7b2f7f27f68f5b7bf6c8ca008f9af7e80dc3ee03ced45a28deb61b5bfd4"
        );
        assert_eq!(
            hex::encode(tx_2.from.as_bytes()),
            "b3072378821cdafac340bf18a0fbf15c72feb83b"
        );
        assert_eq!(
            hex::encode(tx_2.to.unwrap().as_bytes()),
            "c6e5185438e1730959c1ef3551059a3fec744e90"
        );
        assert_eq!(tx_2.logs.len(), 1);
    }
}
