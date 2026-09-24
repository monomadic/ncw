//! Decode every fixture in `tests/data` and compare it sample-for-sample
//! against the reference WAV next to it.

use std::{fs, path::Path};

use ncw::{NcwReader, SampleFormat};

struct Wav {
    channels: u16,
    sample_rate: u32,
    bits: u16,
    float: bool,
    /// Samples as the decoder would return them: sign-extended ints, or
    /// f32 bit patterns for float files.
    samples: Vec<i32>,
}

/// Minimal RIFF/WAVE parser, enough for the fixtures (PCM and IEEE float).
fn read_wav(path: &Path) -> Wav {
    let d = fs::read(path).unwrap();
    assert_eq!(&d[0..4], b"RIFF");
    assert_eq!(&d[8..12], b"WAVE");

    let mut fmt = None;
    let mut data = None;
    let mut pos = 12;
    while pos + 8 <= d.len() {
        let id = &d[pos..pos + 4];
        let size = u32::from_le_bytes(d[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let body = &d[pos + 8..(pos + 8 + size).min(d.len())];
        match id {
            b"fmt " => fmt = Some(body.to_vec()),
            b"data" => data = Some(body.to_vec()),
            _ => {}
        }
        pos += 8 + size + (size & 1);
    }
    let fmt = fmt.expect("fmt chunk");
    let data = data.expect("data chunk");

    let tag = u16::from_le_bytes([fmt[0], fmt[1]]);
    let channels = u16::from_le_bytes([fmt[2], fmt[3]]);
    let sample_rate = u32::from_le_bytes(fmt[4..8].try_into().unwrap());
    let bits = u16::from_le_bytes([fmt[14], fmt[15]]);
    let float = tag == 3;
    let bytes = bits as usize / 8;

    let samples = data
        .chunks_exact(bytes)
        .map(|c| {
            let mut raw = [0u8; 4];
            raw[..bytes].copy_from_slice(c);
            let v = u32::from_le_bytes(raw);
            if float {
                v as i32
            } else {
                let shift = 32 - bits as u32;
                ((v << shift) as i32) >> shift
            }
        })
        .collect();

    Wav {
        channels,
        sample_rate,
        bits,
        float,
        samples,
    }
}

fn check(name: &str) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let ncw_path = dir.join(format!("{name}.ncw"));
    let wav_path = dir.join(format!("{name}.wav"));

    let mut ncw = NcwReader::read(fs::File::open(&ncw_path).unwrap()).unwrap();
    let decoded = ncw.decode_samples().unwrap();
    let wav = read_wav(&wav_path);

    assert_eq!(ncw.header.channels, wav.channels, "{name}: channels");
    assert_eq!(
        ncw.header.sample_rate, wav.sample_rate,
        "{name}: sample rate"
    );
    assert_eq!(ncw.header.bits_per_sample, wav.bits, "{name}: bit depth");
    assert_eq!(
        ncw.sample_format == SampleFormat::Float,
        wav.float,
        "{name}: sample format"
    );

    let expected_len = ncw.header.num_samples as usize * ncw.header.channels as usize;
    assert_eq!(decoded.len(), expected_len, "{name}: decoded length");

    // Some reference WAVs were written padded to a whole block, so they may be
    // slightly longer than the NCW's declared length; never shorter.
    assert!(
        wav.samples.len() >= decoded.len(),
        "{name}: reference wav shorter than decoded output"
    );

    let mismatches: Vec<usize> = decoded
        .iter()
        .zip(&wav.samples)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();
    assert!(
        mismatches.is_empty(),
        "{name}: {} mismatched samples, first at index {} (got {}, expected {})",
        mismatches.len(),
        mismatches[0],
        decoded[mismatches[0]],
        wav.samples[mismatches[0]]
    );
}

#[test]
fn decode_16_bit_mono() {
    check("16-bit-mono");
}

#[test]
fn decode_16_bit_stereo() {
    check("16-bit-stereo");
}

#[test]
fn decode_24_bit_mono() {
    check("24-bit-mono");
}

#[test]
fn decode_32_bit_mono_float() {
    check("32-bit-mono-float");
}

#[test]
fn decode_32_bit_stereo_float() {
    check("32-bit-stereo-float");
}

#[test]
fn decode_onezero_single_block() {
    check("testfile-onezero-16-bit-stereo");
}

#[test]
fn decode_onezero_multiblock() {
    check("testfile-onezero-16-bit-stereo-multiblock");
}

#[test]
fn decode_24_bit_stereo_has_no_reference_but_decodes() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/24-bit-stereo.ncw");
    let mut ncw = NcwReader::read(fs::File::open(path).unwrap()).unwrap();
    let samples = ncw.decode_samples().unwrap();
    assert_eq!(
        samples.len(),
        ncw.header.num_samples as usize * ncw.header.channels as usize
    );
}

