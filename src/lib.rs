#![doc = include_str!("../README.md")]

mod error;
mod read_bytes;
mod reader;

pub use self::error::NcwError;
pub use self::reader::{
    BlockHeader, ChannelEncoding, NcwHeader, NcwReader, SAMPLES_PER_BLOCK, SampleFormat,
};
