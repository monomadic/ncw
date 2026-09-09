use std::io::Read;

use crate::bits::{packed_values, sign_extend};
use crate::header::NcwHeader;
use crate::read_bytes::ReadBytesExt;

type Error = crate::NcwError;

const BLOCK_HEADER_SIZE: usize = 16;
const BLOCK_MAGIC: u32 = 0x160C9A3E;

/// Every block holds this many samples for one channel. The final block of a
/// file is padded up to this size.
pub const SAMPLES_PER_BLOCK: usize = 512;

/// The 16-byte header that precedes each per-channel block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockHeader {
    /// First sample of the block when delta-encoded.
    pub base_value: i32,
    /// Positive: delta-encoded at this many bits per delta.
    /// Negative: raw samples at `abs(bits)` bits each.
    /// Zero: raw samples at the file's `bits_per_sample`.
    pub bits: i16,
    pub flags: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelEncoding {
    LeftRight,
    MidSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    Pcm,
    Float,
}

impl BlockHeader {
    pub fn read<R: Read>(reader: &mut R) -> Result<BlockHeader, Error> {
        let buf = reader.read_bytes(BLOCK_HEADER_SIZE)?;
        let mut reader = buf.as_slice();

        if reader.read_u32_be()? != BLOCK_MAGIC {
            return Err(Error::InvalidBlockSignature);
        }

        Ok(BlockHeader {
            base_value: reader.read_i32_le()?,
            bits: reader.read_i16_le()?,
            flags: reader.read_u16_le()?,
        })
    }

    pub fn channel_encoding(&self) -> ChannelEncoding {
        if self.flags & 0b01 != 0 {
            ChannelEncoding::MidSide
        } else {
            ChannelEncoding::LeftRight
        }
    }

    pub fn sample_format(&self) -> SampleFormat {
        if self.flags & 0b10 != 0 {
            SampleFormat::Float
        } else {
            SampleFormat::Pcm
        }
    }
}

/// Read and decode one channel's block body, appending the samples to `out`.
/// The reader must be positioned just after the block header.
pub(crate) fn read_block<R: Read>(
    reader: &mut R,
    header: &NcwHeader,
    block: &BlockHeader,
    out: &mut Vec<i32>,
) -> Result<(), Error> {
    let bits = block.bits.unsigned_abs() as usize;
    if bits > 32 {
        return Err(Error::UnsupportedBitDepth(block.bits));
    }

    match block.bits.cmp(&0) {
        std::cmp::Ordering::Greater => {
            // Delta encoded: each value is the difference to the next sample.
            let data = reader.read_bytes(bits * SAMPLES_PER_BLOCK / 8)?;
            let mut current = block.base_value;
            for delta in packed_values(&data, bits) {
                out.push(current);
                current = current.wrapping_add(delta);
            }
        }
        std::cmp::Ordering::Less => {
            // Bit truncated: raw samples packed at `bits` bits each.
            let data = reader.read_bytes(bits * SAMPLES_PER_BLOCK / 8)?;
            out.extend(packed_values(&data, bits));
        }
        std::cmp::Ordering::Equal => {
            // Uncompressed at the file's native bit depth (validated on read).
            let bytes_per_sample = header.bits_per_sample as usize / 8;
            let data = reader.read_bytes(bytes_per_sample * SAMPLES_PER_BLOCK)?;
            out.extend(data.chunks_exact(bytes_per_sample).map(|chunk| {
                let mut raw = [0u8; 4];
                raw[..bytes_per_sample].copy_from_slice(chunk);
                sign_extend(u32::from_le_bytes(raw), bytes_per_sample * 8)
            }));
        }
    }
    Ok(())
}

/// Convert a mid/side channel pair to left/right in place.
///
/// The encoder stores `mid = (l + r) / 2` and `side = (l - r) / 2`, so left is
/// `mid + side` and right is `mid - side`. The same arithmetic applies to PCM
/// and float files.
pub(crate) fn decode_mid_side(mid: &mut [i32], side: &mut [i32], format: SampleFormat) {
    for (m, s) in mid.iter_mut().zip(side.iter_mut()) {
        match format {
            SampleFormat::Pcm => {
                let (left, right) = (m.wrapping_add(*s), m.wrapping_sub(*s));
                *m = left;
                *s = right;
            }
            SampleFormat::Float => {
                let (mid, side) = (f32::from_bits(*m as u32), f32::from_bits(*s as u32));
                *m = (mid + side).to_bits() as i32;
                *s = (mid - side).to_bits() as i32;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mid_side_pcm() {
        let mut mid = vec![10, -3, i32::MAX];
        let mut side = vec![4, 5, 1];
        decode_mid_side(&mut mid, &mut side, SampleFormat::Pcm);
        assert_eq!(mid, vec![14, 2, i32::MIN]);
        assert_eq!(side, vec![6, -8, i32::MAX - 1]);
    }

    #[test]
    fn mid_side_float() {
        let mut mid = vec![0.5f32.to_bits() as i32];
        let mut side = vec![0.25f32.to_bits() as i32];
        decode_mid_side(&mut mid, &mut side, SampleFormat::Float);
        assert_eq!(f32::from_bits(mid[0] as u32), 0.75);
        assert_eq!(f32::from_bits(side[0] as u32), 0.25);
    }

    #[test]
    fn block_header_flags() {
        let mut bytes = BLOCK_MAGIC.to_be_bytes().to_vec();
        bytes.extend(7i32.to_le_bytes());
        bytes.extend((-16i16).to_le_bytes());
        bytes.extend(0b11u16.to_le_bytes());
        bytes.extend([0; 4]);
        let h = BlockHeader::read(&mut &bytes[..]).unwrap();
        assert_eq!(h.base_value, 7);
        assert_eq!(h.bits, -16);
        assert_eq!(h.channel_encoding(), ChannelEncoding::MidSide);
        assert_eq!(h.sample_format(), SampleFormat::Float);
    }

    #[test]
    fn block_header_bad_magic() {
        let bytes = [0u8; BLOCK_HEADER_SIZE];
        assert!(matches!(
            BlockHeader::read(&mut &bytes[..]),
            Err(Error::InvalidBlockSignature)
        ));
    }
}
