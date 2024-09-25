use crate::legacy::decode_submit_result;
use crate::metrics::{record_metric, LATEST_BLOCK_PROCESSED};
use crate::utils::{as_h256, keccak256, TxMetadata};
use aurora_engine::engine::create_legacy_address;
use aurora_engine::parameters::{
    CallArgs, FunctionCallArgsV1, ResultLog, SubmitArgs, SubmitResult,
};
use aurora_engine_sdk::sha256;
use aurora_engine_sdk::types::near_account_to_evm_address;
use aurora_engine_transactions::{
    Error as ParseTransactionError, EthTransactionKind, NormalizedEthTransaction,
};
use aurora_engine_types::borsh::BorshDeserialize;
use aurora_engine_types::types::{Wei, WeiU256};
use aurora_engine_types::{H256, U256};
use aurora_refiner_types::aurora_block::{
    AdditionalSubmitArgs, AuroraBlock, AuroraTransaction, AuroraTransactionBuilder,
    AuroraTransactionBuilderError, CallArgsVersion, HashchainInputKind, HashchainMetadata,
    HashchainOutputKind, NearBlock, NearBlockHeader, NearTransaction,
};
use aurora_refiner_types::bloom::Bloom;
use aurora_refiner_types::near_block::{BlockView, ExecutionOutcomeWithReceipt, NEARBlock};
use aurora_refiner_types::near_primitives::hash::CryptoHash;
use aurora_refiner_types::near_primitives::types::{AccountId, BlockHeight};
use aurora_refiner_types::near_primitives::views::{
    ActionView, ExecutionStatusView, ReceiptEnumView,
};
use byteorder::{BigEndian, WriteBytesExt};
use engine_standalone_storage::sync::{
    types::TransactionKindTag, TransactionExecutionResult, TransactionIncludedOutcome,
};
use engine_standalone_storage::Storage;
use std::collections::{HashMap, HashSet};
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::io::Write;
use std::str::FromStr;
use triehash_ethereum::ordered_trie_root;

/// The least amount of gas any EVM transaction could spend is 21_000.
/// This corresponds to `G_transaction` from the Yellow Paper. This is
/// the amount of gas "paid for every transaction" (see Appendix G of the Yellow Paper).
const MIN_EVM_GAS: u64 = 21_000;

fn compute_block_hash_preimage(
    height: BlockHeight,
    engine_account_id: &str,
    chain_id: u64,
) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(25 + 8 + engine_account_id.len() + 8);
    let _ = buffer.write(&[0; 25]);
    let _ = buffer.write_u64::<BigEndian>(chain_id);
    let _ = buffer.write(engine_account_id.as_bytes());
    let _ = buffer.write_u64::<BigEndian>(height);

    buffer
}

fn compute_block_hash(height: BlockHeight, engine_account_id: &str, chain_id: u64) -> H256 {
    sha256(&compute_block_hash_preimage(
        height,
        engine_account_id,
        chain_id,
    ))
}

struct TxExtraData {
    transaction_hash: H256,
    /// Hash of the result of the transaction
    receipt_hash: H256,
}

pub struct Refiner {
    chain_id: u64,
    /// Account id of the engine contract on the chain
    engine_account_id: AccountId,
    /// Constant value of an empty merkle tree root
    empty_merkle_tree_root: H256,
    /// Last prev_state_root known (useful to compute state roots on skip blocks)
    /// Refiner can't start from a skip block so this field will be always set.
    /// Previous valid prev_state_root will be used for next
    prev_state_root: H256,
    /// Partial state used during the computation of a block.
    partial_state: PartialState,
}

/// Data that must be recomputed on every block
#[derive(Default)]
struct PartialState {
    /// Amount of gas spent on a block
    total_gas: u64,
    /// Estimate of the size (in bytes) of all the transactions in a block
    size: u64,
    /// Partial bloom filter
    bloom_filter: Bloom,
    /// List of all current transactions
    transactions: Vec<AuroraTransaction>,
    /// Transactions data used to build transactions and receipts merkle tree
    transactions_extra_data: Vec<TxExtraData>,
    /// List with all observed receipts. A Receipt can be seen multiple times, one per action
    seen_receipts: HashSet<CryptoHash>,
}

impl Refiner {
    pub fn new(chain_id: u64, engine_account_id: AccountId) -> Self {
        Self {
            chain_id,
            engine_account_id,
            empty_merkle_tree_root: H256::from(
                TryInto::<&[u8; 32]>::try_into(
                    &hex::decode(
                        "56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
                    )
                    .unwrap()[..],
                )
                .unwrap(),
            ),
            prev_state_root: H256::zero(),
            partial_state: Default::default(),
        }
    }
}