#[test]
fn truncated_file_is_an_error_not_a_panic() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/16-bit-stereo.ncw");
    let bytes = fs::read(path).unwrap();
    let cut = &bytes[..bytes.len() / 2];
    assert!(
        NcwReader::read(std::io::Cursor::new(cut))
            .and_then(|mut reader| reader.decode_samples())
            .is_err()
    );
}

// ---------------------------------------------------------------------------
// Synthetic fixtures. No real file in `tests/data` uses mid/side or raw
// (`bits == 0`) blocks, so those paths are covered with NCW files built here.
// ---------------------------------------------------------------------------

mod synth {
    use ncw::SAMPLES_PER_BLOCK;

    pub const MID_SIDE: u16 = 0b01;
    pub const FLOAT: u16 = 0b10;

    /// Pack signed values LSB-first at `bits` per value, as the format does.
    pub fn pack(values: &[i32], bits: usize) -> Vec<u8> {
        assert_eq!(values.len(), SAMPLES_PER_BLOCK);
        let mut out = Vec::new();
        let mut acc: u64 = 0;
        let mut have = 0;
        for &v in values {
            acc |= ((v as u32 as u64) & ((1u64 << bits) - 1)) << have;
            have += bits;
            while have >= 8 {
                out.push(acc as u8);
                acc >>= 8;
                have -= 8;
            }
        }
        assert_eq!(have, 0);
        out
    }

    /// One channel's sub-block: 16-byte header followed by the body.
    pub fn block(base: i32, bits: i16, flags: u16, body: &[u8]) -> Vec<u8> {
        let mut b = 0x160C9A3Eu32.to_be_bytes().to_vec();
        b.extend(base.to_le_bytes());
        b.extend(bits.to_le_bytes());
        b.extend(flags.to_le_bytes());
        b.extend([0u8; 4]); // reserved, always zero
        b.extend(body);
        b
    }

    /// Assemble a whole file. Each entry of `blocks` is one block: the
    /// concatenated per-channel sub-blocks.
    pub fn file(
        channels: u16,
        bits_per_sample: u16,
        num_samples: u32,
        blocks: &[Vec<u8>],
    ) -> Vec<u8> {
        let blocks_offset = 120u32;
        let data_offset = blocks_offset + 4 * (blocks.len() as u32 + 1);
        let data_size: u32 = blocks.iter().map(|b| b.len() as u32).sum();

        let mut f = 0x01A89ED631010000u64.to_be_bytes().to_vec();
        f.extend(channels.to_le_bytes());
        f.extend(bits_per_sample.to_le_bytes());
        f.extend(48000u32.to_le_bytes());
        f.extend(num_samples.to_le_bytes());
        f.extend(blocks_offset.to_le_bytes());
        f.extend(data_offset.to_le_bytes());
        f.extend(data_size.to_le_bytes());
        f.resize(blocks_offset as usize, 0);

        let mut offset = 0u32;
        for b in blocks {
            f.extend(offset.to_le_bytes());
            offset += b.len() as u32;
        }
        f.extend(data_size.to_le_bytes());
        for b in blocks {
            f.extend(b);
        }
        f
    }
}

type MemReader = NcwReader<std::io::Cursor<Vec<u8>>>;

fn decode(bytes: Vec<u8>) -> Result<(MemReader, Vec<i32>), ncw::NcwError> {
    let mut ncw = NcwReader::read(std::io::Cursor::new(bytes))?;
    let samples = ncw.decode_samples()?;
    Ok((ncw, samples))
}

fn ramp(offset: i32) -> Vec<i32> {
    (0..ncw::SAMPLES_PER_BLOCK as i32)
        .map(|i| i * 7 - 1000 + offset)
        .collect()
}

