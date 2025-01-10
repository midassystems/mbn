pub const METADATA_LENGTH_MULTIPLIER: u8 = 4;
pub const PRICE_SCALE: i64 = 1_000_000_000;
pub mod backtest;
pub mod backtest_decoder;
pub mod backtest_encode;
pub mod decode;
pub mod decode_iterator;
pub mod encode;
pub mod enums;
pub mod error;
pub mod live;
pub mod metadata;
pub mod record_enum;
pub mod record_ref;
pub mod records;
pub mod symbols;
pub mod utils;

pub use error::{Error, Result};

#[cfg(feature = "python")]
pub mod python;
