use crate::tracing::alchemy::{conversion, AlchemyCallTrace, AlchemyTrace};
use aurora_engine_types::{types::Address, U256};
use engine_standalone_tracing::types::call_tracer::{CallFrame, CallType};
use std::path::Path;

#[test]
fn test_create_convert() {
    let trace = read_trace(Path::new("src/res/create_trace.json"));
    let ctx = conversion::TransactionContext {
        block_hash: Default::default(),
        block_number: 0,
        transaction_hash: Default::default(),
        transaction_position: 0,
    };
    let mut alchemy_trace = conversion::from_call_frame(ctx, trace.clone());

    assert_eq!(
        alchemy_trace.len(),
        1,
        "There are no subcalls in the create trace"
    );

    let alchemy_trace = match alchemy_trace.pop().unwrap() {
        AlchemyTrace::Create(trace) => trace,
        other => panic!("Expected Create trace, got {other:?}"),
    };

    assert_eq!(alchemy_trace.action.from, trace.from);
    assert_eq!(alchemy_trace.action.value, trace.value);
    assert_eq!(alchemy_trace.action.gas, trace.gas);
    assert_eq!(alchemy_trace.action.init, trace.input);
    assert_eq!(alchemy_trace.result.address, trace.to.unwrap());
    assert_eq!(alchemy_trace.result.code, trace.output);
}

#[test]
fn test_call_convert() {
    let trace = read_trace(Path::new("src/res/call_trace.json"));
    let ctx = conversion::TransactionContext {
        block_hash: Default::default(),
        block_number: 0,
        transaction_hash: Default::default(),
        transaction_position: 0,
    };
    let alchemy_traces = conversion::from_call_frame(ctx, trace.clone());

    assert_eq!(
        alchemy_traces.len(),
        7,
        "There are 7 total calls in the call trace"
    );

    for alchemy_trace in alchemy_traces {
        let alchemy_trace = match alchemy_trace {
            AlchemyTrace::Call(trace) => trace,
            other => panic!("Expected Call trace, got {other:?}"),
        };
        let frame = {
            let mut ptr = &trace;
            for index in alchemy_trace.trace_address.iter().copied() {
                ptr = ptr.calls.get(index as usize).unwrap();
            }
            ptr
        };
        validate_call_trace(&alchemy_trace, frame);
    }
}

fn validate_call_trace(alchemy_trace: &AlchemyCallTrace, trace: &CallFrame) {
    assert_eq!(alchemy_trace.action.from, trace.from);
    assert_eq!(alchemy_trace.action.value, trace.value);
    assert_eq!(alchemy_trace.action.gas, trace.gas);
    assert_eq!(alchemy_trace.result.gas_used, trace.gas_used);
    assert_eq!(alchemy_trace.action.to, trace.to.unwrap());
    assert_eq!(alchemy_trace.action.input, trace.input);
    assert_eq!(alchemy_trace.result.output, trace.output);

    // Traverse the subcalls to count them
    let mut q = std::collections::VecDeque::new();
    q.push_back(trace);
    let mut subtraces = 0;
    while let Some(t) = q.pop_front() {
        subtraces += t.calls.len();
        for child in t.calls.iter() {
            q.push_back(child);
        }
    }

    assert_eq!(alchemy_trace.subtraces, subtraces as u32);
}

fn read_trace(path: &Path) -> CallFrame {
    let json_data = std::fs::read_to_string(path).unwrap();
    let serialized: DeserializableCallFrame = serde_json::from_str(&json_data).unwrap();
    serialized.into()
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct DeserializableCallFrame {
    #[serde(rename = "type")]
    call_type: String,
    from: String,
    #[serde(default)]
    to: Option<String>,
    #[serde(default)]
    value: Option<String>,
    gas: String,
    #[serde(rename = "gasUsed")]
    gas_used: String,
    input: String,
    output: String,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    calls: Vec<DeserializableCallFrame>,
}

impl From<DeserializableCallFrame> for CallFrame {
    fn from(serialized: DeserializableCallFrame) -> Self {
        Self {
            call_type: parse_call_type(&serialized.call_type),
            from: Address::decode(serialized.from.strip_prefix("0x").unwrap()).unwrap(),
            to: Address::decode(serialized.to.unwrap().strip_prefix("0x").unwrap()).ok(),
            value: U256::from_str_radix(
                serialized
                    .value
                    .as_deref()
                    .unwrap_or("0x0")
                    .strip_prefix("0x")
                    .unwrap(),
                16,
            )
            .unwrap(),
            gas: u64::from_str_radix(serialized.gas.strip_prefix("0x").unwrap(), 16).unwrap(),
            gas_used: u64::from_str_radix(serialized.gas.strip_prefix("0x").unwrap(), 16).unwrap(),
            input: hex::decode(serialized.input.strip_prefix("0x").unwrap()).unwrap(),
            output: hex::decode(serialized.output.strip_prefix("0x").unwrap()).unwrap(),
            error: serialized.error,
            calls: serialized.calls.into_iter().map(Into::into).collect(),
        }
    }
}

fn parse_call_type(s: &str) -> CallType {
    match s {
        "CALL" => CallType::Call,
        "STATICCALL" => CallType::StaticCall,
        "DELEGATECALL" => CallType::DelegateCall,
        "CALLCODE" => CallType::CallCode,
        "CREATE" => CallType::Create,
        "CREATE2" => CallType::Create2,
        "SELFDESTRUCT" => CallType::SelfDestruct,
        _ => panic!("Unknown call type {s}"),
    }
}
