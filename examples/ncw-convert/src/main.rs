use std::{
    error::Error,
    fs::File,
    io::{Read, Seek, Write},
};

use hound::{WavSpec, WavWriter};
use ncw::{NcwReader, SampleFormat};

pub fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: ncw-convert <INPUT> <OUTPUT>");
        std::process::exit(2);
    }

    let input = File::open(&args[1])?;
    let mut output = File::create(&args[2])?;

    write_wav(&mut NcwReader::read(input)?, &mut output)?;

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
            _ => return Err(format!("unsupported sample format: {bits}-bit float={is_float}").into()),
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
            .join("../../tests/data")
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
