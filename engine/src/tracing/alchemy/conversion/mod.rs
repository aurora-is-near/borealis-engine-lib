use crate::tracing::alchemy::{
    AlchemyCallAction, AlchemyCallKind, AlchemyCallResult, AlchemyCallTrace, AlchemyCallType,
    AlchemyCreateAction, AlchemyCreateResult, AlchemyCreateTrace, AlchemyCreateType,
    AlchemySuicideAction, AlchemySuicideResult, AlchemySuicideTrace, AlchemySuicideType,
    AlchemyTrace,
};
use aurora_engine_types::{types::Address, H256, U256};
use engine_standalone_tracing::types::call_tracer::{CallFrame, CallType};
use std::collections::VecDeque;

mod sum_trie;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionContext {
    block_hash: H256,
    block_number: u64,
    transaction_hash: H256,
    transaction_position: u32,
}

pub fn from_call_frame(ctx: TransactionContext, trace: CallFrame) -> Vec<AlchemyTrace> {
    let mut result = Vec::new();
    let mut traversal_queue = VecDeque::new();
    let mut subtraces = sum_trie::SumTrie::default();

    traversal_queue.push_back((trace, Vec::new()));

    while let Some((frame, trace_address)) = traversal_queue.pop_front() {
        let (frame, subcalls) = extract_subcalls(frame);
        subtraces.insert(&trace_address, subcalls.len());

        for (i, child) in subcalls.into_iter().enumerate() {
            // Unwrap is safe since there will never be more than u32::MAX subcalls in a real EVM transaction.
            let i = u32::try_from(i).unwrap();
            let child_address = {
                let mut buf = trace_address.clone();
                buf.push(i);
                buf
            };
            traversal_queue.push_back((child, child_address));
        }

        let alchemy_trace = inner_convert_call_frame(
            frame,
            ctx.block_hash,
            ctx.block_number,
            ctx.transaction_hash,
            ctx.transaction_position,
            0, // subtraces values fixed at the end
            trace_address,
        );
        result.push(alchemy_trace);
    }

    // Set subtraces values. This is done at the end because we don't know how many total
    // subcalls there are until we finish traversing the whole structure once.
    for (count, alchemy_trace) in subtraces.bf_traverse().into_iter().zip(result.iter_mut()) {
        let count = u32::try_from(count).unwrap();
        match alchemy_trace {
            AlchemyTrace::Call(t) => t.subtraces = count,
            AlchemyTrace::Create(t) => t.subtraces = count,
            AlchemyTrace::Suicide(t) => t.subtraces = count,
        }
    }

    result
}

fn inner_convert_call_frame(
    trace: FrameLocalData,
    block_hash: H256,
    block_number: u64,
    transaction_hash: H256,
    transaction_position: u32,
    subtraces: u32,
    trace_address: Vec<u32>,
) -> AlchemyTrace {
    match &trace.call_type {
        CallType::Call | CallType::StaticCall | CallType::DelegateCall | CallType::CallCode => {
            let typ = AlchemyCallType;
            let action = AlchemyCallAction {
                from: trace.from,
                call_type: match trace.call_type {
                    CallType::Call => AlchemyCallKind::Call,
                    CallType::StaticCall => AlchemyCallKind::StaticCall,
                    CallType::DelegateCall => AlchemyCallKind::DelegateCall,
                    CallType::CallCode => AlchemyCallKind::CallCode,
                    _ => unreachable!(),
                },
                gas: trace.gas,
                input: trace.input,
                to: trace.to.unwrap_or_default(),
                value: trace.value,
            };
            let result = AlchemyCallResult {
                gas_used: trace.gas_used,
                output: trace.output,
            };
            let value = AlchemyCallTrace {
                action,
                block_hash,
                block_number,
                error: trace.error,
                result,
                subtraces,
                trace_address,
                transaction_hash,
                transaction_position,
                typ,
            };
            AlchemyTrace::Call(value)
        }
        CallType::Create | CallType::Create2 => {
            let typ = AlchemyCreateType;
            let action = AlchemyCreateAction {
                from: trace.from,
                gas: trace.gas,
                init: trace.input,
                value: trace.value,
            };
            let result = AlchemyCreateResult {
                address: trace.to.unwrap_or_default(),
                code: trace.output,
                gas_used: trace.gas_used,
            };
            let value = AlchemyCreateTrace {
                action,
                block_hash,
                block_number,
                error: trace.error,
                result,
                subtraces,
                trace_address,
                transaction_hash,
                transaction_position,
                typ,
            };
            AlchemyTrace::Create(value)
        }
        CallType::SelfDestruct => {
            let typ = AlchemySuicideType;
            let action = AlchemySuicideAction {
                address: trace.from,
                refund_address: trace.to.unwrap_or_default(),
                balance: trace.value,
            };
            let result = AlchemySuicideResult;
            let value = AlchemySuicideTrace {
                action,
                block_hash,
                block_number,
                error: trace.error,
                result,
                subtraces,
                trace_address,
                transaction_hash,
                transaction_position,
                typ,
            };
            AlchemyTrace::Suicide(value)
        }
    }
}

struct FrameLocalData {
    call_type: CallType,
    from: Address,
    to: Option<Address>,
    value: U256,
    gas: u64,
    gas_used: u64,
    input: Vec<u8>,
    output: Vec<u8>,
    error: Option<String>,
}

fn extract_subcalls(frame: CallFrame) -> (FrameLocalData, Vec<CallFrame>) {
    let local = FrameLocalData {
        call_type: frame.call_type,
        from: frame.from,
        to: frame.to,
        value: frame.value,
        gas: frame.gas,
        gas_used: frame.gas_used,
        input: frame.input,
        output: frame.output,
        error: frame.error,
    };
    (local, frame.calls)
}
