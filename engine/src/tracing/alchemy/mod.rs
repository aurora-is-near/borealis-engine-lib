use aurora_engine_types::{types::Address, H256, U256};

pub mod serialization;

/// EVM tracing format based on https://docs.alchemy.com/reference/what-are-evm-traces
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlchemyTrace {
    Create(AlchemyCreateTrace),
    Suicide(AlchemySuicideTrace),
    Call(AlchemyCallTrace),
}

impl AlchemyTrace {
    pub fn serializable(&self) -> serialization::SerializableAlchemyTrace {
        match self {
            Self::Create(x) => x.into(),
            Self::Suicide(x) => x.into(),
            Self::Call(x) => x.into(),
        }
    }

    pub fn deserialize(
        serializable: serialization::SerializableAlchemyTrace,
    ) -> anyhow::Result<Self> {
        match serializable.typ.as_str() {
            serialization::ALCHEMY_CREATE_TYPE_TAG => {
                let trace = AlchemyCreateTrace::try_from(serializable)?;
                Ok(Self::Create(trace))
            }
            serialization::ALCHEMY_SUICIDE_TYPE_TAG => {
                let trace = AlchemySuicideTrace::try_from(serializable)?;
                Ok(Self::Suicide(trace))
            }
            serialization::ALCHEMY_CALL_TYPE_TAG => {
                let trace = AlchemyCallTrace::try_from(serializable)?;
                Ok(Self::Call(trace))
            }
            other => Err(anyhow::Error::msg(format!(
                "Unknown `type` value: {}",
                other
            ))),
        }
    }
}

/// See https://docs.alchemy.com/reference/what-are-evm-traces#create
pub type AlchemyCreateTrace =
    AlchemyTraceTemplate<AlchemyCreateAction, AlchemyCreateResult, AlchemyCreateType>;

/// See https://docs.alchemy.com/reference/what-are-evm-traces#suicide
pub type AlchemySuicideTrace =
    AlchemyTraceTemplate<AlchemySuicideAction, AlchemySuicideResult, AlchemySuicideType>;

