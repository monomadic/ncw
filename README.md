# Native Instruments NCW Audio File Format

<p>
<a href="https://crates.io/crates/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/crates/v/ncw.svg" alt="crates.io"></a>
<a href="https://docs.rs/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/docsrs/ncw" alt="docs.rs"></a>
</p>

NCW (Native Instruments Compressed Wave) is the lossless audio container used by Kontakt libraries. It is essentially DPCM plus bit truncation, with optional mid/side stereo. This repository is part of a [wider reverse engineering effort](https://github.com/open-sound) of proprietary audio formats and backs the NCW support in [ni-file](https://github.com/monomadic/ni-file).

| Crate | What it is |
|---|---|
| [`crates/ncw`](crates/ncw) | Zero-dependency decoder library ([docs.rs](https://docs.rs/ncw)) |
| [`crates/ncw-convert`](crates/ncw-convert) | `ncw-convert <INPUT> <OUTPUT>` command-line NCW to WAV converter |

Format notes, including what is known and what is still guessed, live in [FORMAT.md](FORMAT.md). Release history is in [CHANGELOG.md](CHANGELOG.md).

## Quick start

```bash
cargo install ncw-convert
ncw-convert sample.ncw sample.wav
```

Library usage is documented in the [crate README](crates/ncw/README.md).

## Help wanted

The decoder is verified sample-for-sample against reference WAVs for every fixture in `crates/ncw/tests/data`, but those fixtures only exercise part of the format. Several code paths are currently covered by synthetic files that follow the documented formulas rather than by real Kontakt output. If you can produce any of the following from a library you own, please open an issue or pull request with the `.ncw` and, where possible, the original `.wav` it was made from:

1. **A mid/side encoded stereo file.** Block header flag bit 0. This is the most important gap: the left = mid + side, right = mid − side reconstruction has never been checked against real data. Wide stereo material such as pads or reverb tails is the most likely to trigger this encoding.
2. **A reference WAV for `24-bit-stereo.ncw`.** The file decodes, but with no original to compare against its test only checks the sample count.
3. **A file with uncompressed blocks.** Block header `bits == 0`, meaning samples are stored raw at the file's bit depth. Noise or very dense material is the most likely source.
4. **An 8-bit file**, if Kontakt can produce one at all.
5. **A file with more than two channels**, to learn how (or whether) mid/side and block layout apply beyond stereo.
6. **A file with the alternate signature** `01 A8 9E D6 30 01 00 00`. Both signatures are accepted, but only `31` has been seen.
7. **Blocks truncated to a width of 8 or more that is not a multiple of 8**, for example 12-bit. The unpacker handles any width, but real files have only shown widths under 8 or exactly 8, 16, 24, 32.

Fixtures do not need to be long: a few thousand samples is enough, and the decoder pads the last block anyway.

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

The minimum supported Rust version is 1.85 (edition 2024) and is checked in CI.

## License

MIT OR Apache-2.0
