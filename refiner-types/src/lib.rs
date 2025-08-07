#![allow(clippy::literal_string_with_formatting_args)]

pub use near_primitives;

pub mod aurora_block;
pub mod bloom;
pub mod conversion;
pub mod near_block;
pub mod source_config;
pub mod utils;

/// Trait that converts a data_lake type to a same nearcore type.
pub trait Converter<T> {
    fn convert(self) -> T;
}
