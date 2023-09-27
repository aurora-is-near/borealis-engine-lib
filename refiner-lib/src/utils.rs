//! Useful tools to convert between equivalent types

use aurora_engine_types::{H256, U256};
pub use aurora_refiner_types::utils::keccak256;
use rlp::{DecoderError, Rlp};
use std::convert::TryFrom;

pub fn as_h256(data: &[u8]) -> H256 {
    let buffer = &mut [0u8; 32];
    buffer.copy_from_slice(data);
    H256::from(buffer)
}

pub struct TxMetadata {
    pub tx_type: u8,
    pub v: u64,
    pub r: U256,
    pub s: U256,
}

impl TryFrom<&[u8]> for TxMetadata {
    type Error = DecoderError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(DecoderError::Custom("Transaction"))
        } else {
            match value[0] {
                0x01 => {
                    let rlp = Rlp::new(&value[1..]);
                    Ok(Self {
                        tx_type: 0x1,
                        v: rlp.val_at(8)?,
                        r: rlp.val_at(9)?,
                        s: rlp.val_at(10)?,
                    })
                }
                0x02 => {
                    let rlp = Rlp::new(&value[1..]);
                    Ok(Self {
                        tx_type: 0x2,
                        v: rlp.val_at(9)?,
                        r: rlp.val_at(10)?,
                        s: rlp.val_at(11)?,
                    })
                }
                0x00..=0x7f => Err(DecoderError::Custom(
                    "Unsupported transaction type [0x00:0x7f]",
                )),
                0xff => Err(DecoderError::Custom("Unsupported transaction type (0xff)")),
                _ => {
                    let rlp = Rlp::new(value);
                    Ok(Self {
                        tx_type: 0x0,
                        v: rlp.val_at(6)?,
                        r: rlp.val_at(7)?,
                        s: rlp.val_at(8)?,
                    })
                }
            }
        }
    }
}
