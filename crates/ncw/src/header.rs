use std::io::Read;

use crate::read_bytes::ReadBytesExt;

type Error = crate::NcwError;

pub const HEADER_SIZE: usize = 120;
const FILE_MAGICS: [u64; 2] = [0x01A89ED631010000, 0x01A89ED630010000];

/// The 120-byte file header. Only the first 32 bytes are understood; the
/// remainder is preserved by writers but has no known meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NcwHeader {
    pub channels: u16,
    pub bits_per_sample: u16,
    pub sample_rate: u32,
    /// Number of sample frames (samples per channel).
    pub num_samples: u32,
    /// Absolute offset of the block offset table.
    pub blocks_offset: u32,
    /// Absolute offset of the first block.
    pub data_offset: u32,
    pub data_size: u32,
}

impl NcwHeader {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let buf = reader.read_bytes(HEADER_SIZE)?;
        let mut reader = buf.as_slice();

        if !FILE_MAGICS.contains(&reader.read_u64_be()?) {
            return Err(Error::InvalidFileSignature);
        }

        Ok(Self {
            channels: reader.read_u16_le()?,
            bits_per_sample: reader.read_u16_le()?,
            sample_rate: reader.read_u32_le()?,
            num_samples: reader.read_u32_le()?,
            blocks_offset: reader.read_u32_le()?,
            data_offset: reader.read_u32_le()?,
            data_size: reader.read_u32_le()?,
        })
    }

    /// Reject headers the decoder cannot act on before any data is read.
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.channels == 0 {
            return Err(Error::InvalidHeader("channel count is zero"));
        }
        if !matches!(self.bits_per_sample, 8 | 16 | 24 | 32) {
            return Err(Error::InvalidHeader("unsupported bits per sample"));
        }
        if self.data_offset < self.blocks_offset {
            return Err(Error::InvalidHeader(
                "data offset precedes block offset table",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_magic_is_an_error() {
        let bytes = [0u8; HEADER_SIZE];
        assert!(matches!(
            NcwHeader::read(&mut &bytes[..]),
            Err(Error::InvalidFileSignature)
        ));
    }

    #[test]
    fn short_header_is_an_io_error() {
        let bytes = [0u8; HEADER_SIZE - 1];
        assert!(matches!(
            NcwHeader::read(&mut &bytes[..]),
            Err(Error::IoError(_))
        ));
    }
}
