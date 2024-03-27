//! This module contains functions for computing the hashchain value of Aurora Blocks.
//! See [AIP-008](https://github.com/aurora-is-near/AIPs/pull/8) for details.

use aurora_engine::parameters::{
    CallArgs, FunctionCallArgsV1, FunctionCallArgsV2, SubmitArgs, SubmitResult, TransactionStatus,
};
use aurora_engine_hashchain::merkle::StreamCompactMerkleTree;
use aurora_engine_transactions::{eip_1559, eip_2930, legacy, EthTransactionKind};
use aurora_engine_types::{types::u256_to_arr, H256};
use aurora_refiner_types::aurora_block::{
    AuroraBlock, AuroraTransaction, CallArgsVersion, HashchainInputKind, HashchainOutputKind,
    ResultStatusTag,
};
use std::borrow::Cow;

const MUST_BORSH_SERIALIZE: &str = "Must borsh serialize";

pub fn compute_hashchain(
    previous_hashchain: H256,
    block: &AuroraBlock,
) -> Result<H256, ValidationError> {
    let chain_id = u256_to_arr(&block.chain_id.into());

    let mut merkle = StreamCompactMerkleTree::new();
    let tx_hashes = block
        .transactions
        .iter()
        .filter_map(|tx| validate_tx_hashchain(tx).transpose());
    for hash in tx_hashes {
        let hash = hash?;
        merkle.add(hash.0);
    }
    let txs_hash = merkle.compute_hash();

    let data = [
        chain_id.as_slice(),
        block.engine_account_id.as_bytes(),
        &block.height.to_be_bytes(),
        &previous_hashchain.0,
        &txs_hash,
        block.logs_bloom.as_bytes(),
    ]
    .concat();

    Ok(aurora_engine_sdk::keccak(&data))
}

pub fn validate_tx_hashchain(
    transaction: &AuroraTransaction,
) -> Result<Option<H256>, ValidationError> {
    let hashchain_metadata = match &transaction.near_metadata.hashchain_metadata {
        Some(x) => x,
        None => return Ok(None),
    };

    let input = match &hashchain_metadata.input {
        HashchainInputKind::Rlp => Cow::Owned(rlp_encode(transaction)?),
        HashchainInputKind::CallArgsLegacy => {
            let call_args = FunctionCallArgsV1 {
                contract: transaction.to.ok_or(ValidationError::MissingToInCallTx)?,
                input: transaction.input.clone(),
            };
            Cow::Owned(borsh::to_vec(&call_args).expect(MUST_BORSH_SERIALIZE))
        }
        HashchainInputKind::CallArgs(CallArgsVersion::V1) => {
            let call_args = FunctionCallArgsV1 {
                contract: transaction.to.ok_or(ValidationError::MissingToInCallTx)?,
                input: transaction.input.clone(),
            };
            let call_args = CallArgs::V1(call_args);
            Cow::Owned(borsh::to_vec(&call_args).expect(MUST_BORSH_SERIALIZE))
        }
        HashchainInputKind::CallArgs(CallArgsVersion::V2) => {
            let call_args = FunctionCallArgsV2 {
                contract: transaction.to.ok_or(ValidationError::MissingToInCallTx)?,
                input: transaction.input.clone(),
                value: transaction.value.to_bytes(),
            };
            let call_args = CallArgs::V2(call_args);
            Cow::Owned(borsh::to_vec(&call_args).expect(MUST_BORSH_SERIALIZE))
        }
        HashchainInputKind::SubmitWithArgs(args) => {
            let submit_args = SubmitArgs {
                tx_data: rlp_encode(transaction)?,
                max_gas_price: args.max_gas_price,
                gas_token_address: args.gas_token_address,
            };
            Cow::Owned(borsh::to_vec(&submit_args).expect(MUST_BORSH_SERIALIZE))
        }
        HashchainInputKind::Explicit => Cow::Borrowed(transaction.input.as_slice()),
    };

    let output = match &hashchain_metadata.output {
        HashchainOutputKind::SubmitResultLegacyV1(tag) => {
            let status = tag_to_status(tag, transaction);
            let result = crate::legacy::SubmitResultLegacyV1 {
                status,
                gas_used: transaction.gas_used,
                logs: transaction.logs.clone(),
            };
            Cow::Owned(borsh::to_vec(&result).expect(MUST_BORSH_SERIALIZE))
        }
        HashchainOutputKind::SubmitResultLegacyV2(tag) => {
            let status = tag_to_status(tag, transaction);
            let result = crate::legacy::SubmitResultLegacyV2 {
                status,
                gas_used: transaction.gas_used,
                logs: crate::legacy::to_v1_logs(&transaction.logs),
            };
            Cow::Owned(borsh::to_vec(&result).expect(MUST_BORSH_SERIALIZE))
        }
        HashchainOutputKind::SubmitResultLegacyV3 => {
            let result = crate::legacy::SubmitResultLegacyV3 {
                status: transaction.status,
                gas_used: transaction.gas_used,
                result: transaction.output.clone(),
                logs: crate::legacy::to_v1_logs(&transaction.logs),
            };
            Cow::Owned(borsh::to_vec(&result).expect(MUST_BORSH_SERIALIZE))
        }
        HashchainOutputKind::SubmitResultV7(tag) => {
            let status = tag_to_status(tag, transaction);
            let result = SubmitResult::new(status, transaction.gas_used, transaction.logs.clone());
            Cow::Owned(borsh::to_vec(&result).expect(MUST_BORSH_SERIALIZE))
        }
        HashchainOutputKind::Explicit => Cow::Borrowed(transaction.output.as_slice()),
        HashchainOutputKind::None => Cow::<'_, [u8]>::Borrowed(&[]),
    };

    let computed_hashchain = compute_tx_hashchain(
        &hashchain_metadata.method_name,
        input.as_ref(),
        output.as_ref(),
    );

    if computed_hashchain.0 != hashchain_metadata.intrinsic_hash.0 {
        Err(ValidationError::IncorrectTxHash)
    } else {
        Ok(Some(computed_hashchain))
    }
}

