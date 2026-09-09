use std::{error::Error, fmt::Display};

#[derive(Debug)]
#[non_exhaustive]
pub enum NcwError {
    /// The file does not start with an NCW magic number.
    InvalidFileSignature,
    /// A block does not start with the NCW block magic number.
    InvalidBlockSignature,
    /// A header field is inconsistent with the rest of the file.
    InvalidHeader(&'static str),
    /// A block header requests an unsupported bit width.
    UnsupportedBitDepth(i16),
    /// Fewer samples were decoded than the header promised.
    TruncatedData { expected: usize, actual: usize },
    /// Reading the underlying stream failed. A file that ends early surfaces
    /// as [`std::io::ErrorKind::UnexpectedEof`].
    IoError(std::io::Error),
}

impl Error for NcwError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for NcwError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFileSignature => write!(f, "invalid NCW file signature"),
            Self::InvalidBlockSignature => write!(f, "invalid NCW block signature"),
            Self::InvalidHeader(what) => write!(f, "invalid NCW header: {what}"),
            Self::UnsupportedBitDepth(bits) => write!(f, "unsupported block bit depth: {bits}"),
            Self::TruncatedData { expected, actual } => write!(
                f,
                "decoded {actual} samples per channel, header promised {expected}"
            ),
            Self::IoError(e) => write!(f, "io error: {e}"),
        }
    }
}

impl From<std::io::Error> for NcwError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}