impl Refiner {
    pub fn on_block_skip(&mut self, height: u64, next_block: &NEARBlock) -> AuroraBlock {
        AuroraBlock {
            chain_id: self.chain_id,
            engine_account_id: self.engine_account_id.clone(),
            hash: compute_block_hash(height, self.engine_account_id.as_str(), self.chain_id),
            parent_hash: compute_block_hash(
                height - 1,
                self.engine_account_id.as_str(),
                self.chain_id,
            ),
            height,
            miner: near_account_to_evm_address(b""),
            timestamp: next_block.block.header.timestamp,
            gas_limit: u64::MAX,
            size: 0,
            gas_used: 0,
            transactions_root: self.empty_merkle_tree_root,
            receipts_root: self.empty_merkle_tree_root,
            transactions: vec![],
            near_metadata: NearBlock::SkipBlock,
            state_root: self.prev_state_root,
            logs_bloom: Default::default(),
        }
    }

    pub fn on_block_start(&mut self, block: &NEARBlock) {
        let NEARBlock { block, shards, .. } = &block;
        // Check if all chunks were parsed
        tracing::trace!(target: "block", "Processing block at height {}, hash={}", block.header.height, block.header.hash);
        if block.header.chunk_mask.len() != shards.len() {
            tracing::warn!(target: "block", "Not all shards are being tracked. Expected number of shards {}, found {}", block.header.chunk_mask.len(), shards.len());
            crate::metrics::MISSING_SHARDS.inc();
        }
    }

    #[allow(clippy::cognitive_complexity)]
    pub fn on_execution_outcome(
        &mut self,
        block: &NEARBlock,
        near_tx_hash: Option<CryptoHash>,
        execution_outcome: &ExecutionOutcomeWithReceipt,
        txs: &HashMap<H256, TransactionIncludedOutcome>,
        storage: &Storage,
    ) {
        let NEARBlock { block, .. } = &block;

        if self
            .partial_state
            .seen_receipts
            .insert(execution_outcome.receipt.receipt_id)
        {
            // Using recent version of borsh to serialize the receipt.
            // Include in the size of the block the size of this transaction.
            self.partial_state.size +=
                borsh::to_vec(&execution_outcome.receipt).unwrap().len() as u64;
        }

        match &execution_outcome.receipt.receipt {
            ReceiptEnumView::Action { actions, .. } => {
                crate::metrics::TRANSACTIONS.inc();

                // Receipts with multiple actions are atomic; they either entirely succeed or
                // there no state changes from any action. If the execution outcome is
                // a failure then we can skip the receipt (regardless of how many actions it has)
                if let ExecutionStatusView::Unknown | ExecutionStatusView::Failure(_) =
                    &execution_outcome.execution_outcome.outcome.status
                {
                    tracing::trace!(target: "transactions", "Failing NEAR Transaction at block: {}", block.header.hash);
                    return;
                }

                let num_actions = actions.len();

                // Create one transaction per action
                for (index, action) in actions.iter().enumerate() {
                    crate::metrics::TRANSACTIONS_ACTION.inc();

                    let near_metadata = NearTransaction {
                        action_index: index,
                        receipt_hash: execution_outcome.receipt.receipt_id,
                        transaction_hash: near_tx_hash,
                        hashchain_metadata: None, // Value filled during `build_transaction`
                    };

                    // The execution outcome only applies to the last action in the batch
                    let status = if index + 1 == num_actions {
                        Some(&execution_outcome.execution_outcome.outcome.status)
                    } else {
                        None
                    };

                    let virtual_receipt_id = build_virtual_receipt_id(
                        &execution_outcome.receipt.receipt_id,
                        index as u32,
                        num_actions as u32,
                    );

                    match build_transaction(
                        block,
                        action,
                        &execution_outcome.receipt.predecessor_id,
                        near_metadata,
                        status,
                        self.chain_id,
                        self.partial_state.transactions.len() as u32,
                        virtual_receipt_id,
                        txs,
                        storage,
                    ) {
                        Ok(tx) => {
                            let BuiltTransaction {
                                transaction,
                                transaction_hash,
                            } = tx;

                            let result_hash = sha256(transaction.output.as_slice());
                            tracing::trace!(target: "transactions", "New transaction: {}", transaction.hash);
                            self.partial_state.total_gas = self
                                .partial_state
                                .total_gas
                                .saturating_add(transaction.gas_used);
                            self.partial_state
                                .bloom_filter
                                .accrue_bloom(&transaction.logs_bloom);
                            self.partial_state.transactions.push(transaction);
                            self.partial_state
                                .transactions_extra_data
                                .push(TxExtraData {
                                    transaction_hash,
                                    receipt_hash: result_hash,
                                });
                        }
                        Err(RefinerError::FailNearTx) => {
                            tracing::trace!(target: "transactions", "Failing NEAR Transaction at block: {}", block.header.hash);
                        }
                        Err(err) => {
                            tracing::error!(target: "transactions", "Error while building transaction: {:?}. Block: {}", err, block.header.hash);
                            crate::metrics::ERROR_BUILDING_TRANSACTION.inc();
                        }
                    }
                }
            }
            // Ignore receipts of type Data
            ReceiptEnumView::Data { data_id, .. } => {
                crate::metrics::TRANSACTIONS_DATA.inc();
                tracing::warn!(target: "transactions",
                    "Ignore receipt data. Receipt Id: {} Data Id: {:?}",
                    execution_outcome.receipt.receipt_id,
                    data_id,
                )
            }
        }
    }