pub fn compute_tx_hashchain(method_name: &str, input: &[u8], output: &[u8]) -> H256 {
    fn as_u32(x: usize) -> u32 {
        x.try_into().unwrap_or(u32::MAX)
    }

    let data = [
        &as_u32(method_name.len()).to_be_bytes(),
        method_name.as_bytes(),
        &as_u32(input.len()).to_be_bytes(),
        input,
        &as_u32(output.len()).to_be_bytes(),
        output,
    ]
    .concat();

    aurora_engine_sdk::keccak(&data)
}

fn tag_to_status(tag: &ResultStatusTag, transaction: &AuroraTransaction) -> TransactionStatus {
    match tag {
        ResultStatusTag::Success => TransactionStatus::Succeed(transaction.output.clone()),
        ResultStatusTag::Revert => TransactionStatus::Revert(transaction.output.clone()),
        ResultStatusTag::OutOfGas => TransactionStatus::OutOfGas,
        ResultStatusTag::OutOfFund => TransactionStatus::OutOfFund,
        ResultStatusTag::OutOfOffset => TransactionStatus::OutOfOffset,
        ResultStatusTag::CallTooDeep => TransactionStatus::CallTooDeep,
    }
}

fn rlp_encode(transaction: &AuroraTransaction) -> Result<Vec<u8>, ValidationError> {
    match transaction.tx_type {
        0 => {
            let tx = legacy::LegacyEthSignedTransaction {
                transaction: legacy::TransactionLegacy {
                    nonce: transaction.nonce.into(),
                    gas_price: transaction.gas_price,
                    gas_limit: transaction.gas_limit.into(),
                    to: transaction.to,
                    value: transaction.value,
                    data: transaction.input.clone(),
                },
                v: transaction.v,
                r: transaction.r,
                s: transaction.s,
            };
            let bytes = (&EthTransactionKind::Legacy(tx)).into();
            Ok(bytes)
        }
        eip_1559::TYPE_BYTE => {
            let tx = eip_1559::SignedTransaction1559 {
                transaction: eip_1559::Transaction1559 {
                    chain_id: transaction.chain_id,
                    nonce: transaction.nonce.into(),
                    max_priority_fee_per_gas: transaction.max_priority_fee_per_gas,
                    max_fee_per_gas: transaction.max_fee_per_gas,
                    gas_limit: transaction.gas_limit.into(),
                    to: transaction.to,
                    value: transaction.value,
                    data: transaction.input.clone(),
                    access_list: transaction.access_list.clone(),
                },
                parity: transaction.v as u8,
                r: transaction.r,
                s: transaction.s,
            };
            let bytes = (&EthTransactionKind::Eip1559(tx)).into();
            Ok(bytes)
        }
        eip_2930::TYPE_BYTE => {
            let tx = eip_2930::SignedTransaction2930 {
                transaction: eip_2930::Transaction2930 {
                    chain_id: transaction.chain_id,
                    nonce: transaction.nonce.into(),
                    gas_price: transaction.gas_price,
                    gas_limit: transaction.gas_limit.into(),
                    to: transaction.to,
                    value: transaction.value,
                    data: transaction.input.clone(),
                    access_list: transaction.access_list.clone(),
                },
                parity: transaction.v as u8,
                r: transaction.r,
                s: transaction.s,
            };
            let bytes = (&EthTransactionKind::Eip2930(tx)).into();
            Ok(bytes)
        }
        _ => Err(ValidationError::UnknownEthTxType),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    IncorrectTxHash,
    UnknownEthTxType,
    MissingToInCallTx,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::near_stream::tests::{read_block, TestContext};

    // All transactions in test blocks should pass `validate_tx_hashchain`.
    // I.e. the method of reproducing the original Near input and output
    // should be correct.
    #[tokio::test]
    async fn test_compute_hashchain() {
        let db_dir = tempfile::tempdir().unwrap();
        let test_blocks = [
            "tests/res/block-70834059.json",
            "tests/res/block-70834061.json",
            "tests/res/block-89402026.json",
            "tests/res/block-81206675.json",
            // This block contains an EIP-2930 submit transaction
            "tests/res/block-55905793.json",
        ];

        for file in test_blocks {
            let near_block = read_block(file);
            let ctx = TestContext::new(&db_dir);
            let mut stream = ctx.create_stream();
            let aurora_blocks = stream.next_block(&near_block).await;

            for block in aurora_blocks {
                assert!(compute_hashchain(H256::default(), &block).is_ok());
            }
        }
    }
}
