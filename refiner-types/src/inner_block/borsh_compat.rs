//! Version-bound codecs for NEAR fields whose exact Borsh bytes are needed by
//! Aurora output. NEAR block JSON has no Borsh payload for these fields, so unknown
//! protocol variants cannot be encoded without adding their schema here.

use borsh::BorshSerialize;
use near_primitives::{
    action::delegate::{DelegateAction, VersionedDelegateActionPayload},
    errors::TxExecutionError,
};
use serde::Deserialize;
use serde_json::value::RawValue;
use std::io;

pub(super) fn execution_error_bytes(value: &RawValue) -> io::Result<Option<Vec<u8>>> {
    let Ok(error) = serde_json::from_str::<TxExecutionError>(value.get()) else {
        return Ok(None);
    };
    borsh::to_vec(&error).map(Some)
}

/// Decode only when Borsh encoding is requested, not while reading the JSON
/// envelope. A future delegate version in an unrelated receipt can then
/// remain opaque without blocking the block.
#[derive(Clone, Deserialize)]
pub(super) struct DelegateActionCompat(Box<RawValue>);

impl BorshSerialize for DelegateActionCompat {
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        let action: DelegateAction =
            serde_json::from_str(self.0.get()).map_err(io::Error::other)?;
        action.serialize(writer)
    }
}

#[derive(Clone, Deserialize)]
pub(super) struct VersionedDelegateActionPayloadCompat(Box<RawValue>);

impl BorshSerialize for VersionedDelegateActionPayloadCompat {
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        let action: VersionedDelegateActionPayload =
            serde_json::from_str(self.0.get()).map_err(io::Error::other)?;
        action.serialize(writer)
    }
}
