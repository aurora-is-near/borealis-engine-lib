//! Based on Parity Common Eth Bloom implementation
//! Link: https://github.com/paritytech/parity-common/blob/master/ethbloom/src/lib.rs
//!
//! Reimplemented here since there is a large miss match in types and dependencies.
#![allow(clippy::non_canonical_clone_impl)]
use fixed_hash::construct_fixed_hash;
use impl_serde::impl_fixed_hash_serde;

use crate::utils;

const BLOOM_SIZE: usize = 256;
const BLOOM_BITS: u32 = 3;

construct_fixed_hash! {
    /// Bloom hash type with 256 bytes (2048 bits) size.
    pub struct Bloom(BLOOM_SIZE);
}

impl_fixed_hash_serde!(Bloom, BLOOM_SIZE);

/// Returns log2.
const fn log2(x: usize) -> u32 {
    if x <= 1 {
        return 0;
    }

    let n = x.leading_zeros();
    std::mem::size_of::<usize>() as u32 * 8 - n
}

impl Bloom {
    /// Add a new element to the bloom filter
    pub fn accrue(&mut self, input: &[u8]) {
        let m = self.0.len();
        let bloom_bits = m * 8;
        let mask = bloom_bits - 1;
        let bloom_bytes = log2(bloom_bits).div_ceil(8);
        let hash = utils::keccak256(input);
        let mut ptr = 0;

        for i in 0..BLOOM_BITS {
            let _ = i;
            let mut index = 0;
            for _ in 0..bloom_bytes {
                index = (index << 8) | hash[ptr] as usize;
                ptr += 1;
            }
            index &= mask;
            self.0[m - 1 - index / 8] |= 1 << (index % 8);
        }
    }

    /// Merge two bloom filters
    pub fn accrue_bloom(&mut self, bloom: &Self) {
        for i in 0..BLOOM_SIZE {
            self.0[i] |= bloom.0[i];
        }
    }
}
