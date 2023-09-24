use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::read_bytes::ReadBytesExt;

type Error = crate::NcwError;

const HEADER_SIZE: usize = 120;
const BLOCK_HEADER_SIZE: usize = 16;
const MAX_SAMPLES_PER_BLOCK: usize = 512;
// const MAX_CHANNELS: usize = 6;

#[derive(Debug)]
pub struct NcwReader<R> {
    pub reader: R,
    pub header: NcwHeader,
    pub block_offsets: Vec<u32>,
    pub current_block: usize,
}

#[derive(Debug)]
pub struct NcwHeader {
    pub channels: u16,
    pub bits_per_sample: u16,
    pub sample_rate: u32,
    pub num_samples: u32,
    pub blocks_offset: u32,
    pub data_offset: u32,
    pub data_size: u32,
}

#[derive(Debug)]
pub struct BlockHeader {
    pub base_value: i32,
    pub bits: i16,
    pub flags: u16,
}

impl<R: Read + Seek> NcwReader<R> {
    pub fn read(mut reader: R) -> Result<Self, Error> {
        let header = NcwHeader::read(&mut reader)?;
        let block_offsets_len = header.data_offset - header.blocks_offset;
        let num_blocks = block_offsets_len / 4;

        let mut block_offsets = Vec::new();
        for _ in 1..num_blocks {
            block_offsets.push(reader.read_u32_le()?);
        }

        Ok(Self {
            reader,
            header,
            block_offsets,
            current_block: 0,
        })
    }

    pub fn read_block(&self, block_data: &Vec<u8>, block_header: &BlockHeader) -> Vec<i32> {
        match block_header.flags {
            0 => {}
            1 => unimplemented!("mid/side compression not implemented yet!"),
            2 => unimplemented!("type 2 compression not implemented yet!"),
            _ => panic!("Unknown interleaving format"),
        }

        let bits = block_header.bits.unsigned_abs() as usize;
        // let block_data_len = bits * 64;
        // let mut block_data = vec![0; block_data_len];
        // self.reader.read_exact(&mut block_data).unwrap();

        match block_header.bits.cmp(&0) {
            std::cmp::Ordering::Greater => {
                // Delta decode, block_data represents the delta from base_value
                decode_delta_block_i32(block_header.base_value, &block_data, bits)
            }
            std::cmp::Ordering::Less => {
                // Bit truncation (simple compression)
                let bits = block_header.bits.unsigned_abs() as usize;
                decode_truncated_block_i32(&block_data, bits)
            }
            std::cmp::Ordering::Equal => {
                // No compression
                let bytes_per_sample = self.header.bits_per_sample as usize / 8;
                let mut samples = Vec::new();

                let mut reader = Cursor::new(block_data);

                for _ in 0..512 {
                    let sample_bytes = reader.read_bytes(bytes_per_sample).unwrap();
                    let sample = i32::from_le_bytes(sample_bytes.try_into().unwrap());
                    samples.push(sample);
                }

                samples
            }
        }
    }

    // /// Decode all blocks into contiguous 32-bit PCM samples.
    // pub fn decode_samples(&mut self) -> Result<Vec<i32>, Error> {
    //     let total_samples = self.header.num_samples as usize * self.header.channels as usize;
    //     let mut interleaved_samples = Vec::with_capacity(total_samples);
    //
    //     let mut channels = vec![
    //         vec![0u8; self.header.bits_per_sample as usize * 64];
    //         self.header.channels as usize
    //     ];
    // }

