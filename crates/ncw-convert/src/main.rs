use std::{
    error::Error,
    fs::File,
    io::{Read, Seek, Write},
};

use hound::{WavSpec, WavWriter};
use ncw::{NcwReader, SampleFormat};

fn usage() -> &'static str {
    "usage:\n  ncw-convert <INPUT.ncw> <OUTPUT.wav>\n  ncw-convert decode <INPUT.ncw> <OUTPUT.wav>\n  ncw-convert encode <INPUT.wav> <OUTPUT.ncw> [--mode auto|direct|mid-side | --template ORIGINAL.ncw]\n  ncw-convert roundtrip <INPUT.ncw> <OUTPUT.ncw>"
}

// Kontakt 8.9 writes a 20-byte PCM fmt chunk with four zero extension bytes.
// Hound rejects this otherwise valid file. Normalize only that observed variant.
fn normalize_pcm_fmt(mut bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Ok(bytes);
    }
    let mut pos = 12usize;
    while pos + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into()?) as usize;
        let end = pos
            .checked_add(8)
            .and_then(|v| v.checked_add(size))
            .ok_or("WAV chunk overflow")?;
        if end > bytes.len() {
            return Err("truncated WAV chunk".into());
        }
        if &bytes[pos..pos + 4] == b"fmt "
            && size == 20
            && bytes[pos + 8..pos + 10] == [1, 0]
            && bytes[pos + 24..end] == [0, 0, 0, 0]
        {
            let riff_size = u32::from_le_bytes(bytes[4..8].try_into()?)
                .checked_sub(4)
                .ok_or("invalid RIFF size")?;
            bytes[pos + 4..pos + 8].copy_from_slice(&16u32.to_le_bytes());
            bytes.drain(pos + 24..end);
            bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
            return Ok(bytes);
        }
        pos = end.checked_add(size & 1).ok_or("WAV chunk overflow")?;
    }
    Ok(bytes)
}

fn read_pcm(path: &str) -> Result<(Vec<i32>, ncw::PcmSpec), Box<dyn Error>> {
    let bytes = normalize_pcm_fmt(std::fs::read(path)?)?;
    let mut reader = hound::WavReader::new(std::io::Cursor::new(bytes))?;
    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Int || !matches!(spec.bits_per_sample, 16 | 24) {
        return Err("writing currently supports integer PCM16/24 WAV only".into());
    }
    let samples = reader.samples::<i32>().collect::<Result<Vec<_>, _>>()?;
    Ok((
        samples,
        ncw::PcmSpec {
            channels: spec.channels,
            bits_per_sample: spec.bits_per_sample,
            sample_rate: spec.sample_rate,
        },
    ))
}

