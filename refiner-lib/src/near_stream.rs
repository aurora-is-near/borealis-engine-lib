use crate::metrics::{PROCESSED_BLOCKS, SKIP_BLOCKS};
use crate::refiner_inner::Refiner;
use crate::tx_hash_tracker::TxHashTracker;
use aurora_engine_modexp::AuroraModExp;
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

    async fn handle_block(&mut self, near_block: &NEARBlock) -> AuroraBlock {
        self.handler.on_block_start(near_block);

        let mut txs = Default::default();

        // Can specify a concrete modexp algorithm here because only transactions
        // that executed successfully on-chain are executed again here.
        aurora_standalone_engine::consume_near_block::<AuroraModExp>(
            near_block,
            &mut self.context,
            Some(&mut txs),
        )
        .await
        .unwrap(); // Panic if engine can't consume this block

        // Panic if transaction hash tracker cannot consume the block
        self.tx_tracker
            .consume_near_block(near_block)
            .expect("Transaction tracker consume_near_block error");

        let storage = self.context.storage.as_ref().write().await;
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
                    &storage,
                );
            });

        let aurora_block = self.handler.on_block_end(near_block);
        self.tx_tracker
            .on_block_end(near_block.block.header.height)
            .expect("Transaction tracker on_block_end error");
        aurora_block
    }

    pub async fn next_block(&mut self, near_block: &NEARBlock) -> Vec<AuroraBlock> {
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
        let block = self.handle_block(near_block).await;
        blocks.push(block);
        PROCESSED_BLOCKS.inc();

        blocks
    }
}

#[cfg(test)]
mod tests {
    use aurora_refiner_types::aurora_block::NearBlock;
    use engine_standalone_storage::json_snapshot::{initialize_engine_state, types::JsonSnapshot};
    use std::{collections::HashSet, matches};

    use super::*;

    #[tokio::test]
    async fn test_block_120572296() {
        // The testnet block at height 120572296 contains a `DelegateAction` action.
        // See https://github.com/near/NEPs/blob/master/neps/nep-0366.md for details.

        let db_dir = tempfile::tempdir().unwrap();
        let ctx = TestContext::new(&db_dir);
        let mut stream = ctx.create_stream();
        let block = read_block("tests/res/testnet-block-120572296.json");

        let mut aurora_blocks = stream.next_block(&block).await;

        assert_eq!(aurora_blocks.len(), 1);
        let aurora_block = aurora_blocks.pop().unwrap();
        assert!(aurora_block.transactions.is_empty());
    }

    #[tokio::test]
    async fn test_block_89402026() {
        let db_dir = tempfile::tempdir().unwrap();
        let ctx = TestContext::new(&db_dir);
        let mut stream = ctx.create_stream();
        let block = read_block("tests/res/block-89402026.json");

        let mut aurora_blocks = stream.next_block(&block).await;
        assert_eq!(aurora_blocks.len(), 1);
        let aurora_block = aurora_blocks.pop().unwrap();

        let tx_count = aurora_block.transactions.len();
        let unique_txs: HashSet<_> = aurora_block.transactions.iter().map(|tx| tx.hash).collect();
        assert_eq!(tx_count, unique_txs.len());
    }

    #[tokio::test]
    async fn test_block_84423722() {
        // The block at hight 84423722 contains a transaction with zero actions.
        // The refiner should be able to process such a block without crashing.

        let db_dir = tempfile::tempdir().unwrap();
        let ctx = TestContext::new(&db_dir);
        let mut stream = ctx.create_stream();
        let block = read_block("tests/res/block-84423722.json");

        let mut aurora_blocks = stream.next_block(&block).await;

        assert_eq!(aurora_blocks.len(), 1);
        let aurora_block = aurora_blocks.pop().unwrap();
        assert!(aurora_block.transactions.is_empty());
    }

