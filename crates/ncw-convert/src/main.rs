use std::{
    error::Error,
    fs::File,
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
};

use hound::{WavSpec, WavWriter};
use ncw::{NcwReader, SampleFormat};

fn usage() -> &'static str {
    "usage:\n  \
     ncw-convert <INPUT> [OUTPUT] [OPTIONS]\n  \
     ncw-convert decode <INPUT.ncw> [OUTPUT.wav]\n  \
     ncw-convert encode <INPUT.wav> [OUTPUT.ncw] [OPTIONS]\n  \
     ncw-convert roundtrip <INPUT.ncw> [OUTPUT.ncw]\n\n\
     options:\n  \
     --mode auto|direct|mid-side   stereo transform for fresh encoding (default auto)\n  \
     --template ORIGINAL.ncw       rebuild using an existing file's encoding metadata\n  \
     --help, --version\n\n\
     OUTPUT defaults to INPUT with the extension replaced (.wav or .ncw); roundtrip \
     defaults to INPUT.roundtrip.ncw. Existing outputs are never overwritten."
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Ncw,
    Wav,
}

/// Identify a file by its leading bytes, falling back to the extension.
fn detect_kind(path: &Path) -> Result<Kind, Box<dyn Error>> {
    let mut magic = [0u8; 4];
    let read = File::open(path)?.read(&mut magic)?;
    match &magic[..read] {
        [0x01, 0xA8, 0x9E, 0xD6] => return Ok(Kind::Ncw),
        b"RIFF" | b"RF64" => return Ok(Kind::Wav),
        _ => {}
    }
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("ncw") => Ok(Kind::Ncw),
        Some("wav" | "wave") => Ok(Kind::Wav),
        _ => Err(format!("cannot tell whether {} is NCW or WAV", path.display()).into()),
    }
}

struct Options {
    mode: Option<String>,
    template: Option<String>,
}

/// Split arguments into positionals and the recognised `--flag VALUE` options.
fn parse_args(args: &[String]) -> (Vec<&str>, Options) {
    let mut positional = Vec::new();
    let mut options = Options {
        mode: None,
        template: None,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--mode" | "--template" => {
                let Some(value) = iter.next() else {
                    usage_error()
                };
                let slot = if arg == "--mode" {
                    &mut options.mode
                } else {
                    &mut options.template
                };
                if slot.replace(value.clone()).is_some() {
                    usage_error()
                }
            }
            flag if flag.starts_with('-') && flag.len() > 1 => usage_error(),
            _ => positional.push(arg.as_str()),
        }
    }
    (positional, options)
}

