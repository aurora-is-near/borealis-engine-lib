//! Version-bound codecs for NEAR fields whose exact Borsh bytes are needed by
//! Aurora output. NEAR block JSON has no Borsh payload for these fields, so unknown
//! protocol variants cannot be encoded without adding their schema here.

use borsh::BorshSerialize;
use near_crypto::PublicKey;
use near_primitives::{
    action::delegate::{DelegateAction, VersionedDelegateActionPayload},
    errors::TxExecutionError,
    receipt::AuthorizationContext,
    types::ProtocolVersion,
};
use serde::Deserialize;
use serde_json::value::RawValue;
use std::io;

/// Validate the authorization fields using the same combinations as nearcore's receipt view.
pub(super) fn is_legacy_action_receipt(
    signer_public_key: Option<&PublicKey>,
    authorization_context: Option<&AuthorizationContext>,
    creation_protocol_version: Option<ProtocolVersion>,
) -> io::Result<bool> {
    match (
        signer_public_key,
        authorization_context,
        creation_protocol_version,
    ) {
        (Some(_), None, None) => Ok(true),
        (None, None, None) => Err(io::Error::other(
            "legacy Action receipt requires signer_public_key",
        )),
        (None, Some(_), Some(_)) => Ok(false),
        (Some(signer), Some(AuthorizationContext::AccessKey(key)), Some(_)) if signer == key => {
            Ok(false)
        }
        (Some(_), Some(_), Some(_)) => Err(io::Error::other(
            "signer_public_key disagrees with authorization_context",
        )),
        _ => Err(io::Error::other(
            "authorization_context and creation_protocol_version must be present together",
        )),
    }
}

pub(super) fn receipt_view_size(
    receipt: &impl BorshSerialize,
    legacy_action: bool,
) -> io::Result<u64> {
    let size = borsh::object_length(receipt)? as u64;
    // Nearcore 2.14.1 encoded the signer key directly and omitted both V3 fields.
    // Each of these three fields adds a one-byte Borsh option tag in the new view.
    Ok(if legacy_action { size - 3 } else { size })
}

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