#[test]
fn mid_side_pcm_blocks_are_converted_to_left_right() {
    let mid = ramp(0);
    let side = ramp(300);
    let file = synth::file(
        2,
        16,
        512,
        &[[
            synth::block(0, -16, synth::MID_SIDE, &synth::pack(&mid, 16)),
            synth::block(0, -16, synth::MID_SIDE, &synth::pack(&side, 16)),
        ]
        .concat()],
    );
    let (_, samples) = decode(file).unwrap();
    for i in 0..512 {
        assert_eq!(samples[2 * i], mid[i] + side[i], "left at {i}");
        assert_eq!(samples[2 * i + 1], mid[i] - side[i], "right at {i}");
    }
}

#[test]
fn mid_side_is_applied_per_block() {
    // First block plain left/right, second block mid/side; both must decode.
    let a = ramp(0);
    let b = ramp(5);
    let plain = [
        synth::block(0, -16, 0, &synth::pack(&a, 16)),
        synth::block(0, -16, 0, &synth::pack(&b, 16)),
    ]
    .concat();
    let ms = [
        synth::block(0, -16, synth::MID_SIDE, &synth::pack(&a, 16)),
        synth::block(0, -16, synth::MID_SIDE, &synth::pack(&b, 16)),
    ]
    .concat();
    let (_, samples) = decode(synth::file(2, 16, 1024, &[plain, ms])).unwrap();
    for i in 0..512 {
        assert_eq!(samples[2 * i], a[i]);
        assert_eq!(samples[2 * i + 1], b[i]);
        assert_eq!(samples[1024 + 2 * i], a[i] + b[i]);
        assert_eq!(samples[1024 + 2 * i + 1], a[i] - b[i]);
    }
}

#[test]
fn mid_side_float_blocks_use_float_arithmetic() {
    let mid: Vec<i32> = (0..512)
        .map(|i| (i as f32 / 1024.0).to_bits() as i32)
        .collect();
    let side: Vec<i32> = (0..512).map(|_| 0.125f32.to_bits() as i32).collect();
    let flags = synth::MID_SIDE | synth::FLOAT;
    let file = synth::file(
        2,
        32,
        512,
        &[[
            synth::block(0, -32, flags, &synth::pack(&mid, 32)),
            synth::block(0, -32, flags, &synth::pack(&side, 32)),
        ]
        .concat()],
    );
    let (ncw, samples) = decode(file).unwrap();
    assert_eq!(ncw.sample_format, SampleFormat::Float);
    for i in 0..512 {
        let m = i as f32 / 1024.0;
        assert_eq!(f32::from_bits(samples[2 * i] as u32), m + 0.125);
        assert_eq!(f32::from_bits(samples[2 * i + 1] as u32), m - 0.125);
    }
}

#[test]
fn mid_side_on_mono_is_an_error() {
    let file = synth::file(
        1,
        16,
        512,
        &[synth::block(
            0,
            -16,
            synth::MID_SIDE,
            &synth::pack(&ramp(0), 16),
        )],
    );
    assert!(matches!(decode(file), Err(ncw::NcwError::InvalidHeader(_))));
}

#[test]
fn raw_blocks_decode_at_native_bit_depth() {
    for (bits_per_sample, values) in [
        (
            8u16,
            (0..512).map(|i| (i % 256) - 128).collect::<Vec<i32>>(),
        ),
        (16, ramp(0)),
        (24, (0..512).map(|i| i * 30000 - 8_000_000).collect()),
        (
            32,
            (0..512).map(|i| i * 4_000_000 - 1_000_000_000).collect(),
        ),
    ] {
        let body: Vec<u8> = values
            .iter()
            .flat_map(|v| v.to_le_bytes()[..bits_per_sample as usize / 8].to_vec())
            .collect();
        let file = synth::file(1, bits_per_sample, 512, &[synth::block(0, 0, 0, &body)]);
        let (_, samples) = decode(file).unwrap();
        assert_eq!(samples, values, "{bits_per_sample}-bit raw block");
    }
}

#[test]
fn delta_blocks_accumulate_from_base() {
    let deltas: Vec<i32> = (0..512).map(|i| (i % 9) - 4).collect();
    let file = synth::file(
        1,
        16,
        512,
        &[synth::block(1000, 4, 0, &synth::pack(&deltas, 4))],
    );
    let (_, samples) = decode(file).unwrap();
    let mut expected = Vec::new();
    let mut cur = 1000;
    for d in deltas {
        expected.push(cur);
        cur += d;
    }
    assert_eq!(samples, expected);
}

#[test]
fn sample_format_change_between_blocks_is_an_error() {
    let file = synth::file(
        1,
        16,
        1024,
        &[
            synth::block(0, -16, 0, &synth::pack(&ramp(0), 16)),
            synth::block(0, -16, synth::FLOAT, &synth::pack(&ramp(0), 16)),
        ],
    );
    assert!(matches!(decode(file), Err(ncw::NcwError::InvalidHeader(_))));
}

