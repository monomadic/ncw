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
    assert_eq!(ncw.header.sample_rate, wav.sample_rate, "{name}: sample rate");
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
    let mut ncw = NcwReader::read(std::io::Cursor::new(cut)).unwrap();
    assert!(ncw.decode_samples().is_err());
}
