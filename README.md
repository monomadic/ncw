# Native Instruments NCW Audio File Format

<p>
<a href="https://crates.io/crates/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/crates/v/ncw.svg" alt="crates.io"></a>
<a href="https://docs.rs/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/docsrs/ncw" alt="docs.rs"></a>
</p>

## Description

NCW (Native Instruments Compressed Wave) is a lossless compression algorithm developed by Native Instruments which is essentially DPCM and bit truncation.

This library is a zero-dependency Rust-based library to decode NCW files. It serves as part of a [wider reverse engineering effort](https://github.com/open-sound) of proprietary audio formats, and this particular library is used in [ni-file](https://github.com/monomadic/ni-file), a library for Native Instruments file formats support in rust.

This repository also includes an ncw to wav conversion cli tool, `ncw-convert`.

## Requirements

- Rust 1.85 or higher (edition 2024)

## Usage

```rust,no_run
use ncw::{NcwReader, SampleFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::fs::File::open("sample.ncw")?;
    let mut ncw = NcwReader::read(input)?;

    println!("channels: {}", ncw.header.channels);
    println!("sample_rate: {}", ncw.header.sample_rate);
    println!("bits_per_sample: {}", ncw.header.bits_per_sample);

    // Samples are interleaved i32 in plain channel order; mid/side encoded
    // blocks are converted to left/right for you. PCM files give
    // sign-extended integers at the file's bit depth; float files give raw
    // f32 bit patterns.
    for sample in ncw.decode_samples()? {
        match ncw.sample_format {
            SampleFormat::Pcm => println!("{sample}"),
            SampleFormat::Float => println!("{}", f32::from_bits(sample as u32)),
        }
    }
    Ok(())
}
```

## Utility (ncw-convert)

To install the cli utility, you can use cargo:

```bash
cargo install ncw-convert
```

### Usage

Run the program with the following command-line arguments:

```bash
ncw-convert <INPUT> <OUTPUT>
```

- `<INPUT>`: Path to the input NCW file.
- `<OUTPUT>`: Path where the output WAV file will be saved.

## Contribution

To contribute, create a pull request with your proposed changes.