    pub fn on_block_end(&mut self, block: &NEARBlock) -> AuroraBlock {
        let NEARBlock { block, .. } = &block;

        // Compute near metadata
        let near_header = NearBlockHeader {
            near_hash: block.header.hash,
            near_parent_hash: block.header.prev_hash,
            author: block.author.clone(),
        };

        // Build transactions root
        let transactions_root = as_h256(
            ordered_trie_root(
                self.partial_state
                    .transactions_extra_data
                    .iter()
                    .map(|tx| tx.transaction_hash),
            )
            .as_bytes(),
        );
        let receipts_root = as_h256(
            ordered_trie_root(
                self.partial_state
                    .transactions_extra_data
                    .iter()
                    .map(|tx| tx.receipt_hash),
            )
            .as_bytes(),
        );

        self.prev_state_root = H256::from(block.header.prev_state_root.0);

        let aurora_block = AuroraBlock {
            chain_id: self.chain_id,
            engine_account_id: self.engine_account_id.clone(),
            hash: compute_block_hash(
                block.header.height,
                self.engine_account_id.as_str(),
                self.chain_id,
            ),
            parent_hash: compute_block_hash(
                block.header.height - 1,
                self.engine_account_id.as_str(),
                self.chain_id,
            ),
            height: block.header.height,
            miner: near_account_to_evm_address(block.author.as_bytes()),
            timestamp: block.header.timestamp,
            gas_limit: u64::MAX,
            state_root: self.prev_state_root,
            size: self.partial_state.size,
            gas_used: self.partial_state.total_gas,
            transactions_root,
            receipts_root,
            transactions: self.partial_state.transactions.drain(..).collect(),
            near_metadata: NearBlock::ExistingBlock(near_header),
            logs_bloom: self.partial_state.bloom_filter,
        };

        LATEST_BLOCK_PROCESSED.set(block.header.height as i64);

        // Reset the partial state
        self.partial_state = Default::default();

        aurora_block
    }
}

/// Receipt id of internal actions, when several actions are batched in the same receipt
fn build_virtual_receipt_id(
    receipt_id: &CryptoHash,
    action_index: u32,
    total_actions: u32,
) -> CryptoHash {
    if action_index + 1 == total_actions {
        *receipt_id
    } else {
        let mut bytes = [0u8; 36];
        bytes[0..32].copy_from_slice(receipt_id.0.as_slice());
        bytes[32..36].copy_from_slice(&action_index.to_be_bytes());
        CryptoHash(aurora_refiner_types::utils::keccak256(&bytes).0)
    }
}

struct BuiltTransaction {
    transaction: AuroraTransaction,
    transaction_hash: H256,
}

