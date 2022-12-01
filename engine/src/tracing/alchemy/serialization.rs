use crate::tracing::alchemy::{
    AlchemyCallAction, AlchemyCallKind, AlchemyCallResult, AlchemyCallType, AlchemyCreateAction,
    AlchemyCreateResult, AlchemyCreateType, AlchemySuicideAction, AlchemySuicideResult,
    AlchemySuicideType, AlchemyTraceTemplate,
};
use aurora_engine_types::{types::Address, H256, U256};
use std::{borrow::Cow, str::FromStr, string::ToString};

pub const ALCHEMY_CREATE_TYPE_TAG: &str = "create";
pub const ALCHEMY_SUICIDE_TYPE_TAG: &str = "suicide";
pub const ALCHEMY_CALL_TYPE_TAG: &str = "call";
const ALCHEMY_CALL_KIND_CALL_TAG: &str = "call";
const ALCHEMY_CALL_KIND_DELEGATE_CALL_TAG: &str = "delegatecall";
const ALCHEMY_CALL_KIND_CALL_CODE_TAG: &str = "callcode";
const ALCHEMY_CALL_KIND_STATIC_CALL_TAG: &str = "staticcall";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializableAlchemyTrace<'a> {
    pub action: SerializableAlchemyTraceAction,
    pub block_hash: String,
    pub block_number: u64,
    pub result: Option<SerializableAlchemyTraceResult>,
    pub subtraces: u32,
    pub trace_address: Cow<'a, [u32]>,
    pub transaction_hash: String,
    pub transaction_position: u32,
    #[serde(rename = "type")]
    pub typ: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializableAlchemyTraceAction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializableAlchemyTraceResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

impl<'a, A, R, T> From<&'a AlchemyTraceTemplate<A, R, T>> for SerializableAlchemyTrace<'a>
where
    SerializableAlchemyTraceAction: for<'b> From<&'b A>,
    Option<SerializableAlchemyTraceResult>: for<'b> From<&'b R>,
    T: ToString,
{
    fn from(trace: &'a AlchemyTraceTemplate<A, R, T>) -> Self {
        Self {
            action: (&trace.action).into(),
            block_hash: encode_hex(trace.block_hash.as_bytes()),
            block_number: trace.block_number,
            result: (&trace.result).into(),
            subtraces: trace.subtraces,
            trace_address: Cow::Borrowed(&trace.trace_address),
            transaction_hash: encode_hex(trace.transaction_hash.as_bytes()),
            transaction_position: trace.transaction_position,
            typ: trace.typ.to_string(),
        }
    }
}

impl<'a, A, R, T> TryFrom<SerializableAlchemyTrace<'a>> for AlchemyTraceTemplate<A, R, T>
where
    A: for<'b> TryFrom<&'b SerializableAlchemyTraceAction, Error = anyhow::Error>,
    R: for<'b> TryFrom<&'b Option<SerializableAlchemyTraceResult>, Error = anyhow::Error>,
    T: FromStr<Err = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(value: SerializableAlchemyTrace<'a>) -> Result<Self, Self::Error> {
        let action = (&value.action).try_into()?;
        let block_hash = decode_hex_h256(&value.block_hash)?;
        let result = (&value.result).try_into()?;
        let trace_address = match value.trace_address {
            Cow::Borrowed(x) => x.to_vec(),
            Cow::Owned(x) => x,
        };
        let transaction_hash = decode_hex_h256(&value.transaction_hash)?;
        let typ = T::from_str(&value.typ)?;

        Ok(Self {
            action,
            block_hash,
            block_number: value.block_number,
            result,
            subtraces: value.subtraces,
            trace_address,
            transaction_hash,
            transaction_position: value.transaction_position,
            typ,
        })
    }
}

impl From<&AlchemyCreateAction> for SerializableAlchemyTraceAction {
    fn from(action: &AlchemyCreateAction) -> Self {
        Self {
            from: Some(encode_hex(action.from.as_bytes())),
            gas: Some(encode_hex_int(action.gas)),
            init: Some(encode_hex(action.init.as_slice())),
            value: Some(encode_hex_int(action.value)),
            address: None,
            refund_address: None,
            balance: None,
            call_type: None,
            input: None,
            to: None,
        }
    }
}

impl TryFrom<&SerializableAlchemyTraceAction> for AlchemyCreateAction {
    type Error = anyhow::Error;

    fn try_from(value: &SerializableAlchemyTraceAction) -> Result<Self, Self::Error> {
        let from_str = value
            .from
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `from` field"))?;
        let from = decode_hex_address(from_str)?;
        let gas_str = value
            .gas
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `gas` field"))?;
        let gas = decode_hex_int(gas_str)?;
        let init_str = value
            .init
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `init` field"))?;
        let init = decode_hex(init_str)?;
        let value_str = value
            .value
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `value` field"))?;
        let value = decode_hex_u256(value_str)?;

        Ok(Self {
            from,
            gas,
            init,
            value,
        })
    }
}