fn parse_mode(mode: Option<&str>) -> ncw::StereoMode {
    match mode {
        None | Some("auto") => ncw::StereoMode::Auto,
        Some("direct") => ncw::StereoMode::Direct,
        Some("mid-side") => ncw::StereoMode::MidSide,
        Some(_) => usage_error(),
    }
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
    let (positional, options) = parse_args(&args);
    let (command, input, output) = match positional.as_slice() {
        [command, input, rest @ ..]
            if matches!(*command, "encode" | "decode" | "roundtrip") && rest.len() <= 1 =>
        {
            (*command, *input, rest.first().copied())
        }
        [input, rest @ ..] if rest.len() <= 1 => {
            let command = match detect_kind(Path::new(input))? {
                Kind::Ncw => "decode",
                Kind::Wav => "encode",
            };
            (command, *input, rest.first().copied())
        }
        _ => usage_error(),
    };
    let has_encode_options = options.mode.is_some() || options.template.is_some();
    if command != "encode" && has_encode_options {
        usage_error()
    }
    if options.mode.is_some() && options.template.is_some() {
        usage_error()
    }
    let output = match output {
        Some(path) => PathBuf::from(path),
        None => match command {
            "decode" => Path::new(input).with_extension("wav"),
            "encode" => Path::new(input).with_extension("ncw"),
            _ => Path::new(input).with_extension("roundtrip.ncw"),
        },
    };
    // Complete decoding/encoding before creating an output; never overwrite an input or existing file.
    let mut message = None;
    let result = match command {
        "decode" => {
            let mut reader = NcwReader::read(File::open(input)?)?;
            let mut out = std::io::Cursor::new(Vec::new());
            write_wav(&mut reader, &mut out)?;
            out.into_inner()
        }
        "encode" => {
            let mode = parse_mode(options.mode.as_deref());
            let (samples, spec) = read_pcm(input)?;
            match &options.template {
                None => ncw::encode_pcm(&samples, spec, mode)?,
                Some(path) => ncw::encode_pcm_with_template(&samples, spec, &std::fs::read(path)?)?,
            }
        }
        _ => {
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
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .map_err(|e| format!("cannot create {}: {e}", output.display()))?;
    file.write_all(&result)?;
    println!("wrote {}", output.display());
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

    // Hound computes these fields with unchecked arithmetic. Reject invalid
    // parameters before it writes anything (including for empty audio).
    if spec.channels == 0
        || spec.sample_rate == 0
        || !matches!(bits, 8 | 16 | 24 | 32)
        || (is_float && bits != 32)
    {
        return Err("unsupported WAV sample format or zero channels/sample rate".into());
    }
    let block_align = spec
        .channels
        .checked_mul(bits / 8)
        .ok_or("WAV block alignment exceeds 16 bits")?;
    spec.sample_rate
        .checked_mul(u32::from(block_align))
        .ok_or("WAV byte rate exceeds 32 bits")?;
    // Hound uses 44-byte PCM or 68-byte extensible headers, including RIFF's
    // eight-byte prefix. Its RIFF and data lengths are both limited to u32.
    let overhead = if spec.channels > 2 || bits > 16 {
        60
    } else {
        36
    };
    reader
        .header
        .num_samples
        .checked_mul(u32::from(block_align))
        .and_then(|bytes| bytes.checked_add(overhead))
        .ok_or("audio exceeds the RIFF WAV size limit")?;

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
    use std::io::Cursor;

    // Original synthetic raw blocks keep the CLI package's tests self-contained.
    // Codec correctness against reference WAVs is covered by the library tests.
    fn fixture(channels: u16, bits: u16, float: bool, samples: &[i32]) -> Vec<u8> {
        let frames = samples.len() / channels as usize;
        assert!((1..=512).contains(&frames));
        assert_eq!(samples.len() % channels as usize, 0);
        let mut data = Vec::new();
        for channel in 0..channels as usize {
            data.extend([0x16, 0x0c, 0x9a, 0x3e]);
            data.extend(0i32.to_le_bytes());
            data.extend((-(bits as i16)).to_le_bytes());
            data.extend((if float { 2u16 } else { 0 }).to_le_bytes());
            data.extend([0; 4]);
            for frame in 0..512 {
                let sample = samples[frame.min(frames - 1) * channels as usize + channel];
                data.extend(&sample.to_le_bytes()[..bits as usize / 8]);
            }
        }
        let mut bytes = vec![0; 120];
        bytes[..8].copy_from_slice(&[1, 0xa8, 0x9e, 0xd6, 0x31, 1, 0, 0]);
        bytes[8..10].copy_from_slice(&channels.to_le_bytes());
        bytes[10..12].copy_from_slice(&bits.to_le_bytes());
        for (offset, value) in [
            (12, 48000),
            (16, frames as u32),
            (20, 120),
            (24, 128),
            (28, data.len() as u32),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes.extend(0u32.to_le_bytes());
        bytes.extend((data.len() as u32).to_le_bytes());
        bytes.extend(data);
        bytes
    }

    fn convert(bytes: Vec<u8>) -> Vec<u8> {
        let mut ncw = NcwReader::read(Cursor::new(bytes)).unwrap();
        let mut buffer = Cursor::new(Vec::new());
        write_wav(&mut ncw, &mut buffer).unwrap();
        buffer.into_inner()
    }

    #[test]
    fn converts_16_bit_mono() {
        let samples = [-32768, -1, 0, 32767];
        let wav = convert(fixture(1, 16, false, &samples));
        let mut reader = hound::WavReader::new(Cursor::new(wav)).unwrap();
        assert_eq!(reader.spec().bits_per_sample, 16);
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Int);
        assert_eq!(
            reader
                .samples::<i32>()
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            samples
        );
    }

    #[test]
    fn converts_24_bit_stereo() {
        let samples = [-8388608, 8388607, -1, 1, 0, 123456];
        let wav = convert(fixture(2, 24, false, &samples));
        let mut reader = hound::WavReader::new(Cursor::new(wav)).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().bits_per_sample, 24);
        assert_eq!(
            reader
                .samples::<i32>()
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            samples
        );
    }

    #[test]
    fn converts_32_bit_float_as_float() {
        let samples = [-1.0f32, -0.125, 0.0, 0.75, 1.0];
        let bits: Vec<i32> = samples.iter().map(|s| s.to_bits() as i32).collect();
        let wav = convert(fixture(1, 32, true, &bits));
        let mut reader = hound::WavReader::new(Cursor::new(wav)).unwrap();
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
        assert_eq!(
            reader
                .samples::<f32>()
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            samples
        );
    }

    #[test]
    fn invalid_wav_parameters_fail_before_writing() {
        for (channels, rate, frames) in [
            (1, 0, 1),
            (1, u32::MAX, 1),
            (u16::MAX, 1, 1),
            (1, 48000, u32::MAX),
        ] {
            let mut reader = NcwReader::read(Cursor::new(fixture(1, 16, false, &[0]))).unwrap();
            reader.header.channels = channels;
            reader.header.sample_rate = rate;
            reader.header.num_samples = frames;
            let mut output = Cursor::new(Vec::new());
            assert!(write_wav(&mut reader, &mut output).is_err());
            assert!(output.into_inner().is_empty());
        }
    }

    #[test]
    fn highest_representable_wav_byte_rate_is_accepted() {
        let mut reader = NcwReader::read(Cursor::new(fixture(1, 16, false, &[42]))).unwrap();
        reader.header.sample_rate = u32::MAX / 2;
        let mut output = Cursor::new(Vec::new());
        write_wav(&mut reader, &mut output).unwrap();
        let bytes = output.into_inner();
        assert_eq!(
            u32::from_le_bytes(bytes[28..32].try_into().unwrap()),
            u32::MAX - 1
        );
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
