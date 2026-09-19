use std::io::{Read, Seek, SeekFrom};

use crate::block::{BlockHeader, ChannelEncoding, SAMPLES_PER_BLOCK, SampleFormat};
use crate::block::{decode_mid_side, read_block};
use crate::header::NcwHeader;
use crate::read_bytes::ReadBytesExt;

type Error = crate::NcwError;

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
            for (channel_index, channel) in channels.iter_mut().enumerate() {
                let block_header = BlockHeader::read(&mut self.reader)?;
                if block_header.sample_format() != self.sample_format {
                    return Err(Error::InvalidHeader("sample format changes between blocks"));
                }
                if channel_index == 0 {
                    mid_side = block_header.channel_encoding() == ChannelEncoding::MidSide;
                }
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
