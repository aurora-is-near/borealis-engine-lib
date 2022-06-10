use aurora_engine_types::H256;
use sha3::Digest;

pub fn keccak256(input: &[u8]) -> H256 {
    let mut hasher = sha3::Keccak256::default();
    hasher.update(input);
    H256(hasher.finalize().into())
}