/// Given the raw `execution_status` from Near and `engine_outcome` from Borealis Engine,
/// try to create a single `SubmitResult` instance. This function also checks that the
/// two raw outcomes match in the case that they are both present.
fn normalize_output(
    receipt_id: &CryptoHash,
    tx_kind: TransactionKindTag,
    execution_status: Option<&ExecutionStatusView>,
    engine_outcome: Option<&TransactionIncludedOutcome>,
) -> Result<(SubmitResult, HashchainOutputKind, Vec<u8>), RefinerError> {
    let engine_output = engine_outcome
        .and_then(|x| x.maybe_result.as_ref().ok())
        .and_then(Option::as_ref)
        .map(|result| match result {
            TransactionExecutionResult::Submit(result) => match result {
                Ok(result) => result.clone(),
                Err(err) => SubmitResult::new(
                    aurora_engine::parameters::TransactionStatus::Revert(
                        format!("{:?}", err.kind).into_bytes(),
                    ),
                    err.gas_used,
                    Vec::new(),
                ),
            },
            TransactionExecutionResult::DeployErc20(address) => SubmitResult::new(
                aurora_engine::parameters::TransactionStatus::Succeed(address.as_bytes().to_vec()),
                MIN_EVM_GAS,
                Vec::new(),
            ),
            TransactionExecutionResult::Promise(p) => SubmitResult::new(
                aurora_engine::parameters::TransactionStatus::Succeed(
                    format!("{:?}", p).into_bytes(),
                ),
                MIN_EVM_GAS,
                Vec::new(),
            ),
        });

    let near_output = match execution_status {
        Some(ExecutionStatusView::Unknown | ExecutionStatusView::Failure(_)) => {
            // Regardless of anything else, if the transaction failed on Near then we report an error.
            crate::metrics::FAILING_NEAR_TRANSACTION.inc();
            tracing::debug!(
                "Failing NEAR transaction {}: {:?}",
                receipt_id,
                execution_status
            );
            return Err(RefinerError::FailNearTx);
        }
        Some(ExecutionStatusView::SuccessValue(result)) => {
            let bytes = result.clone();
            match tx_kind {
                TransactionKindTag::Submit
                | TransactionKindTag::Call
                | TransactionKindTag::Deploy
                | TransactionKindTag::SubmitWithArgs => {
                    // These transaction kinds should have a `SubmitResult` as an outcome
                    let (result, output_kind) = decode_submit_result(&bytes).unwrap_or_else(|_| {
                        // This is now considered a fatal error because we must know how
                        // to reproduce the Near output for all transactions
                        panic!(
                            "Submit Result format unknown for receipt {:?}. (FIX)",
                            receipt_id
                        );
                    });
                    Some((result, output_kind, bytes))
                }
                _ => {
                    // Everything else we'll just use the bytes directly as the output and
                    // set the other fields with default values.
                    Some((
                        SubmitResult::new(
                            aurora_engine::parameters::TransactionStatus::Succeed(bytes.clone()),
                            MIN_EVM_GAS,
                            Vec::new(),
                        ),
                        HashchainOutputKind::Explicit,
                        bytes,
                    ))
                }
            }
        }
        // In the case of `withdraw_wnear_to_router` we take the `SubmitResult` from the
        // Standalone Engine because the promise value is only returned to properly link
        // the execution of various XCC receipts, but the EVM computation still happened.
        // In terms of the hashchain, we still treat it as `HashchainOutputKind::None` because
        // `io.return_output` is never called since the promise is returned instead.
        Some(ExecutionStatusView::SuccessReceiptId(result))
            if tx_kind == TransactionKindTag::WithdrawWnearToRouter =>
        {
            let submit_result = engine_output.as_ref().cloned().unwrap_or_else(|| {
                let bytes = result.0.to_vec();
                SubmitResult::new(
                    aurora_engine::parameters::TransactionStatus::Succeed(bytes),
                    MIN_EVM_GAS,
                    Vec::new(),
                )
            });
            Some((submit_result, HashchainOutputKind::None, Vec::new()))
        }
        Some(ExecutionStatusView::SuccessReceiptId(result)) => {
            // No need to check the transaction kind in this case because transactions that
            // produce a SubmitResult as output do not produce a receipt id.
            let bytes = result.0.to_vec();
            Some((
                SubmitResult::new(
                    aurora_engine::parameters::TransactionStatus::Succeed(bytes),
                    MIN_EVM_GAS,
                    Vec::new(),
                ),
                HashchainOutputKind::None,
                Vec::new(),
            ))
        }
        None => None,
    };

    match (near_output, engine_output) {
        (Some(near_output), Some(engine_output)) => {
            // In the case of ft_on_transfer, the bridge may mint new ERC-20 tokens.
            // However, the NEP-141 protocol must still be followed, therefore the on-chain
            // output cannot be a `SubmitResult`. But the Standalone Engine can still capture
            // the `SubmitResult` from the execution. Hence, in this case we combine the
            // Standalone's result with the the on-chain output to get a complete picture.
            if tx_kind == TransactionKindTag::FtOnTransfer {
                return Ok((engine_output, near_output.1, near_output.2));
            }
            // We have a result from both sources, so we should compare them to
            // make sure they match. Log a warning and use the Near output if they don't.
            if near_output.0 != engine_output {
                tracing::warn!("Mismatch between Near and Engine outputs. The internal Engine instance may not have the correct state.");
            }
            Ok(near_output)
        }
        (None, Some(output)) => {
            // No Near outcome to rely on, so we simply have to trust the Borealis Engine
            // outcome without validation. This case happens for actions in a batch except
            // for the last one (Near only records the outcome of the last action in a batch).
            let tag = (&output.status).into();
            let bytes = borsh::to_vec(&output).expect("Must be able to serialize Result");
            Ok((output, HashchainOutputKind::SubmitResultV7(tag), bytes))
        }
        (Some(output), None) => {
            // No engine outcome to use, so can only rely on the NEAR output.
            // This case could arise if the last action in a batch is an aurora-engine call
            // where the Borealis Engine does not record an outcome (e.g. `ft_on_transfer`).
            Ok(output)
        }
        (None, None) => {
            // if there is no outcome from either source then use a default value
            Ok((
                SubmitResult::new(
                    aurora_engine::parameters::TransactionStatus::Succeed(Vec::new()),
                    MIN_EVM_GAS,
                    Vec::new(),
                ),
                HashchainOutputKind::None,
                Vec::new(),
            ))
        }
    }
}