impl From<&AlchemySuicideAction> for SerializableAlchemyTraceAction {
    fn from(action: &AlchemySuicideAction) -> Self {
        Self {
            from: None,
            gas: None,
            init: None,
            value: None,
            address: Some(encode_hex(action.address.as_bytes())),
            refund_address: Some(encode_hex(action.refund_address.as_bytes())),
            balance: Some(encode_hex_int(action.balance)),
            call_type: None,
            input: None,
            to: None,
        }
    }
}

impl TryFrom<&SerializableAlchemyTraceAction> for AlchemySuicideAction {
    type Error = anyhow::Error;

    fn try_from(value: &SerializableAlchemyTraceAction) -> Result<Self, Self::Error> {
        let address_str = value
            .address
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `address` field"))?;
        let address = decode_hex_address(address_str)?;
        let refund_address_str = value
            .refund_address
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `refundAddress` field"))?;
        let refund_address = decode_hex_address(refund_address_str)?;
        let balance_str = value
            .balance
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `balance` field"))?;
        let balance = decode_hex_u256(balance_str)?;

        Ok(Self {
            address,
            refund_address,
            balance,
        })
    }
}

impl From<&AlchemyCallAction> for SerializableAlchemyTraceAction {
    fn from(action: &AlchemyCallAction) -> Self {
        Self {
            from: Some(encode_hex(action.from.as_bytes())),
            gas: Some(encode_hex_int(action.gas)),
            init: None,
            value: Some(encode_hex_int(action.value)),
            address: None,
            refund_address: None,
            balance: None,
            call_type: Some(action.call_type.to_string()),
            input: Some(encode_hex(action.input.as_slice())),
            to: Some(encode_hex(action.to.as_bytes())),
        }
    }
}

impl TryFrom<&SerializableAlchemyTraceAction> for AlchemyCallAction {
    type Error = anyhow::Error;

    fn try_from(value: &SerializableAlchemyTraceAction) -> Result<Self, Self::Error> {
        let from_str = value
            .from
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `from` field"))?;
        let from = decode_hex_address(from_str)?;
        let call_type_str = value
            .call_type
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `callType` field"))?;
        let call_type = AlchemyCallKind::from_str(call_type_str)?;
        let gas_str = value
            .gas
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `gas` field"))?;
        let gas = decode_hex_int(gas_str)?;
        let input = value
            .input
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `input` field"))?;
        let input = decode_hex(input)?;
        let to_str = value
            .to
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `to` field"))?;
        let to = decode_hex_address(to_str)?;
        let value_str = value
            .value
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `value` field"))?;
        let value = decode_hex_u256(value_str)?;

        Ok(Self {
            from,
            call_type,
            gas,
            input,
            to,
            value,
        })
    }
}

impl From<&AlchemyCreateResult> for Option<SerializableAlchemyTraceResult> {
    fn from(result: &AlchemyCreateResult) -> Self {
        Some(SerializableAlchemyTraceResult {
            address: Some(encode_hex(result.address.as_bytes())),
            code: Some(encode_hex(result.code.as_slice())),
            gas_used: Some(encode_hex_int(result.gas_used)),
            output: None,
        })
    }
}

impl TryFrom<&Option<SerializableAlchemyTraceResult>> for AlchemyCreateResult {
    type Error = anyhow::Error;

    fn try_from(value: &Option<SerializableAlchemyTraceResult>) -> Result<Self, Self::Error> {
        let value = value
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `result` field"))?;
        let gas_used_str = value
            .gas_used
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `gasUsed` field"))?;
        let gas_used = decode_hex_int(gas_used_str)?;
        let code_str = value
            .code
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `output` field"))?;
        let code = decode_hex(code_str)?;
        let address_str = value
            .address
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `address` field"))?;
        let address = decode_hex_address(address_str)?;
        Ok(Self {
            gas_used,
            code,
            address,
        })
    }
}

impl From<&AlchemySuicideResult> for Option<SerializableAlchemyTraceResult> {
    fn from(_: &AlchemySuicideResult) -> Self {
        None
    }
}

impl TryFrom<&Option<SerializableAlchemyTraceResult>> for AlchemySuicideResult {
    type Error = anyhow::Error;

    fn try_from(value: &Option<SerializableAlchemyTraceResult>) -> Result<Self, Self::Error> {
        match value {
            None => Ok(AlchemySuicideResult),
            Some(_) => Err(anyhow::Error::msg("Unexpected `result` field")),
        }
    }
}

impl From<&AlchemyCallResult> for Option<SerializableAlchemyTraceResult> {
    fn from(result: &AlchemyCallResult) -> Self {
        Some(SerializableAlchemyTraceResult {
            address: None,
            code: None,
            gas_used: Some(encode_hex_int(result.gas_used)),
            output: Some(encode_hex(result.output.as_slice())),
        })
    }
}

