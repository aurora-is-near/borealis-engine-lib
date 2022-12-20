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
                let near_tx_hash = match self.tx_tracker.get_tx_hash(rx_hash) {
                    Some(tx_hash) => tx_hash,
                    None => {
                        tracing::warn!("Transaction provenance unknown for receipt {}", rx_hash);
                        Default::default()
                    }
                };
                self.handler
                    .on_execution_outcome(near_block, near_tx_hash, outcome, &txs);
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