fn fill_hashchain_metadata(
    tx: AuroraTransactionBuilder,
    mut near_metadata: NearTransaction,
    method_name: String,
    raw_input: &[u8],
    raw_output: &[u8],
    input_kind: HashchainInputKind,
    output_kind: HashchainOutputKind,
) -> AuroraTransactionBuilder {
    let intrinsic_hash =
        crate::hashchain::compute_tx_hashchain(&method_name, raw_input, raw_output);
    let hashchain_metadata = HashchainMetadata {
        method_name,
        input: input_kind,
        output: output_kind,
        intrinsic_hash: CryptoHash(intrinsic_hash.0),
    };
    near_metadata.hashchain_metadata = Some(hashchain_metadata);
    tx.near_metadata(near_metadata)
}

#[allow(clippy::too_many_arguments)]
fn build_transaction(
    near_block: &BlockView,
    action: &ActionView,
    predecessor_id: &AccountId,
    near_metadata: NearTransaction,
    execution_status: Option<&ExecutionStatusView>,
    chain_id: u64,
    transaction_index: u32,
    virtual_receipt_id: CryptoHash,
    txs: &HashMap<H256, TransactionIncludedOutcome>,
    storage: &Storage,
) -> Result<BuiltTransaction, RefinerError> {
    let mut bloom = Bloom::default();

    let hash;
    let receipt_id = near_metadata.receipt_hash;
    let account_id = storage.get_engine_account_id();
    let engine_account_id = account_id
        .as_ref()
        .map(|id| id.as_ref())
        .unwrap_or("aurora");

    let mut tx = AuroraTransactionBuilder::default()
        .block_hash(compute_block_hash(
            near_block.header.height,
            engine_account_id,
            chain_id,
        ))
        .block_height(near_block.header.height)
        .chain_id(chain_id)
        .transaction_index(transaction_index)
        .gas_price(U256::zero());

    // Hash used to build transactions merkle tree
    let mut transaction_hash = H256::zero();

    match action {
        ActionView::FunctionCall {
            method_name, args, ..
        } => {
            let raw_input = args.to_vec();

            transaction_hash = sha256(raw_input.as_slice());

            let raw_tx_kind: TransactionKindTag =
                TransactionKindTag::from_str(method_name.as_str())
                    .unwrap_or(TransactionKindTag::Unknown);

            record_metric(&raw_tx_kind);

            if TransactionKindTag::Unknown == raw_tx_kind {
                tracing::warn!("Unknown method: {}", method_name);
            }

            tx = match raw_tx_kind {
                TransactionKindTag::SubmitWithArgs => {
                    let input_kind = match SubmitArgs::try_from_slice(&raw_input) {
                        Ok(args) => {
                            let bytes = args.tx_data;

                            let tx_metadata = TxMetadata::try_from(bytes.as_slice())
                                .map_err(RefinerError::ParseMetadata)?;

                            let mut eth_tx: NormalizedEthTransaction =
                                EthTransactionKind::try_from(bytes.as_slice())
                                    .and_then(TryFrom::try_from)
                                    .map_err(RefinerError::ParseTransaction)?;

                            if let Some(gas_price) = args.max_gas_price {
                                let gas_price: U256 = gas_price.into();
                                eth_tx.max_fee_per_gas = eth_tx.max_fee_per_gas.min(gas_price);
                                eth_tx.max_priority_fee_per_gas =
                                    eth_tx.max_priority_fee_per_gas.min(gas_price);
                            }

                            hash = keccak256(bytes.as_slice()); // https://ethereum.stackexchange.com/a/46579/45323

                            tx = tx
                                .hash(hash)
                                .from(eth_tx.address)
                                .nonce(aurora_refiner_types::utils::saturating_cast(eth_tx.nonce))
                                .gas_limit(aurora_refiner_types::utils::saturating_cast(
                                    eth_tx.gas_limit,
                                ))
                                .gas_price(eth_tx.max_fee_per_gas)
                                .max_priority_fee_per_gas(eth_tx.max_priority_fee_per_gas)
                                .max_fee_per_gas(eth_tx.max_fee_per_gas)
                                .value(eth_tx.value)
                                .input(eth_tx.data)
                                .access_list(eth_tx.access_list)
                                .tx_type(tx_metadata.tx_type)
                                .v(tx_metadata.v)
                                .r(tx_metadata.r)
                                .s(tx_metadata.s);

                            tx = if eth_tx.to.is_some() {
                                tx.to(eth_tx.to).contract_address(None)
                            } else {
                                let contract_address =
                                    create_legacy_address(&eth_tx.address, &eth_tx.nonce);
                                tx.to(None).contract_address(Some(contract_address))
                            };
                            HashchainInputKind::SubmitWithArgs(AdditionalSubmitArgs {
                                max_gas_price: args.max_gas_price,
                                gas_token_address: args.gas_token_address,
                            })
                        }
                        Err(_) => {
                            hash = virtual_receipt_id.0.into();
                            let from_address =
                                near_account_to_evm_address(predecessor_id.as_bytes());
                            tx = tx.hash(hash).from(from_address);
                            tx = fill_tx(tx, raw_input.clone());
                            HashchainInputKind::Explicit
                        }
                    };

                    let (result, output_kind, raw_output) = normalize_output(
                        &receipt_id,
                        raw_tx_kind,
                        execution_status,
                        txs.get(&hash),
                    )?;
                    tx = fill_hashchain_metadata(
                        tx,
                        near_metadata,
                        method_name.clone(),
                        &raw_input,
                        &raw_output,
                        input_kind,
                        output_kind,
                    );
                    fill_with_submit_result(tx, result, &mut bloom)
                }
                TransactionKindTag::Submit => {
                    let tx_metadata = TxMetadata::try_from(raw_input.as_slice())
                        .map_err(RefinerError::ParseMetadata)?;

                    let eth_tx: NormalizedEthTransaction =
                        EthTransactionKind::try_from(raw_input.as_slice())
                            .and_then(TryFrom::try_from)
                            .map_err(RefinerError::ParseTransaction)?;

                    hash = keccak256(raw_input.as_slice()); // https://ethereum.stackexchange.com/a/46579/45323

                    tx = tx
                        .hash(hash)
                        .from(eth_tx.address)
                        .nonce(aurora_refiner_types::utils::saturating_cast(eth_tx.nonce))
                        .gas_limit(aurora_refiner_types::utils::saturating_cast(
                            eth_tx.gas_limit,
                        ))
                        .gas_price(eth_tx.max_fee_per_gas)
                        .max_priority_fee_per_gas(eth_tx.max_priority_fee_per_gas)
                        .max_fee_per_gas(eth_tx.max_fee_per_gas)
                        .value(eth_tx.value)
                        .input(eth_tx.data)
                        .access_list(eth_tx.access_list)
                        .tx_type(tx_metadata.tx_type)
                        .v(tx_metadata.v)
                        .r(tx_metadata.r)
                        .s(tx_metadata.s);

                    tx = if eth_tx.to.is_some() {
                        tx.to(eth_tx.to).contract_address(None)
                    } else {
                        let contract_address =
                            create_legacy_address(&eth_tx.address, &eth_tx.nonce);
                        tx.to(None).contract_address(Some(contract_address))
                    };

                    let (result, output_kind, raw_output) = normalize_output(
                        &receipt_id,
                        raw_tx_kind,
                        execution_status,
                        txs.get(&hash),
                    )?;
                    tx = fill_hashchain_metadata(
                        tx,
                        near_metadata,
                        method_name.clone(),
                        &raw_input,
                        &raw_output,
                        HashchainInputKind::Rlp,
                        output_kind,
                    );
                    fill_with_submit_result(tx, result, &mut bloom)
                }
                TransactionKindTag::Call => {
                    hash = virtual_receipt_id.0.into();
                    let from_address = near_account_to_evm_address(predecessor_id.as_bytes());

                    tx = tx.hash(hash).from(from_address);

                    let input_kind = if let Some(call_args) = CallArgs::deserialize(&raw_input) {
                        let (to_address, value, input, input_kind) = match call_args {
                            CallArgs::V2(args) => (
                                args.contract,
                                args.value,
                                args.input,
                                HashchainInputKind::CallArgs(CallArgsVersion::V2),
                            ),
                            CallArgs::V1(args) => {
                                let input_kind =
                                    if FunctionCallArgsV1::try_from_slice(&raw_input).is_err() {
                                        HashchainInputKind::CallArgs(CallArgsVersion::V1)
                                    } else {
                                        HashchainInputKind::CallArgsLegacy
                                    };
                                (args.contract, WeiU256::default(), args.input, input_kind)
                            }
                        };

                        let nonce = storage
                            .with_engine_access(
                                near_block.header.height,
                                transaction_index.try_into().unwrap_or(u16::MAX),
                                &[],
                                |io| aurora_engine::engine::get_nonce(&io, &from_address),
                            )
                            .result;

                        tx = tx
                            .to(Some(to_address))
                            .nonce(aurora_refiner_types::utils::saturating_cast(nonce))
                            .gas_limit(u64::MAX)
                            .max_priority_fee_per_gas(U256::zero())
                            .max_fee_per_gas(U256::zero())
                            .value(value.into())
                            .input(input)
                            .access_list(vec![])
                            .tx_type(0xff)
                            .contract_address(None)
                            .v(0)
                            .r(U256::zero())
                            .s(U256::zero());

                        input_kind
                    } else {
                        tx = fill_tx(tx, raw_input.clone());
                        HashchainInputKind::Explicit
                    };

                    let (result, output_kind, raw_output) = normalize_output(
                        &receipt_id,
                        raw_tx_kind,
                        execution_status,
                        txs.get(&hash),
                    )?;
                    tx = fill_hashchain_metadata(
                        tx,
                        near_metadata,
                        method_name.clone(),
                        &raw_input,
                        &raw_output,
                        input_kind,
                        output_kind,
                    );
                    fill_with_submit_result(tx, result, &mut bloom)
                }
                TransactionKindTag::Deploy | TransactionKindTag::DeployErc20 => {
                    hash = virtual_receipt_id.0.into();
                    let from_address = near_account_to_evm_address(predecessor_id.as_bytes());
                    let nonce = storage
                        .with_engine_access(
                            near_block.header.height,
                            transaction_index.try_into().unwrap_or(u16::MAX),
                            &[],
                            |io| aurora_engine::engine::get_nonce(&io, &from_address),
                        )
                        .result;
                    let contract_address = create_legacy_address(&from_address, &nonce);

                    tx = tx.hash(hash).from(from_address);

                    tx = tx
                        .to(None)
                        .nonce(aurora_refiner_types::utils::saturating_cast(nonce))
                        .gas_limit(u64::MAX)
                        .max_priority_fee_per_gas(U256::zero())
                        .max_fee_per_gas(U256::zero())
                        .value(Wei::zero())
                        .input(raw_input.clone())
                        .access_list(vec![])
                        .tx_type(0xff)
                        .contract_address(Some(contract_address))
                        .v(0)
                        .r(U256::zero())
                        .s(U256::zero());

                    let (result, output_kind, raw_output) = normalize_output(
                        &receipt_id,
                        raw_tx_kind,
                        execution_status,
                        txs.get(&hash),
                    )?;
                    tx = fill_hashchain_metadata(
                        tx,
                        near_metadata,
                        method_name.clone(),
                        &raw_input,
                        &raw_output,
                        HashchainInputKind::Explicit,
                        output_kind,
                    );
                    fill_with_submit_result(tx, result, &mut bloom)
                }
                _ => {
                    hash = virtual_receipt_id.0.into();
                    tx = tx
                        .hash(hash)
                        .from(near_account_to_evm_address(predecessor_id.as_bytes()));
                    let (result, output_kind, raw_output) = normalize_output(
                        &receipt_id,
                        raw_tx_kind,
                        execution_status,
                        txs.get(&hash),
                    )?;
                    tx = fill_hashchain_metadata(
                        tx,
                        near_metadata,
                        method_name.clone(),
                        &raw_input,
                        &raw_output,
                        HashchainInputKind::Explicit,
                        output_kind,
                    );
                    tx = fill_with_submit_result(tx, result, &mut bloom);
                    fill_tx(tx, raw_input)
                }
            }
        }
        action => {
            let input = borsh::to_vec(&action).unwrap();

            tx = tx
                .hash(virtual_receipt_id.0.into())
                .from(near_account_to_evm_address(predecessor_id.as_bytes()))
                .to(Some(near_account_to_evm_address(
                    engine_account_id.as_bytes(),
                )))
                .contract_address(None)
                .nonce(0)
                .gas_limit(0)
                .gas_used(0)
                .max_priority_fee_per_gas(U256::zero())
                .max_fee_per_gas(U256::zero())
                .value(Wei::new(U256::zero()))
                .input(input)
                .access_list(vec![])
                .tx_type(0xff)
                .logs(vec![])
                .v(0)
                .r(U256::zero())
                .s(U256::zero())
                // Type for NEAR custom transactions
                .tx_type(0xfe)
                .access_list(vec![])
                .near_metadata(near_metadata);

            match execution_status {
                None | Some(ExecutionStatusView::Unknown) => {
                    tx = tx.output(vec![]).status(false);
                }
                Some(ExecutionStatusView::Failure(err)) => {
                    tx = tx.output(borsh::to_vec(err).unwrap()).status(false);
                }
                Some(ExecutionStatusView::SuccessValue(value)) => {
                    tx = tx.output(value.clone()).status(true);
                }
                Some(ExecutionStatusView::SuccessReceiptId(data)) => {
                    tx = tx.output(data.0.to_vec()).status(true);
                }
            }
        }
    }

    tx = tx.logs_bloom(bloom);

    Ok(BuiltTransaction {
        transaction: tx.build().map_err(RefinerError::BuilderError)?,
        transaction_hash,
    })
}

