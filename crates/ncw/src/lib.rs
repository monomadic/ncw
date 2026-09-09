#![doc = include_str!("../README.md")]

mod bits;
mod block;
mod error;
mod header;
mod read_bytes;
mod reader;

pub use self::block::{BlockHeader, ChannelEncoding, SAMPLES_PER_BLOCK, SampleFormat};
pub use self::error::NcwError;
pub use self::header::NcwHeader;
pub use self::reader::NcwReader;
