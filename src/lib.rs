//! Decoder for the Native Instruments NCW (Compressed Wave) audio format.
//!
//! ```no_run
//! # fn main() -> Result<(), ncw::NcwError> {
//! let file = std::fs::File::open("sample.ncw")?;
//! let mut ncw = ncw::NcwReader::read(file)?;
//! println!("{} Hz, {} channels", ncw.header.sample_rate, ncw.header.channels);
//! let samples = ncw.decode_samples()?; // interleaved i32
//! # Ok(()) }
//! ```

mod error;
mod read_bytes;
mod reader;

pub use self::error::NcwError;
pub use self::reader::{BlockHeader, ChannelEncoding, NcwHeader, NcwReader, SampleFormat};
