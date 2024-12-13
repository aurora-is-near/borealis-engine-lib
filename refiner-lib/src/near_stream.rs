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
        let engine_account_id = context
            .engine_account_id
            .as_ref()
            .parse()
            .expect("Engine account ID must be valid");
        Self {
            last_block_height,
            handler: Refiner::new(chain_id, engine_account_id),
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
pub mod tests {
    use aurora_engine::{
        engine::setup_receive_erc20_tokens_input, parameters::NEP141FtOnTransferArgs,
        state::EngineStateError,
    };
    use aurora_engine_sdk::types::near_account_to_evm_address;
    use aurora_engine_types::{
        account_id::AccountId,
        types::{Address, Balance, Wei},
        U256,
    };
    use aurora_refiner_types::aurora_block::NearBlock;
    use engine_standalone_storage::json_snapshot::{initialize_engine_state, types::JsonSnapshot};
    use std::{collections::HashSet, matches, str::FromStr};

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
    async fn test_block_131407300() {
        // The block 131407300 contains a receipt with batch of actions, one of which inits silo
        // contract. The test checks that init action in the batch is processed correctly and
        // initializes the borealis engine's state.
        let db_dir = tempfile::tempdir().unwrap();
        let chain_id = 1313161566;
        let ctx = TestContextBuilder::new()
            .with_account_id("0x4e45415e.c.aurora")
            .with_chain_id(chain_id)
            .build(&db_dir);
        let mut stream = ctx.create_stream();

        {
            let result = stream
                .context
                .storage
                .read()
                .await
                .with_engine_access(131407300, 1, &[], |io| aurora_engine::state::get_state(&io))
                .result;
            assert!(matches!(result, Err(EngineStateError::NotFound)));
        }

        let block = read_block("tests/res/block_131407300.json");
        let _ = stream.next_block(&block).await;
        let chain_id_from_state = stream
            .context
            .storage
            .read()
            .await
            .with_engine_access(131407300, 1, &[], |io| aurora_engine::state::get_state(&io))
            .result
            .map(|state| U256::from_big_endian(&state.chain_id).as_u64())
            .unwrap();
        assert_eq!(chain_id_from_state, chain_id);
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
        // The block at height 84423722 contains a transaction with zero actions.
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

    // Tests processing the transaction https://explorer.mainnet.near.org/transactions/964KbgjnkCfyUS1kaHVJNGuXAMsahdNHiP1jWkMnx1Bk
    // which is a bridge transfer of some tokens into Aurora. The ERC-20 logs should be present
    // based on the tokens that were minted from th bridging.
    #[tokio::test]
    async fn test_block_75306841_bridge_tx() {
        // load state snapshot and main objects
        let db_dir = tempfile::tempdir().unwrap();
        let mut ctx = TestContext::new(&db_dir);
        ctx.init_with_snapshot("tests/res/state_EVVnmqiPm6efCJGWLS5DgMTq3spVnevvh4fEgvc2e2Hz.json")
            .await;
        let mut stream = ctx.create_stream();

        // parameters of the test
        let block = read_block("tests/res/block-75306841.json");

        // run and assert
        let mut aurora_block = stream.next_block(&block).await.pop().unwrap();

        assert_eq!(aurora_block.transactions.len(), 1);
        let mut target_aurora_tx = aurora_block.transactions.pop().unwrap();

        assert_eq!(target_aurora_tx.logs.len(), 1);
        let log = target_aurora_tx.logs.pop().unwrap();

        // ERC-20 event hex signature for `Transfer(address,address,uint256)`
        // https://www.4byte.directory/event-signatures/?bytes_signature=0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef
        assert_eq!(
            hex::encode(log.topics[0]),
            "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
        );

        // Transfer from 0x00 (i.e. mint)
        assert_eq!(log.topics[1], [0u8; 32]);

        // Transfer to user's address (given in the `ft_on_transfer` message).
        assert_eq!(
            hex::encode(log.topics[2]),
            "000000000000000000000000852285d421bb5682470ad46e2eb99adf001ab9f1"
        );

        // Transfer amount equal to value specified in `ft_on_transfer`
        assert_eq!(
            aurora_engine_types::U256::from_big_endian(&log.data),
            13870504203340000000000_u128.into()
        );
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

    #[tokio::test]
    async fn test_block_128945880_contains_eth_token_mint() {
        let db_dir = tempfile::tempdir().unwrap();
        let ctx = TestContext::new(&db_dir);
        let mut stream = ctx.create_stream();

        let near_block = read_block("tests/res/block_128945880.json");

        let aurora_blocks = stream.next_block(&near_block).await;
        assert_eq!(aurora_blocks.len(), 1);
        assert_eq!(aurora_blocks[0].height, 128945880);
        assert!(matches!(
            aurora_blocks[0].near_metadata,
            NearBlock::ExistingBlock(..)
        ));

        // Expected values from base64-decoded args:
        // echo eyJzZW5kZXJfaWQiOiJhdXJvcmEiLCJhbW91bnQiOiIxMDAwMDAwMDAwMDAwMDAwMDAiLCJtc2ciOiIwYzdlMWYwM2Q2NzFiMTE4NWJlYTZmYjA2ZjExNGEwYmQ4YmJhMmY4In0=|base64 -D
        // {"sender_id":"aurora","amount":"100000000000000000","msg":"0c7e1f03d671b1185bea6fb06f114a0bd8bba2f8"}%
        let expected_sender = Address::decode("4444588443c3a91288c5002483449aba1054192b").unwrap(); // `sender_id` is near_account_to_evm_address("aurora")
        let expected_recipient =
            Address::decode("0c7e1f03d671b1185bea6fb06f114a0bd8bba2f8").unwrap(); // The recipient is the address from the args-decoded msg field
        let expected_amount = Wei::new(U256::from_dec_str("100000000000000000").unwrap());

        let aurora_block = aurora_blocks.first().unwrap();
        let ft_on_transfer_eth_tx = aurora_block.transactions.iter().find(|tx| {
            tx.from == expected_sender
                && tx.to == Some(expected_recipient)
                && tx.value == expected_amount
        });

        assert!(
            ft_on_transfer_eth_tx.is_some(),
            "Expected ft_on_transfer ETH mint transaction not found in block 128945880"
        );
    }

    #[tokio::test]
    async fn test_block_125229395_contains_erc20_token_mint() {
        let db_dir = tempfile::tempdir().unwrap();
        let ctx = TestContext::new(&db_dir);
        let mut stream = ctx.create_stream();

        // Read the block where the wNEAR contract is created to obtain a state that contains a key-value pair representing the wrap.near and ERC20 addresses.
        let near_block_wnear_contract_create = read_block("tests/res/block_42598892.json");
        let aurora_blocks = stream.next_block(&near_block_wnear_contract_create).await;
        assert_eq!(aurora_blocks.len(), 1);
        assert_eq!(aurora_blocks[0].height, 42598892);
        assert!(matches!(
            aurora_blocks[0].near_metadata,
            NearBlock::ExistingBlock(..)
        ));

        // Directly setting the height, we ensure the stream will process the target block (125229395)
        stream.last_block_height = Some(125229394);
        // Read the block that contains the ERC20 token mint transaction.
        let near_block = read_block("tests/res/block_125229395.json");
        let aurora_blocks = stream.next_block(&near_block).await;
        assert_eq!(aurora_blocks.len(), 1);
        assert_eq!(aurora_blocks[0].height, 125229395);
        assert!(matches!(
            aurora_blocks[0].near_metadata,
            NearBlock::ExistingBlock(..)
        ));

        // `sender_id` is near_account_to_evm_address("aurora")
        let expected_sender = Address::decode("4444588443c3a91288c5002483449aba1054192b").unwrap();

        // "wrap.near" -> nep141_account_id -> aurora_engine::engine::get_erc20_from_nep141
        let expected_recipient =
            Address::decode("c42c30ac6cc15fac9bd938618bcaa1a1fae8501d").unwrap();

        let expected_amount = Wei::zero();
        let expected_input = {
            // Expected arguments are extracted from the base64-encoded string of the NEAR receipt id "D4PhVsM2PFNgyc73mjR5oLYpz6rNAwBSy4rRo1Aariea"
            // echo eyJzZW5kZXJfaWQiOiI2NmZiMWQzZDBjOGIzODkzYjFiNTNhNGE5NjRhOGIwMzU4NmNjMGRiNWM5NjIxMDE0ZjU0ZWZiMTEwNjhiNzJlIiwiYW1vdW50IjoiMTE2MzM3NDg3MDg3NTg2NzY2ODk5NTAiLCJtc2ciOiIwZmU5NTdlNmFjYmI0ZmQ5MzVjZWU1YmEwMzNlMDAwODhkZjg2YWRiIn0=|base64 -D
            // {"sender_id":"66fb1d3d0c8b3893b1b53a4a964a8b03586cc0db5c9621014f54efb11068b72e","amount":"11633748708758676689950","msg":"0fe957e6acbb4fd935cee5ba033e00088df86adb"}%
            let args = NEP141FtOnTransferArgs {
                sender_id: AccountId::from_str(
                    "66fb1d3d0c8b3893b1b53a4a964a8b03586cc0db5c9621014f54efb11068b72e",
                )
                .unwrap(),
                amount: Balance::new(11633748708758676689950),
                msg: "0fe957e6acbb4fd935cee5ba033e00088df86adb".to_string(),
            };
            setup_receive_erc20_tokens_input(&args, &expected_recipient)
        };

        let aurora_block = aurora_blocks.first().unwrap();
        let ft_on_transfer_erc20_tx = aurora_block.transactions.iter().find(|tx| {
            tx.from == expected_sender
                && tx.to == Some(expected_recipient)
                && tx.value == expected_amount
                && tx.input == expected_input
        });

        assert!(
            ft_on_transfer_erc20_tx.is_some(),
            "Expected ft_on_transfer ERC20 mint transaction not found in block 125229395"
        );
    }

    #[tokio::test]
    async fn test_block_182018895_contains_deploy_erc20_token() {
        let db_dir = tempfile::tempdir().unwrap();
        let ctx = TestContextBuilder::new()
            .with_chain_id(1313161555)
            .build(&db_dir);
        let mut stream = ctx.create_stream();

        let block = read_block("tests/res/testnet_block_182018895.json");
        let aurora_blocks = stream.next_block(&block).await;
        assert_eq!(aurora_blocks.len(), 1);
        assert_eq!(aurora_blocks[0].height, 182018895);
        assert!(matches!(
            aurora_blocks[0].near_metadata,
            NearBlock::ExistingBlock(..)
        ));

        let expected_sender = near_account_to_evm_address(b"crocus.testnet");
        let expected_amount = Wei::zero();
        let expected_input =
            aurora_engine::engine::setup_deploy_erc20_input(&"aurora".parse().unwrap(), None);

        let aurora_block = aurora_blocks.first().unwrap();
        let deploy_erc20_token = aurora_block.transactions.iter().find(|tx| {
            tx.from == expected_sender
                && tx.to == None
                && tx.value == expected_amount
                && tx.input == expected_input
        });

        assert!(
            deploy_erc20_token.is_some(),
            "Expected deploy_erc20_token transaction not found in block 182018895"
        );
    }

    pub fn read_block(path: &str) -> NEARBlock {
        let data = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&data).unwrap_or_else(|e| {
            panic!("Failed to parse block from {}: {}", path, e);
        })
    }

    pub struct TestContext {
        chain_id: u64,
        engine_context: EngineContext,
        tx_tracker: TxHashTracker,
    }

    impl TestContext {
        pub fn new(db_dir: &tempfile::TempDir) -> Self {
            TestContextBuilder::new().build(db_dir)
        }

        pub fn new_with_args(
            db_dir: &tempfile::TempDir,
            account_id: AccountId,
            chain_id: u64,
        ) -> Self {
            let engine_path = db_dir.path().join("engine");
            let tracker_path = db_dir.path().join("tracker");
            crate::storage::init_storage(&engine_path, &account_id, chain_id);
            let engine_context = EngineContext::new(&engine_path, account_id, chain_id).unwrap();
            let tx_tracker = TxHashTracker::new(tracker_path, 0).unwrap();

            Self {
                chain_id,
                engine_context,
                tx_tracker,
            }
        }

        pub async fn init_with_snapshot(&mut self, snapshot_path: &str) {
            let json_snapshot: JsonSnapshot = {
                let json_snapshot_data = std::fs::read_to_string(snapshot_path).unwrap();
                serde_json::from_str(&json_snapshot_data).unwrap()
            };
            let storage = self.engine_context.storage.as_ref().write().await;
            initialize_engine_state(&storage, json_snapshot).unwrap();
        }

        pub fn create_stream(self) -> NearStream {
            NearStream::new(self.chain_id, None, self.engine_context, self.tx_tracker)
        }
    }

    pub struct TestContextBuilder {
        chain_id: u64,
        account_id: AccountId,
    }

    impl TestContextBuilder {
        pub fn new() -> Self {
            Self {
                chain_id: 1313161554,
                account_id: "aurora".parse().unwrap(),
            }
        }

        pub fn with_account_id(mut self, account_id: &str) -> Self {
            self.account_id = account_id.parse().unwrap();
            self
        }

        pub const fn with_chain_id(mut self, chain_id: u64) -> Self {
            self.chain_id = chain_id;
            self
        }

        pub fn build(self, db_dir: &tempfile::TempDir) -> TestContext {
            TestContext::new_with_args(db_dir, self.account_id, self.chain_id)
        }
    }

    impl Default for TestContextBuilder {
        fn default() -> Self {
            Self::new()
        }
    }
}