    /// Decode all blocks into contiguous 32-bit PCM samples.
    pub fn decode_samples(&mut self) -> Result<Vec<i32>, Error> {
        let total_samples = self.header.num_samples as usize * self.header.channels as usize;
        let mut samples = Vec::new();

        let overflow_samples =
            (total_samples % MAX_SAMPLES_PER_BLOCK) / self.header.channels as usize;

        for i in 0..self.block_offsets.len() {
            let is_final_block: bool = i == self.block_offsets.len() - 1;

            // Seek to current block
            self.reader.seek(SeekFrom::Start(
                self.header.data_offset as u64 + self.block_offsets[i] as u64,
            ))?;

            for _ in 0..self.header.channels {
                let block_header = BlockHeader::read(&mut self.reader)?;

                let bits = block_header.bits.unsigned_abs();
                let data = self.reader.read_bytes(bits as usize * 64)?;

                // dbg!(block_header, data);

                let mut current_sample = 0;
                for sample in self.read_block(&data.clone(), &block_header) {
                    let is_final_sample: bool = current_sample >= overflow_samples;

                    current_sample += 1;

                    // if we are on the final block
                    if is_final_block && is_final_sample {
                    } else {
                        samples.push(sample);
                    }
                }

                // for sample in self.read_block(block_header) {
                //     // dbg!(&sample, self.header.num_samples, current_sample);
                //     samples.push(sample);
                //     current_sample += 1;
                //
                //     // if this is the last block
                //     if current_block == self.block_offsets.len() - 1 {
                //         return Ok(samples);
                //     }
                // }
            }
        }

        return Ok(samples);
    }

    // /// Decode all blocks into interleaved 32-bit PCM samples.
    // pub fn decode_interleaved_samples(&mut self) -> Result<Vec<i32>, Error> {
    //     let mut interleaved_samples = Vec::new();
    //
    //     for block_offset in self.block_offsets.clone() {
    //         let mut channels = vec![Vec::new(); self.header.channels as usize];
    //         let relative_block_offset = (self.header.data_offset + block_offset) as u64;
    //
    //         // Seek to block
    //         self.reader.seek(SeekFrom::Start(relative_block_offset))?;
    //
    //         for (i, channel) in (0..self.header.channels).enumerate() {
    //             let block_header = BlockHeader::read(&mut self.reader)?;
    //             let samples = self.read_block(block_header);
    //
    //             dbg!(&samples);
    //             // dbg!(i == self.header.channels as usize - 1);
    //             // dbg!(self.header.num_samples as usize % MAX_SAMPLES_PER_BLOCK);
    //
    //             for sample in samples {
    //                 channels[channel as usize].push(sample);
    //             }
    //         }
    //
    //         // interleave samples
    //         let min_length = channels.iter().map(|ch| ch.len()).min().unwrap();
    //         for i in 0..min_length {
    //             for ch in &channels {
    //                 interleaved_samples.push(ch[i]);
    //             }
    //         }
    //     }
    //     Ok(interleaved_samples)
    // }
}

fn decode_delta_block_i32(base_sample: i32, deltas: &[u8], bits: usize) -> Vec<i32> {
    assert_eq!(deltas.len(), bits * 64);

    let mut samples: Vec<i32> = vec![0; MAX_SAMPLES_PER_BLOCK];
    let mut prev_base = base_sample;
    let delta_values = read_packed_values_i32(deltas, bits);

    for (i, delta) in delta_values.iter().enumerate() {
        samples[i] = prev_base;
        prev_base += delta;
    }

    samples
}

fn decode_truncated_block_i32(data: &[u8], bit_size: usize) -> Vec<i32> {
    let mut samples: Vec<i32> = Vec::new();
    let mut bit_offset = 0;

    while bit_offset + bit_size <= (data.len() * 8) {
        let byte_offset = bit_offset / 8;
        let bit_remainder = bit_offset % 8;

        let mut temp: i32 = 0;
        for i in 0..((bit_size + 7) / 8) {
            temp |= (data[byte_offset + i] as i32) << (i * 8);
        }
        let value = (temp >> bit_remainder) & ((1 << bit_size) - 1);
        samples.push(value);

        bit_offset += bit_size;
    }

    samples
}

