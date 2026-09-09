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
    reader: R,
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
        header.validate()?;

        // The table holds one entry per block plus a trailing end-of-data
        // sentinel, which is not a block.
        let table_entries = (header.data_offset - header.blocks_offset) / 4;
        let num_blocks = table_entries.saturating_sub(1) as usize;

        reader.seek(SeekFrom::Start(header.blocks_offset as u64))?;
        // Not pre-sized: `num_blocks` comes from an untrusted header, and a
        // short file will fail the reads long before the vector grows large.
        let mut block_offsets = Vec::new();
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

    /// Borrow the underlying reader.
    pub fn get_ref(&self) -> &R {
        &self.reader
    }

    /// Mutably borrow the underlying reader. Moving its position is harmless:
    /// [`NcwReader::decode_samples`] seeks to every block explicitly.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    /// Consume the decoder and return the underlying reader.
    pub fn into_inner(self) -> R {
        self.reader
    }

    /// Decode every block into interleaved 32-bit samples.
    ///
    /// The result has exactly `num_samples * channels` entries. Mid/side
    /// encoded blocks (see [`BlockHeader::channel_encoding`]) are converted to
    /// left/right, so the output is always plain channel order.
    pub fn decode_samples(&mut self) -> Result<Vec<i32>, Error> {
        let num_samples = self.header.num_samples as usize;
        let num_channels = self.header.channels as usize;

        // Reserve no more than the block table can actually deliver, so a
        // bogus `num_samples` cannot force a huge allocation up front.
        let capacity = num_samples.min(self.block_offsets.len() * SAMPLES_PER_BLOCK);
        let mut channels: Vec<Vec<i32>> = (0..num_channels)
            .map(|_| Vec::with_capacity(capacity))
            .collect();

        for &offset in &self.block_offsets {
            if channels[0].len() >= num_samples {
                break;
            }
            let block_start = channels[0].len();

            self.reader.seek(SeekFrom::Start(
                self.header.data_offset as u64 + offset as u64,
            ))?;

            let mut mid_side = false;
            for channel in channels.iter_mut() {
                let block_header = BlockHeader::read(&mut self.reader)?;
                if block_header.sample_format() != self.sample_format {
                    return Err(Error::InvalidHeader("sample format changes between blocks"));
                }
                mid_side |= block_header.channel_encoding() == ChannelEncoding::MidSide;
                read_block(&mut self.reader, &self.header, &block_header, channel)?;
            }

            if mid_side {
                if num_channels != 2 {
                    return Err(Error::InvalidHeader(
                        "mid/side encoding requires exactly two channels",
                    ));
                }
                let (mid, side) = channels.split_at_mut(1);
                decode_mid_side(
                    &mut mid[0][block_start..],
                    &mut side[0][block_start..],
                    self.sample_format,
                );
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

/// Read and decode one channel's block body, appending the samples to `out`.
/// The reader must be positioned just after the block header.
fn read_block<R: Read>(
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
fn decode_mid_side(mid: &mut [i32], side: &mut [i32], format: SampleFormat) {
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

/// Unpack little-endian bit-packed signed integers of `bits` width (1..=32).
fn packed_values(data: &[u8], bits: usize) -> PackedValues<'_> {
    debug_assert!((1..=32).contains(&bits));
    PackedValues {
        data: data.iter(),
        bits,
        mask: (1u64 << bits) - 1,
        accumulator: 0,
        available: 0,
    }
}

struct PackedValues<'a> {
    data: std::slice::Iter<'a, u8>,
    bits: usize,
    mask: u64,
    accumulator: u64,
    available: usize,
}

impl Iterator for PackedValues<'_> {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        while self.available < self.bits {
            let &byte = self.data.next()?;
            self.accumulator |= (byte as u64) << self.available;
            self.available += 8;
        }
        let value = sign_extend((self.accumulator & self.mask) as u32, self.bits);
        self.accumulator >>= self.bits;
        self.available -= self.bits;
        Some(value)
    }
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

    fn validate(&self) -> Result<(), Error> {
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

    fn unpack(data: &[u8], bits: usize) -> Vec<i32> {
        packed_values(data, bits).collect()
    }

    #[test]
    fn packed_values_sign_extend() {
        // Two 4-bit values: 0x7 and 0xF (-1), then 0x8 (-8) and 0x0.
        assert_eq!(unpack(&[0xF7, 0x08], 4), vec![7, -1, -8, 0]);
    }

    #[test]
    fn packed_values_full_width() {
        assert_eq!(unpack(&(-24i32).to_le_bytes(), 32), vec![-24]);
        assert_eq!(unpack(&(-24i16).to_le_bytes(), 16), vec![-24]);
    }

    #[test]
    fn packed_values_unaligned_width() {
        // 5-bit values 1, 2, 3 packed LSB-first: 0b00011_00010_00001 = 0x0C41
        assert_eq!(unpack(&[0x41, 0x0C], 5), vec![1, 2, 3]);
    }

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
    fn bad_magic_is_an_error() {
        let bytes = [0u8; HEADER_SIZE];
        assert!(matches!(
            NcwReader::read(std::io::Cursor::new(bytes)),
            Err(Error::InvalidFileSignature)
        ));
    }
}