#[test]
fn invalid_bits_per_sample_is_rejected_on_read() {
    let file = synth::file(
        1,
        12,
        512,
        &[synth::block(0, -16, 0, &synth::pack(&ramp(0), 16))],
    );
    assert!(matches!(
        NcwReader::read(std::io::Cursor::new(file)),
        Err(ncw::NcwError::InvalidHeader(_))
    ));
}

#[test]
fn corrupt_tables_and_headers_are_rejected() {
    let block = synth::block(10, 2, 0, &synth::pack(&vec![0; 512], 2));
    let good = synth::file(1, 16, 1024, &[block.clone(), block]);
    for (offset, value) in [
        (124, 0u32), // duplicated offset
        (120, 144),  // nonzero first offset
        (124, 400),  // beyond data
        (128, 0),    // corrupt sentinel
        (28, 0),     // corrupt data size
        (20, 116),   // table overlaps header
        (24, 133),   // misaligned table
        (16, 512),   // table count differs from frames
        (12, 0),     // invalid sample rate
    ] {
        let mut bad = good.clone();
        bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(decode(bad).is_err(), "offset {offset}, value {value}");
    }
    // A decreasing offset must also fail even when all entries are within data.
    let block = synth::block(10, 2, 0, &synth::pack(&vec![0; 512], 2));
    let mut bad = synth::file(1, 16, 1536, &[block.clone(), block.clone(), block]);
    bad[124..128].copy_from_slice(&288u32.to_le_bytes());
    bad[128..132].copy_from_slice(&144u32.to_le_bytes());
    assert!(decode(bad).is_err());
}

#[test]
fn tiny_file_cannot_request_large_channel_buffers() {
    let block = synth::block(0, 2, 0, &synth::pack(&vec![0; 512], 2));
    let mut bad = synth::file(u16::MAX, 16, 8192, &[block]);
    // Sixteen offsets pointing at one mono payload, despite 65535 channels.
    bad.splice(124..124, [0; 60]);
    bad[24..28].copy_from_slice(&188u32.to_le_bytes());
    assert_eq!(bad.len(), 332);
    assert!(NcwReader::read(std::io::Cursor::new(bad)).is_err());

    // Even a single correctly ordered offset cannot claim absent channels.
    let block = synth::block(0, 2, 0, &synth::pack(&vec![0; 512], 2));
    assert!(decode(synth::file(u16::MAX, 16, 512, &[block])).is_err());
}

#[test]
fn channel_payloads_cannot_cross_group_boundaries() {
    let block = synth::block(10, 2, 0, &synth::pack(&vec![0; 512], 2));
    let mut bad = synth::file(1, 16, 1024, &[block.clone(), block]);
    bad[124..128].copy_from_slice(&80u32.to_le_bytes());
    assert!(decode(bad).is_err());

    let mut extra = synth::block(10, 2, 0, &synth::pack(&vec![0; 512], 2));
    extra.push(0);
    assert!(decode(synth::file(1, 16, 512, &[extra])).is_err());
}

#[test]
fn empty_audio_prefix_and_trailing_bytes_remain_supported() {
    let (_, samples) = decode(synth::file(1, 16, 0, &[])).unwrap();
    assert!(samples.is_empty());
    let block = synth::block(10, 2, 0, &synth::pack(&vec![0; 512], 2));
    let mut file = synth::file(1, 16, 1, &[block]);
    file.splice(120..120, [7; 3]);
    file[20..24].copy_from_slice(&123u32.to_le_bytes());
    file[24..28].copy_from_slice(&131u32.to_le_bytes());
    file.extend([9; 3]);
    assert_eq!(decode(file).unwrap().1, vec![10]);
}

#[test]
fn edited_public_reader_fields_are_revalidated() {
    let block = synth::block(10, 2, 0, &synth::pack(&vec![0; 512], 2));
    let file = synth::file(1, 16, 512, &[block]);
    let mut reader = NcwReader::read(std::io::Cursor::new(file.clone())).unwrap();
    reader.header.channels = 0;
    assert!(reader.decode_samples().is_err());
    let mut reader = NcwReader::read(std::io::Cursor::new(file)).unwrap();
    reader.block_offsets.clear();
    assert!(reader.decode_samples().is_err());
}