fn read_packed_values_i32(data: &[u8], precision_in_bits: usize) -> Vec<i32> {
    let mut values: Vec<i32> = Vec::new();
    let mut bit_accumulator: i32 = 0;
    let mut bits_in_accumulator: usize = 0;
    let mut byte_index = 0;

    while byte_index < data.len() {
        // Accumulate more bits
        bit_accumulator |= (data[byte_index] as i32) << bits_in_accumulator;
        bits_in_accumulator += 8;
        byte_index += 1;

        // Extract values as long as enough bits are available
        while bits_in_accumulator >= precision_in_bits {
            let mut value = bit_accumulator & ((1 << precision_in_bits) - 1);
            if value & (1 << (precision_in_bits - 1)) != 0 {
                value |= !0 << precision_in_bits;
            }
            values.push(value);

            // Remove used bits
            bit_accumulator >>= precision_in_bits;
            bits_in_accumulator -= precision_in_bits;
        }
    }

    values
}

impl BlockHeader {
    pub fn read<R: ReadBytesExt>(mut reader: R) -> Result<BlockHeader, Error> {
        let mut block_reader = Cursor::new(reader.read_bytes(BLOCK_HEADER_SIZE)?);

        let magic = block_reader.read_u32_be()?;
        assert_eq!(magic, 0x160C9A3E);

        Ok(BlockHeader {
            base_value: block_reader.read_i32_le()?,
            bits: block_reader.read_i16_le()?,
            flags: block_reader.read_u16_le()?,
        })
    }
}

impl NcwHeader {
    pub fn read<R: ReadBytesExt>(mut reader: R) -> Result<Self, Error> {
        let mut reader = Cursor::new(reader.read_bytes(HEADER_SIZE)?);

        let magic = reader.read_u64_be()?;
        assert!([0x01A89ED631010000, 0x01A89ED630010000].contains(&magic));

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
    use std::fs::File;

    #[test]
    fn test_read_16bit_mono() -> Result<(), Error> {
        let file = File::open("tests/data/16-bit-mono.ncw")?;
        let mut ncw = NcwReader::read(file)?;
        let samples = ncw.decode_samples()?;

        assert_eq!(samples[0], 0x0000);
        assert_eq!(samples[16], 0x001B);
        assert_eq!(samples[32], 0xFF5A);

        assert_eq!(
            ncw.header.num_samples as usize,
            samples.len(),
            "Incorrect number of decoded samples"
        );
        Ok(())
    }

    #[test]
    fn test_read_onezero_testfile() -> Result<(), Error> {
        let file = File::open("tests/data/testfile-onezero-16-bit-stereo.ncw")?;
        let mut ncw = NcwReader::read(file)?;
        let samples = ncw.decode_samples()?;

        assert_eq!(
            ncw.header.num_samples as usize,
            samples.len() / ncw.header.channels as usize,
            "Incorrect number of decoded samples"
        );
        Ok(())
    }

    #[test]
    fn test_read_16bit_stereo() -> Result<(), Error> {
        let file = File::open("tests/data/16-bit-stereo.ncw")?;
        let mut ncw = NcwReader::read(file)?;
        let samples = ncw.decode_samples()?;

        assert_eq!(samples[0], -1); // left sample 1
        assert_eq!(samples[1], 0); // left sample 1

        assert_eq!(
            ncw.header.num_samples as usize,
            samples.len() / ncw.header.channels as usize,
            "Incorrect number of decoded samples"
        );
        Ok(())
    }

    #[test]
    fn test_read_24bit_mono() -> Result<(), Error> {
        let file = File::open("tests/data/24-bit-mono.ncw")?;
        let mut ncw = NcwReader::read(file)?;
        let _samples = ncw.decode_samples()?;
        Ok(())
    }

    #[test]
    fn test_read_unknown_flag() -> Result<(), Error> {
        let file = File::open("tests/data/unknown-flag.ncw")?;
        let mut ncw = NcwReader::read(file)?;
        ncw.decode_samples()?;
        Ok(())
    }
}
