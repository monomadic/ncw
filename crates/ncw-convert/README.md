# Native Instruments NCW Audio File Format

<p>
<a href="https://crates.io/crates/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/crates/v/ncw.svg" alt="crates.io"></a>
<a href="https://docs.rs/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/docsrs/ncw" alt="docs.rs"></a>
</p>

## Description

NCW (Native Instruments Compressed Wave) is a lossless compression algorithm developed by Native Instruments which is essentially DPCM and bit truncation.

This is a cli frontend to the [ncw crate](https://github.com/monomadic/ncw) which decode NCW files into standard WAV files. It serves as part of a wider reverse engineering effort of proprietary audio formats, and this particular library is used in [ni-file](https://github.com/monomadic/ni-file).

## Installation

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