fn fill_with_submit_result(
    mut tx: AuroraTransactionBuilder,
    result: SubmitResult,
    blooms: &mut Bloom,
) -> AuroraTransactionBuilder {
    for log in result.logs.iter() {
        blooms.accrue_bloom(&get_log_blooms(log));
    }

    tx = tx.gas_used(result.gas_used).logs(result.logs);
    match result.status {
        aurora_engine::parameters::TransactionStatus::Succeed(output) => {
            tx.status(true).output(output)
        }
        aurora_engine::parameters::TransactionStatus::Revert(output) => {
            tx.status(false).output(output)
        }
        _ => tx.status(false).output(vec![]),
    }
}

fn fill_tx(tx: AuroraTransactionBuilder, input: Vec<u8>) -> AuroraTransactionBuilder {
    tx.to(None)
        .nonce(0)
        .gas_limit(0)
        .max_priority_fee_per_gas(U256::zero())
        .max_fee_per_gas(U256::zero())
        .value(Wei::new(U256::zero()))
        .input(input)
        .access_list(vec![])
        .tx_type(0xff)
        .contract_address(None)
        .v(0)
        .r(U256::zero())
        .s(U256::zero())
}

enum RefinerError {
    /// Fail building transaction. Most likely some arguments missing
    BuilderError(AuroraTransactionBuilderError),
    /// Failed to parse Ethereum Transaction
    ParseTransaction(ParseTransactionError),
    /// Failed to parse metadata from Ethereum Transaction
    ParseMetadata(rlp::DecoderError),
    /// NEAR transaction failed
    FailNearTx,
}