impl TryFrom<&Option<SerializableAlchemyTraceResult>> for AlchemyCallResult {
    type Error = anyhow::Error;

    fn try_from(value: &Option<SerializableAlchemyTraceResult>) -> Result<Self, Self::Error> {
        let value = value
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `result` field"))?;
        let gas_used_str = value
            .gas_used
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `gasUsed` field"))?;
        let gas_used = decode_hex_int(gas_used_str)?;
        let output_str = value
            .output
            .as_ref()
            .ok_or_else(|| anyhow::Error::msg("Missing `output` field"))?;
        let output = decode_hex(output_str)?;
        Ok(Self { gas_used, output })
    }
}

impl ToString for AlchemyCallKind {
    fn to_string(&self) -> String {
        match self {
            AlchemyCallKind::Call => ALCHEMY_CALL_KIND_CALL_TAG.into(),
            AlchemyCallKind::DelegateCall => ALCHEMY_CALL_KIND_DELEGATE_CALL_TAG.into(),
            AlchemyCallKind::CallCode => ALCHEMY_CALL_KIND_CALL_CODE_TAG.into(),
            AlchemyCallKind::StaticCall => ALCHEMY_CALL_KIND_STATIC_CALL_TAG.into(),
        }
    }
}
impl FromStr for AlchemyCallKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ALCHEMY_CALL_KIND_CALL_TAG => Ok(Self::Call),
            ALCHEMY_CALL_KIND_DELEGATE_CALL_TAG => Ok(Self::DelegateCall),
            ALCHEMY_CALL_KIND_CALL_CODE_TAG => Ok(Self::CallCode),
            ALCHEMY_CALL_KIND_STATIC_CALL_TAG => Ok(Self::StaticCall),
            _ => Err(anyhow::Error::msg("Invalid callType value")),
        }
    }
}

impl ToString for AlchemyCreateType {
    fn to_string(&self) -> String {
        ALCHEMY_CREATE_TYPE_TAG.into()
    }
}
impl FromStr for AlchemyCreateType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.to_lowercase() == ALCHEMY_CREATE_TYPE_TAG {
            Ok(Self)
        } else {
            Err(anyhow::Error::msg(format!(
                "Expected string `{}`",
                ALCHEMY_CREATE_TYPE_TAG
            )))
        }
    }
}

impl ToString for AlchemyCallType {
    fn to_string(&self) -> String {
        ALCHEMY_CALL_TYPE_TAG.into()
    }
}
impl FromStr for AlchemyCallType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.to_lowercase() == ALCHEMY_CALL_TYPE_TAG {
            Ok(Self)
        } else {
            Err(anyhow::Error::msg(format!(
                "Expected string `{}`",
                ALCHEMY_CALL_TYPE_TAG
            )))
        }
    }
}

impl ToString for AlchemySuicideType {
    fn to_string(&self) -> String {
        ALCHEMY_SUICIDE_TYPE_TAG.into()
    }
}
impl FromStr for AlchemySuicideType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.to_lowercase() == ALCHEMY_SUICIDE_TYPE_TAG {
            Ok(Self)
        } else {
            Err(anyhow::Error::msg(format!(
                "Expected string `{}`",
                ALCHEMY_SUICIDE_TYPE_TAG
            )))
        }
    }
}

fn decode_hex_h256(hex_str: &str) -> anyhow::Result<H256> {
    let bytes = decode_hex(hex_str)?;
    if bytes.len() != 32 {
        return Err(anyhow::Error::msg(
            "Incorrect bytes length (expected 32 bytes)",
        ));
    }
    Ok(H256::from_slice(&bytes))
}

fn decode_hex_address(hex_str: &str) -> anyhow::Result<Address> {
    let bytes = decode_hex(hex_str)?;
    let arr: [u8; 20] = bytes
        .try_into()
        .map_err(|_| anyhow::Error::msg("Incorrect bytes length (expected 20 bytes)"))?;
    Ok(Address::from_array(arr))
}

fn decode_hex(hex_str: &str) -> anyhow::Result<Vec<u8>> {
    let hex_str = strip_0x(hex_str)?;
    let bytes = hex::decode(hex_str)?;
    Ok(bytes)
}

fn decode_hex_int(hex_str: &str) -> anyhow::Result<u64> {
    let hex_str = strip_0x(hex_str)?;
    let n = u64::from_str_radix(hex_str, 16)?;
    Ok(n)
}

fn decode_hex_u256(hex_str: &str) -> anyhow::Result<U256> {
    let hex_str = strip_0x(hex_str)?;
    let n = U256::from_str_radix(hex_str, 16)?;
    Ok(n)
}

fn strip_0x(hex_str: &str) -> anyhow::Result<&str> {
    hex_str
        .strip_prefix("0x")
        .ok_or_else(|| anyhow::Error::msg("Missing 0x prefix on hex data"))
}

fn encode_hex(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

fn encode_hex_int<T: std::fmt::LowerHex>(n: T) -> String {
    format!("{:#x}", n)
}
