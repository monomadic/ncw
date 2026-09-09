use std::io::{Read, Seek, SeekFrom};

use crate::read_bytes::ReadBytesExt;

type Error = crate::NcwError;

const HEADER_SIZE: usize = 120;
const BLOCK_HEADER_SIZE: usize = 16;
const BLOCK_MAGIC: u32 = 0x160C9A3E;
const FILE_MAGICS: [u64; 2] = [0x01A89ED631010000, 0x01A89ED630010000];

/// Every block holds this many samples for one channel. The final block of a
/// file is padded up to this size.
pub const SAMPLES_PER_BLOCK: usize = 512;

/// Reader for an NCW file.
///
/// Samples are returned as `i32`. For PCM files (see [`NcwReader::sample_format`])
/// they are sign-extended integer samples at the header's bit depth. For float
/// files they are the raw IEEE-754 bit patterns; convert with
/// `f32::from_bits(sample as u32)`.
#[derive(Debug)]
pub struct NcwReader<R> {
    pub reader: R,
    pub header: NcwHeader,
    /// Byte offset of each block, relative to `header.data_offset`.
    pub block_offsets: Vec<u32>,
    /// Sample format declared by the first block of the file.
    pub sample_format: SampleFormat,
}

/// The 120-byte file header.
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

impl<R: Read + Seek> NcwReader<R> {
    /// Parse the file header and block offset table.
    pub fn read(mut reader: R) -> Result<Self, Error> {
        let header = NcwHeader::read(&mut reader)?;

        if header.channels == 0 {
            return Err(Error::InvalidHeader("channel count is zero"));
        }
        if header.data_offset < header.blocks_offset {
            return Err(Error::InvalidHeader(
                "data offset precedes block offset table",
            ));
        }

        // The table holds one entry per block plus a trailing end-of-data
        // sentinel, which is not a block.
        let table_entries = (header.data_offset - header.blocks_offset) / 4;
        let num_blocks = table_entries.saturating_sub(1) as usize;

        reader.seek(SeekFrom::Start(header.blocks_offset as u64))?;
        let mut block_offsets = Vec::with_capacity(num_blocks);
        for _ in 0..num_blocks {
            block_offsets.push(reader.read_u32_le()?);
        }

        let sample_format = match block_offsets.first() {
            Some(&offset) => {
                reader.seek(SeekFrom::Start(header.data_offset as u64 + offset as u64))?;
                BlockHeader::read(&mut reader)?.sample_format()
            }
            None => SampleFormat::Pcm,
        };

        Ok(Self {
            reader,
            header,
            block_offsets,
            sample_format,
        })
    }

    /// Decode every block into interleaved 32-bit samples.
    ///
    /// The result has exactly `num_samples * channels` entries. Mid/side
    /// encoded blocks (see [`BlockHeader::channel_encoding`]) are returned as
    /// stored, without conversion to left/right.
    pub fn decode_samples(&mut self) -> Result<Vec<i32>, Error> {
        let num_samples = self.header.num_samples as usize;
        let num_channels = self.header.channels as usize;

        let mut channels = vec![Vec::with_capacity(num_samples); num_channels];

        for (i, &offset) in self.block_offsets.iter().enumerate() {
            self.reader.seek(SeekFrom::Start(
                self.header.data_offset as u64 + offset as u64,
            ))?;

            for channel in channels.iter_mut() {
                let block_header = BlockHeader::read(&mut self.reader)?;
                let samples = read_block(&mut self.reader, &self.header, &block_header)?;
                channel.extend_from_slice(&samples);
            }

            if i + 1 == self.block_offsets.len() || channels[0].len() >= num_samples {
                break;
            }
        }

        for channel in channels.iter_mut() {
            if channel.len() < num_samples {
                return Err(Error::TruncatedData {
                    expected: num_samples,
                    actual: channel.len(),
                });
            }
            // The final block is padded to SAMPLES_PER_BLOCK.
            channel.truncate(num_samples);
        }

        let mut interleaved = Vec::with_capacity(num_samples * num_channels);
        for i in 0..num_samples {
            for channel in &channels {
                interleaved.push(channel[i]);
            }
        }

        Ok(interleaved)
    }
}

