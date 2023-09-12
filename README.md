# Native Instruments NCW Audio File Format

## Description

`ncw-decode` is a zero-dependency Rust-based library to decode NCW files. It also includes a wav unpack cli utility. It serves as part of a wider reverse engineering effort of proprietary audio formats, and this particular library is used in [ni-file](https://github.com/monomadic/ni-file).

NCW is a compression algorithm which is essentially DPCM and bit truncation.

## Requirements

- Rust 1.50 or higher

## Installation

To install the cli utility, install it from the example:

```bash
git clone https://github.com/monomadic/ncw.git
```

Compile the project:

```bash
cd ncw-decode
cargo build --release
```

## Usage

Run the program with the following command-line arguments:

```bash
ncw-decode <INPUT> <OUTPUT>
```

- `<INPUT>`: Path to the input NCW file.
- `<OUTPUT>`: Path where the output WAV file will be saved.

## Contribution

To contribute, create a pull request with your proposed changes.
