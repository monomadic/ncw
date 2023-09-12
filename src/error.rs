use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum NcwError {
    InvalidFileSignature,
    ReadError(usize),
    UTF16Error(Vec<u16>),
    IoError(std::io::Error),
}

impl Error for NcwError {}

impl Display for NcwError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Ncw Error: {e}")
    }
}

impl From<std::io::Error> for NcwError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}
