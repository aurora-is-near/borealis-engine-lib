// Re-export of those types from crates.io that are private when accessed from near_lake_framework::*
// to make them available when converting to near_primitives::*
// See refiner-types/src/near_block.rs for more details
pub use near_crypto::PublicKey as PublicKeyCratesIo;
pub use near_crypto::Secp256K1Signature as Secp256K1SignatureCratesIo;
pub use near_crypto::Signature as SignatureCratesIo;

pub use near_primitives::action::GlobalContractIdentifier as GlobalContractIdentifierCratesIo;
pub use near_primitives::types::ShardId as ShardIdCratesIo;
pub use near_primitives::errors::TxExecutionError as TxExecutionErrorCratesIo;

/// NOTE: temporary added here
///  Convert between types that have the same json representation
pub fn ch_json<U: serde::Serialize, V: serde::de::DeserializeOwned>(input: U) -> V {
    let value = serde_json::to_value(input).unwrap();
    serde_json::from_value(value).unwrap()
}
