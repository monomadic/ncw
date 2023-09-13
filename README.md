# Native Instruments NCW Audio File Format

<p>
<a href="https://crates.io/crates/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/crates/v/ncw.svg" alt="crates.io"></a>
<a href="https://docs.rs/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/docsrs/ncw" alt="docs.rs"></a>
</p>

## Description

NCW (Native Instruments Compressed Wave) is a lossless compression algorithm developed by Native Instruments which is essentially DPCM and bit truncation.

This library is a zero-dependency Rust-based library to decode NCW files. It serves as part of a wider reverse engineering effort of proprietary audio formats, and this particular library is used in [ni-file](https://github.com/monomadic/ni-file).

This repository also includes an ncw to wav conversion cli tool, `ncw-decode`.

## Requirements

- Rust 1.50 or higher

## Usage

```rust
let input = File::open(&args[1])?;
let mut output = File::create(&args[2])?;
let mut ncw = NcwReader::read(&input)?;

let spec = WavSpec {
		channels: ncw.header.channels,
		sample_rate: ncw.header.sample_rate,
		bits_per_sample: ncw.header.bits_per_sample,
		sample_format: hound::SampleFormat::Int,
};

for sample in ncw.decode_samples()? {
	// save or convert each sample into a file or stream
	dbg!(sample);
}
```

## Utility (ncw-convert)

To install the cli utility, install it from cargo:

```bash
cargo install ncw --package ncw-convert
```

### Usage

Run the program with the following command-line arguments:

```bash
ncw-decode <INPUT> <OUTPUT>
```

- `<INPUT>`: Path to the input NCW file.
- `<OUTPUT>`: Path where the output WAV file will be saved.

## Contribution

To contribute, create a pull request with your proposed changes.