impl fmt::Debug for RefinerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BuilderError(err) => write!(f, "BuilderError: {:?}", err),
            Self::ParseTransaction(err) => write!(f, "ParseTransaction: {:?}", err),
            Self::ParseMetadata(err) => write!(f, "ParseMetadata: {:?}", err),
            Self::FailNearTx => write!(f, "FailNearTx"),
        }
    }
}

fn get_log_blooms(log: &ResultLog) -> Bloom {
    let mut bloom = Bloom::default();
    bloom.accrue(log.address.as_bytes());
    for topic in log.topics.iter() {
        bloom.accrue(&topic[..]);
    }
    bloom
}

#[cfg(test)]
mod tests {
    use super::{compute_block_hash, compute_block_hash_preimage};

    #[test]
    fn test_block_hash_preimage() {
        assert_eq!(
            compute_block_hash_preimage(62482103, "aurora", 1313161554),
            vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 78, 69, 65, 82, 97, 117, 114, 111, 114, 97, 0, 0, 0, 0, 3, 185, 102, 183,
            ]
        );
    }

    #[test]
    fn test_block_hash() {
        // Example of block: https://explorer.mainnet.aurora.dev/block/62482103/transactions
        assert_eq!(
            hex::encode(compute_block_hash(62482103, "aurora", 1313161554).as_bytes()),
            "97ccface51e97c896591c88ecb8106c4f48816493e1f7b1172245fb333a0e782"
        );
    }
}