/// Print usage to stderr and exit with status 2, the conventional code for bad arguments.
fn usage_error() -> ! {
    eprintln!("{}", usage());
    std::process::exit(2)
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if matches!(flag.as_str(), "--help" | "-h") => {
            println!("{}", usage());
            return Ok(());
        }
        [flag] if matches!(flag.as_str(), "--version" | "-V") => {
            println!("ncw-convert {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }
    let (command, input, output, options) = match args.as_slice() {
        [command, input, output, options @ ..]
            if matches!(command.as_str(), "encode" | "decode" | "roundtrip") =>
        {
            (command.as_str(), input, output, options)
        }
        [input, output] => ("decode", input, output, &[][..]),
        _ => usage_error(),
    };
    // Complete decoding/encoding before creating an output; never overwrite an input or existing file.
    let mut message = None;
    let result = match command {
        "decode" if options.is_empty() => {
            let mut reader = NcwReader::read(File::open(input)?)?;
            let mut out = std::io::Cursor::new(Vec::new());
            write_wav(&mut reader, &mut out)?;
            out.into_inner()
        }
        "encode" => {
            let (samples, spec) = read_pcm(input)?;
            match options {
                [] => ncw::encode_pcm(&samples, spec, ncw::StereoMode::Auto)?,
                [flag, value] if flag == "--mode" => {
                    let mode = match value.as_str() {
                        "auto" => ncw::StereoMode::Auto,
                        "direct" => ncw::StereoMode::Direct,
                        "mid-side" => ncw::StereoMode::MidSide,
                        _ => usage_error(),
                    };
                    ncw::encode_pcm(&samples, spec, mode)?
                }
                [flag, path] if flag == "--template" => {
                    ncw::encode_pcm_with_template(&samples, spec, &std::fs::read(path)?)?
                }
                _ => usage_error(),
            }
        }
        "roundtrip" if options.is_empty() => {
            let original = std::fs::read(input)?;
            let mut reader = NcwReader::read(std::io::Cursor::new(&original))?;
            if reader.sample_format != SampleFormat::Pcm {
                return Err("roundtrip writer supports PCM16/24 only".into());
            }
            let spec = ncw::PcmSpec {
                channels: reader.header.channels,
                bits_per_sample: reader.header.bits_per_sample,
                sample_rate: reader.header.sample_rate,
            };
            let samples = reader.decode_samples()?;
            let rebuilt = ncw::encode_pcm_with_template(&samples, spec, &original)?;
            if rebuilt != original {
                return Err("roundtrip was not byte-identical; output not written".into());
            }
            message = Some(format!(
                "byte-identical: {} bytes, {} sample values; active PCM decoded and repacked, encoding metadata preserved",
                rebuilt.len(),
                samples.len()
            ));
            rebuilt
        }
        _ => usage_error(),
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(&result)?;
    if let Some(message) = message {
        println!("{message}");
    }
    Ok(())
}

pub fn write_wav<R: Read + Seek, W: Write + Seek>(
    reader: &mut NcwReader<R>,
    writer: &mut W,
) -> Result<(), Box<dyn Error>> {
    let is_float = reader.sample_format == SampleFormat::Float;
    let bits = reader.header.bits_per_sample;

    let spec = WavSpec {
        channels: reader.header.channels,
        sample_rate: reader.header.sample_rate,
        bits_per_sample: bits,
        sample_format: if is_float {
            hound::SampleFormat::Float
        } else {
            hound::SampleFormat::Int
        },
    };

    let mut writer = WavWriter::new(writer, spec)?;

    for sample in reader.decode_samples()? {
        match (is_float, bits) {
            (true, 32) => writer.write_sample(f32::from_bits(sample as u32))?,
            (false, 32) | (false, 24) => writer.write_sample(sample)?,
            (false, 16) => writer.write_sample(sample as i16)?,
            (false, 8) => writer.write_sample(sample as i8)?,
            _ => {
                return Err(
                    format!("unsupported sample format: {bits}-bit float={is_float}").into(),
                );
            }
        }
    }
    writer.finalize()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Cursor, path::Path};

    fn convert(name: &str) -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../ncw/tests/data")
            .join(name);
        let mut ncw = NcwReader::read(File::open(path).unwrap()).unwrap();
        let mut buffer = Cursor::new(Vec::new());
        write_wav(&mut ncw, &mut buffer).unwrap();
        buffer.into_inner()
    }

    #[test]
    fn converts_16_bit_mono() {
        let wav = convert("16-bit-mono.ncw");
        let reader = hound::WavReader::new(Cursor::new(wav)).unwrap();
        assert_eq!(reader.spec().bits_per_sample, 16);
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Int);
    }

    #[test]
    fn converts_24_bit_stereo() {
        let wav = convert("24-bit-stereo.ncw");
        let reader = hound::WavReader::new(Cursor::new(wav)).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().bits_per_sample, 24);
    }

    #[test]
    fn converts_32_bit_float_as_float() {
        let wav = convert("32-bit-mono-float.ncw");
        let mut reader = hound::WavReader::new(Cursor::new(wav)).unwrap();
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
        let samples: Vec<f32> = reader.samples::<f32>().map(|s| s.unwrap()).collect();
        assert!(samples.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
    }
}

#[cfg(test)]
mod pcm_fmt_tests {
    use super::*;
    #[test]
    fn accepts_kontakt_pcm_extension_without_changing_audio() {
        let mut out = std::io::Cursor::new(Vec::new());
        {
            let mut w = WavWriter::new(
                &mut out,
                WavSpec {
                    channels: 2,
                    sample_rate: 48000,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )
            .unwrap();
            w.write_sample(42i16).unwrap();
            w.write_sample(-13i16).unwrap();
            w.finalize().unwrap();
        }
        let standard = out.into_inner();
        let mut kontakt = standard.clone();
        kontakt[16..20].copy_from_slice(&20u32.to_le_bytes());
        kontakt.splice(36..36, [0; 4]);
        let size = (kontakt.len() - 8) as u32;
        kontakt[4..8].copy_from_slice(&size.to_le_bytes());
        assert_eq!(normalize_pcm_fmt(kontakt.clone()).unwrap(), standard);
        kontakt[36] = 1;
        assert_eq!(normalize_pcm_fmt(kontakt.clone()).unwrap(), kontakt);
    }
}
