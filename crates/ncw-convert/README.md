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

## Encoding (working-tree version)

```sh
ncw-convert encode input.wav output.ncw
ncw-convert encode input.wav output.ncw --mode direct
ncw-convert encode input.wav output.ncw --mode mid-side
ncw-convert roundtrip original.ncw rebuilt.ncw
ncw-convert encode decoded.wav rebuilt.ncw --template original.ncw
```

The legacy two-argument command still decodes NCW to WAV; `decode INPUT OUTPUT`
is also accepted. Existing outputs are never overwritten. Build this checkout
with `cargo build -p ncw-convert`; these commands are not claimed to be published.

Writing supports mono/stereo integer PCM16/24 only. Automatic mode chooses a
smaller exactly representable mid/side encoding, otherwise direct channels.
Forced mid/side rejects opposite-parity L/R pairs rather than rounding.

`roundtrip` decodes to PCM and regenerates the NCW using original encoding metadata,
then requires byte identity before writing. `encode --template` does the same
reconstruction from a supplied WAV; parameters and deltas must fit the template.
It does not copy active compressed payloads. The template supplies otherwise lost
headers, block choices, terminal deltas and tail padding. Ordinary fresh encoding
promises lossless PCM, not byte identity. Float/8-bit/32-bit writing is unsupported.