/// Read and decode one channel's block body. The reader must be positioned
/// just after the block header.
fn read_block<R: Read>(
    reader: &mut R,
    header: &NcwHeader,
    block: &BlockHeader,
) -> Result<Vec<i32>, Error> {
    let bits = block.bits.unsigned_abs() as usize;
    if bits > 32 {
        return Err(Error::UnsupportedBitDepth(block.bits));
    }

    match block.bits.cmp(&0) {
        std::cmp::Ordering::Greater => {
            // Delta encoded: each value is the difference from the previous sample.
            let data = reader.read_bytes(bits * SAMPLES_PER_BLOCK / 8)?;
            Ok(decode_delta_block(block.base_value, &data, bits))
        }
        std::cmp::Ordering::Less => {
            // Bit truncated: raw samples packed at `bits` bits each.
            let data = reader.read_bytes(bits * SAMPLES_PER_BLOCK / 8)?;
            Ok(read_packed_values(&data, bits))
        }
        std::cmp::Ordering::Equal => {
            // Uncompressed at the file's native bit depth.
            let bytes_per_sample = header.bits_per_sample as usize / 8;
            if !(1..=4).contains(&bytes_per_sample) || header.bits_per_sample % 8 != 0 {
                return Err(Error::InvalidHeader("unsupported bits per sample"));
            }
            let data = reader.read_bytes(bytes_per_sample * SAMPLES_PER_BLOCK)?;
            Ok(data
                .chunks_exact(bytes_per_sample)
                .map(|chunk| {
                    let mut raw = [0u8; 4];
                    raw[..bytes_per_sample].copy_from_slice(chunk);
                    sign_extend(u32::from_le_bytes(raw), bytes_per_sample * 8)
                })
                .collect())
        }
    }
}

fn decode_delta_block(base_sample: i32, deltas: &[u8], bits: usize) -> Vec<i32> {
    let delta_values = read_packed_values(deltas, bits);

    let mut samples = Vec::with_capacity(delta_values.len());
    let mut current = base_sample;
    for delta in delta_values {
        samples.push(current);
        current = current.wrapping_add(delta);
    }
    samples
}

/// Unpack little-endian bit-packed signed integers of `bits` width (1..=32).
fn read_packed_values(data: &[u8], bits: usize) -> Vec<i32> {
    debug_assert!((1..=32).contains(&bits));
    let mut values = Vec::with_capacity(data.len() * 8 / bits);
    let mask = (1u64 << bits) - 1;
    let mut accumulator: u64 = 0;
    let mut available = 0usize;

    for &byte in data {
        accumulator |= (byte as u64) << available;
        available += 8;

        while available >= bits {
            values.push(sign_extend((accumulator & mask) as u32, bits));
            accumulator >>= bits;
            available -= bits;
        }
    }

    values
}

/// Sign-extend the low `bits` bits of `raw` to an i32.
fn sign_extend(raw: u32, bits: usize) -> i32 {
    let shift = 32 - bits as u32;
    ((raw << shift) as i32) >> shift
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_values_sign_extend() {
        // Two 4-bit values: 0x7 and 0xF (-1), then 0x8 (-8) and 0x0.
        assert_eq!(read_packed_values(&[0xF7, 0x08], 4), vec![7, -1, -8, 0]);
    }

    #[test]
    fn packed_values_full_width() {
        let bytes = (-24i32).to_le_bytes();
        assert_eq!(read_packed_values(&bytes, 32), vec![-24]);
        let bytes = (-24i16).to_le_bytes();
        assert_eq!(read_packed_values(&bytes, 16), vec![-24]);
    }

    #[test]
    fn packed_values_unaligned_width() {
        // 5-bit values 1, 2, 3 packed LSB-first: 0b00011_00010_00001 = 0x0C41
        assert_eq!(read_packed_values(&[0x41, 0x0C], 5), vec![1, 2, 3]);
    }

    #[test]
    fn delta_block_accumulates() {
        // 8-bit deltas: +1, +1, -2 starting at 10 -> 10, 11, 12, 10
        assert_eq!(
            decode_delta_block(10, &[1, 1, 0xFE, 0], 8),
            vec![10, 11, 12, 10]
        );
    }

    #[test]
    fn bad_magic_is_an_error() {
        let bytes = [0u8; HEADER_SIZE];
        assert!(matches!(
            NcwReader::read(std::io::Cursor::new(bytes)),
            Err(Error::InvalidFileSignature)
        ));
    }
}