/// See https://docs.alchemy.com/reference/what-are-evm-traces#call
pub type AlchemyCallTrace =
    AlchemyTraceTemplate<AlchemyCallAction, AlchemyCallResult, AlchemyCallType>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlchemyCreateType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlchemySuicideType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlchemyCallType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlchemyTraceTemplate<A, R, T> {
    pub action: A,
    pub block_hash: H256,
    pub block_number: u64,
    pub result: R,
    pub subtraces: u32,
    pub trace_address: Vec<u32>,
    pub transaction_hash: H256,
    pub transaction_position: u32,
    pub typ: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlchemyCreateAction {
    pub from: Address,
    pub gas: u64,
    pub init: Vec<u8>,
    pub value: U256,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlchemyCreateResult {
    pub address: Address,
    pub code: Vec<u8>,
    pub gas_used: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlchemySuicideAction {
    pub address: Address,
    pub refund_address: Address,
    pub balance: U256,
}

/// This is a singleton because the docs say the result of suicide is always `null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlchemySuicideResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlchemyCallAction {
    pub from: Address,
    pub call_type: AlchemyCallKind,
    pub gas: u64,
    pub input: Vec<u8>,
    pub to: Address,
    pub value: U256,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlchemyCallKind {
    Call,
    DelegateCall,
    CallCode,
    StaticCall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlchemyCallResult {
    pub gas_used: u64,
    pub output: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::AlchemyTrace;

    #[test]
    fn test_call_serialization() {
        // Example from https://docs.alchemy.com/reference/what-are-evm-traces#call
        let example = r#"{
            "action": {
              "from": "0xbc9f06dd67578b0b8b4d87fda9acde453bc4c067",
              "callType": "call",
              "gas": "0x97478",
              "input": "0xfebefd610000000000000000000000000000000000000000000000000000000000000040cc849afc28894f79411f12309e75c71ded27d1666b75a2423633c204e671cb1e00000000000000000000000000000000000000000000000000000000000000036eaec0ff7c4899bec2db1479d7d195d614ca26819a301523d82daaaaf436122d2ceb36dfa12b359202b4dfd756478988f5023bf7297afa81f563d4b6242e36e707671a8bf38ee483a37feca948997dcfba17b3372e166ba5c824629beeed6b5c",
              "to": "0x6090a6e47849629b7245dfa1ca21d94cd15878ef",
              "value": "0x2386f26fc10000"
            },
            "blockHash": "0x6d00f7707938cca36b0730d8f7f090543242002b6fa0fe94bf85b9ab02e6bed6",
            "blockNumber": 4000036,
            "result": {
              "gasUsed": "0x7ad71",
              "output": "0x"
            },
            "subtraces": 4,
            "traceAddress": [],
            "transactionHash": "0x552b31a3a9c92577d65db62cf9f729e81571e10cad90e356423adcfa2caebacc",
            "transactionPosition": 71,
            "type": "call"
          }"#;
        serialization_round_trip(example)
    }

    #[test]
    fn test_create_serialization() {
        // Example from https://docs.alchemy.com/reference/what-are-evm-traces#create
        let example = r#"{
            "action": {
              "from": "0x6090a6e47849629b7245dfa1ca21d94cd15878ef",
              "gas": "0x6a7f1",
              "init": "0x606060405260405160208061051683398101604052515b60028054600160a060020a03808416600160a060020a0319928316179092556000805433909316929091169190911790554260019081556005805460ff19169091179055346004555b505b6104a6806100706000396000f300606060405236156100885763ffffffff60e060020a60003504166305b34410811461008a5780630b5ab3d5146100ac57806313af4035146100be5780632b20e397146100dc5780633fa4f24514610108578063674f220f1461012a5780638da5cb5b14610156578063b0c8097214610182578063bbe427711461019c578063faab9d39146101b1575bfe5b341561009257fe5b61009a6101cf565b60408051918252519081900360200190f35b34156100b457fe5b6100bc6101d5565b005b34156100c657fe5b6100bc600160a060020a036004351661021d565b005b34156100e457fe5b6100ec6102c3565b60408051600160a060020a039092168252519081900360200190f35b341561011057fe5b61009a6102d2565b60408051918252519081900360200190f35b341561013257fe5b6100ec6102d8565b60408051600160a060020a039092168252519081900360200190f35b341561015e57fe5b6100ec6102e7565b60408051600160a060020a039092168252519081900360200190f35b341561018a57fe5b6100bc60043560243515156102f6565b005b34156101a457fe5b6100bc600435610382565b005b34156101b957fe5b6100bc600160a060020a0360043516610431565b005b60015481565b60055460ff16156101e65760006000fd5b600254604051600160a060020a039182169130163180156108fc02916000818181858888f193505050501561021a5761deadff5b5b565b60005433600160a060020a039081169116146102395760006000fd5b600160a060020a038116151561024f5760006000fd5b600280546003805473ffffffffffffffffffffffffffffffffffffffff19908116600160a060020a03808516919091179092559084169116811790915560408051918252517fa2ea9883a321a3e97b8266c2b078bfeec6d50c711ed71f874a90d500ae2eaf369181900360200190a15b5b50565b600054600160a060020a031681565b60045481565b600354600160a060020a031681565b600254600160a060020a031681565b60005433600160a060020a039081169116146103125760006000fd5b60055460ff1615156103245760006000fd5b8160045410156103345760006000fd5b6004829055600254604051600160a060020a039182169130163184900380156108fc02916000818181858888f193505050501580156103705750805b1561037b5760006000fd5b5b5b5b5050565b60005433600160a060020a0390811691161461039e5760006000fd5b60055460ff1615156103b05760006000fd5b6005805460ff1916905561dead6108fc6103e883810330600160a060020a031631025b604051919004801590920291906000818181858888f1935050505015156103fa5760006000fd5b6040517fbb2ce2f51803bba16bc85282b47deeea9a5c6223eabea1077be696b3f265cf1390600090a16102bf6101d5565b5b5b5b50565b60005433600160a060020a0390811691161461044d5760006000fd5b6000805473ffffffffffffffffffffffffffffffffffffffff1916600160a060020a0383161790555b5b505600a165627a7a72305820fbfa6f8a2024760ef0e0eb29a332c9a820526e92f8b4fbcce6f00c7643234b140029000000000000000000000000a7f3659c53820346176f7e0e350780df304db179",
              "value": "0xe4b4b8af6a70000"
            },
            "blockHash": "0x6d00f7707938cca36b0730d8f7f090543242002b6fa0fe94bf85b9ab02e6bed6",
            "blockNumber": 4000036,
            "result": {
              "address": "0xfc9779d9a0f2715435a3e8ebf780322145d7546e",
              "code": "0x606060405236156100885763ffffffff60e060020a60003504166305b34410811461008a5780630b5ab3d5146100ac57806313af4035146100be5780632b20e397146100dc5780633fa4f24514610108578063674f220f1461012a5780638da5cb5b14610156578063b0c8097214610182578063bbe427711461019c578063faab9d39146101b1575bfe5b341561009257fe5b61009a6101cf565b60408051918252519081900360200190f35b34156100b457fe5b6100bc6101d5565b005b34156100c657fe5b6100bc600160a060020a036004351661021d565b005b34156100e457fe5b6100ec6102c3565b60408051600160a060020a039092168252519081900360200190f35b341561011057fe5b61009a6102d2565b60408051918252519081900360200190f35b341561013257fe5b6100ec6102d8565b60408051600160a060020a039092168252519081900360200190f35b341561015e57fe5b6100ec6102e7565b60408051600160a060020a039092168252519081900360200190f35b341561018a57fe5b6100bc60043560243515156102f6565b005b34156101a457fe5b6100bc600435610382565b005b34156101b957fe5b6100bc600160a060020a0360043516610431565b005b60015481565b60055460ff16156101e65760006000fd5b600254604051600160a060020a039182169130163180156108fc02916000818181858888f193505050501561021a5761deadff5b5b565b60005433600160a060020a039081169116146102395760006000fd5b600160a060020a038116151561024f5760006000fd5b600280546003805473ffffffffffffffffffffffffffffffffffffffff19908116600160a060020a03808516919091179092559084169116811790915560408051918252517fa2ea9883a321a3e97b8266c2b078bfeec6d50c711ed71f874a90d500ae2eaf369181900360200190a15b5b50565b600054600160a060020a031681565b60045481565b600354600160a060020a031681565b600254600160a060020a031681565b60005433600160a060020a039081169116146103125760006000fd5b60055460ff1615156103245760006000fd5b8160045410156103345760006000fd5b6004829055600254604051600160a060020a039182169130163184900380156108fc02916000818181858888f193505050501580156103705750805b1561037b5760006000fd5b5b5b5b5050565b60005433600160a060020a0390811691161461039e5760006000fd5b60055460ff1615156103b05760006000fd5b6005805460ff1916905561dead6108fc6103e883810330600160a060020a031631025b604051919004801590920291906000818181858888f1935050505015156103fa5760006000fd5b6040517fbb2ce2f51803bba16bc85282b47deeea9a5c6223eabea1077be696b3f265cf1390600090a16102bf6101d5565b5b5b5b50565b60005433600160a060020a0390811691161461044d5760006000fd5b6000805473ffffffffffffffffffffffffffffffffffffffff1916600160a060020a0383161790555b5b505600a165627a7a72305820fbfa6f8a2024760ef0e0eb29a332c9a820526e92f8b4fbcce6f00c7643234b140029",
              "gasUsed": "0x52ce0"
            },
            "subtraces": 0,
            "traceAddress": [
              0
            ],
            "transactionHash": "0xc9601ea5ca42e57c3ef1d770ab0b278d6aadf2511a4feb879cba573854443423",
            "transactionPosition": 70,
            "type": "create"
          }"#;
        serialization_round_trip(example)
    }

    #[test]
    fn test_suicide_serialization() {
        // Example from https://docs.alchemy.com/reference/what-are-evm-traces#suicide
        let example = r#"{
            "action": {
              "address": "0x87051f6ba0562fdb0485763562bf34cb2ad705b1",
              "refundAddress": "0x000000000000000000000000000000000000dead",
              "balance": "0x0"
            },
            "blockHash": "0x6d00f7707938cca36b0730d8f7f090543242002b6fa0fe94bf85b9ab02e6bed6",
            "blockNumber": 4000036,
            "result": null,
            "subtraces": 0,
            "traceAddress": [
              1,
              2,
              2
            ],
            "transactionHash": "0xbc15addb97490a168dc1d099ab8537caf2e4ff7d1deeff6d685d2d594a750037",
            "transactionPosition": 45,
            "type": "suicide"
          }"#;
        serialization_round_trip(example)
    }

    fn serialization_round_trip(example: &str) {
        let example_value: serde_json::Value = serde_json::from_str(example).unwrap();
        let deser = AlchemyTrace::deserialize(serde_json::from_str(example).unwrap()).unwrap();
        let reser = serde_json::to_value(deser.serializable()).unwrap();
        assert_eq!(example_value, reser);
    }
}