    #[tokio::test]
    async fn test_block_81206675() {
        let db_dir = tempfile::tempdir().unwrap();
        let ctx = TestContext::new(&db_dir);
        let mut stream = ctx.create_stream();
        let block = read_block("tests/res/block-81206675.json");

        let mut aurora_blocks = stream.next_block(&block).await;

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

    #[tokio::test]
    async fn test_block_82654651_nonce() {
        // load state snapshot and main objects
        let db_dir = tempfile::tempdir().unwrap();
        let mut ctx = TestContext::new(&db_dir);
        ctx.init_with_snapshot("tests/res/sate_H7Bfh9qCzWbJW9acao8B2jFMTrkfc31toczmTcMv7hY7.json")
            .await;
        let mut stream = ctx.create_stream();

        // parameters of the test
        let block = read_block("tests/res/block-82654651.json");
        let expected_nonce = 12773;

        // run and assert
        let aurora_block = stream.next_block(&block).await.pop().unwrap();

        assert_eq!(aurora_block.transactions.len(), 1);
        let target_aurora_tx = aurora_block.transactions.first().unwrap();

        assert_eq!(target_aurora_tx.nonce, expected_nonce);
    }

    #[tokio::test]
    async fn test_block_70834061_skip_block() {
        let db_dir = tempfile::tempdir().unwrap();
        let ctx = TestContext::new(&db_dir);
        let mut stream = ctx.create_stream();

        // near block 70834059
        let near_block = read_block("tests/res/block-70834059.json");

        let aurora_blocks = stream.next_block(&near_block).await;

        assert_eq!(aurora_blocks.len(), 1);
        assert_eq!(aurora_blocks[0].height, 70834059);
        assert!(matches!(
            aurora_blocks[0].near_metadata,
            NearBlock::ExistingBlock(..)
        ));

        // near skip block 70834061; 70834060 does not exist
        let near_skip_block = read_block("tests/res/block-70834061.json");

        let aurora_blocks = stream.next_block(&near_skip_block).await;

        assert_eq!(aurora_blocks.len(), 2);
        assert_eq!(aurora_blocks[0].height, 70834060); // dummy skip aurora block
        assert_eq!(aurora_blocks[1].height, 70834061);
        assert!(matches!(
            aurora_blocks[0].near_metadata,
            NearBlock::SkipBlock
        ));
        assert!(matches!(
            aurora_blocks[1].near_metadata,
            NearBlock::ExistingBlock(..)
        ));
    }

    #[tokio::test]
    async fn test_block_34834052_block_before_aurora_genesis() {
        let db_dir = tempfile::tempdir().unwrap();
        let ctx = TestContext::new(&db_dir);
        let mut stream = ctx.create_stream();

        // near block 34834052; aurora block genesis is 34834053
        let near_block = read_block("tests/res/block-34834052.json");

        let aurora_blocks = stream.next_block(&near_block).await;

        assert_eq!(aurora_blocks.len(), 1);
        assert_eq!(aurora_blocks[0].height, 34834052);
        assert!(matches!(
            aurora_blocks[0].near_metadata,
            NearBlock::ExistingBlock(..)
        ));
    }

    fn read_block(path: &str) -> NEARBlock {
        let data = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&data).unwrap()
    }

    struct TestContext {
        chain_id: u64,
        engine_context: EngineContext,
        tx_tracker: TxHashTracker,
    }

    impl TestContext {
        fn new(db_dir: &tempfile::TempDir) -> Self {
            let engine_path = db_dir.path().join("engine");
            let tracker_path = db_dir.path().join("tracker");
            let chain_id = 1313161554_u64;
            crate::storage::init_storage(engine_path.clone(), "aurora".into(), chain_id);
            let engine_context =
                EngineContext::new(&engine_path, "aurora".parse().unwrap(), chain_id).unwrap();
            let tx_tracker = TxHashTracker::new(tracker_path, 0).unwrap();
            Self {
                chain_id,
                engine_context,
                tx_tracker,
            }
        }

        async fn init_with_snapshot(&mut self, snapshot_path: &str) {
            let json_snapshot: JsonSnapshot = {
                let json_snapshot_data = std::fs::read_to_string(snapshot_path).unwrap();
                serde_json::from_str(&json_snapshot_data).unwrap()
            };
            let mut storage = self.engine_context.storage.as_ref().write().await;
            initialize_engine_state(&mut storage, json_snapshot).unwrap();
        }

        fn create_stream(self) -> NearStream {
            NearStream::new(self.chain_id, None, self.engine_context, self.tx_tracker)
        }
    }
}
