# Native Instruments NCW Audio File Format

<p>
<a href="https://crates.io/crates/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/crates/v/ncw.svg" alt="crates.io"></a>
<a href="https://docs.rs/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/docsrs/ncw" alt="docs.rs"></a>
</p>

NCW (Native Instruments Compressed Wave) is the lossless audio container used by Kontakt libraries. Audio is stored in blocks of 512 frames, each either delta coded at a reduced bit width or stored raw, with optional mid/side stereo.

| Crate | What it is |
|---|---|
| [`crates/ncw`](crates/ncw) | Zero-dependency decoder and PCM16/24 writer library ([docs.rs](https://docs.rs/ncw)) |
| [`crates/ncw-convert`](crates/ncw-convert) | Command-line NCW/WAV converter with byte-identical template roundtrips |

Format notes, including what is known, what is still open and what files would help, live in [FORMAT.md](FORMAT.md). Release history is in [CHANGELOG.md](CHANGELOG.md).

## Quick start

```bash
cargo install ncw-convert
ncw-convert sample.ncw          # decodes to sample.wav
ncw-convert sample.wav          # encodes PCM16/24 to sample.ncw
```

Library usage is documented in the [crate README](crates/ncw/README.md), and every CLI option in the [ncw-convert README](crates/ncw-convert/README.md).

## Decoding

The decoder handles mono and stereo files as 16 and 24-bit PCM or 32-bit float, including mid/side blocks. It is verified sample-for-sample against the reference WAVs in `crates/ncw/tests/data`, and PCM16/24 decoding has been checked against Kontakt's own output on commercial library files that cannot be published. Float mid/side and a few block types have no real example yet; see [FORMAT.md](FORMAT.md#unknowns).

## NCW mid/side is not lossless

Native Instruments describes NCW as lossless. Reading the encoder inside a Kontakt 4.2 executable shows that its mid/side mode is not. It stores `mid = (L + R) / 2` and `side = (L − R) / 2` with integer division and no parity check, so whenever `L + R` is odd, one channel decodes one least significant bit closer to zero than the original. That affects roughly half the frames in each mid/side block. Mid/side blocks are found in 94% of a 3,484-file scan of commercial library NCWs, while current Kontakt (8.9.0) never writes them.

Direct (left/right) blocks are lossless, and this crate's writer only uses mid/side where it is exact. Shipped commercial files confirm it: in the three examined files, odd `L + R` sums occur in 47–50% of frames in direct blocks but in none of 581,530 frames in mid/side blocks, which is only possible if those blocks dropped a bit. Details, addresses and the full set of Kontakt 4.2 encoder rules are in [FORMAT.md](FORMAT.md#kontakt-4s-encoder).

## Writing and roundtrips

```sh
ncw-convert input.wav output.ncw
ncw-convert roundtrip original.ncw rebuilt.ncw
ncw-convert decoded.wav rebuilt.ncw --template original.ncw
```

Writing supports mono and stereo integer PCM16/24. Float, 8-bit and 32-bit input is refused. By default mid/side is used only where it is exactly representable and smaller; `--mode direct` disables it, and `--mode mid-side` forces it, failing if any left/right pair has an odd sum. No clipping or rounding is performed. Existing output files are never overwritten.

Fresh encoding is deterministic and lossless and writes the same block format as Native Instruments' tools, but its choice of widths and mid/side is independent of theirs, so the bytes can differ from an original file. To reproduce an original file exactly, use template mode, which keeps the original's header, block widths, flags and padding and **rebuilds the audio itself from PCM**. Block widths 0 and 1 are never written, because their meaning is not understood. Validation against Kontakt is described in [WRITER_VALIDATION.md](WRITER_VALIDATION.md).

## Help wanted

Several parts of the format are covered only by synthetic tests, or not at all. The files that would help, and what each would settle, are listed in [FORMAT.md](FORMAT.md#wanted). The most useful is a mid/side encoded file that can be redistributed as a test fixture.

Such a file has to come from the wild, because the publicly available Kontakt does not seem to produce one. Every encode tried with Kontakt 8.9.0 (generated test signals, noise, and a decoded natural mid/side file, through both its scripting API and instrument saving) wrote only direct left/right blocks. Kontakt still decodes mid/side correctly, and in commercial libraries mid/side is the norm: of 3,484 standalone NCW files scanned from a set of commercial libraries, 3,292 (94%) contain at least one mid/side block. Older versions did write it: Kontakt 4.2's encoder chooses mid/side whenever it is smaller (see above). None of the 111 NCW files bundled with a standard Kontakt installation use mid/side either. The *Batch Compress* option in Kontakt's interface has not been tried.

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
```

The minimum supported Rust version is 1.85 (edition 2024) and is checked in CI. Release steps, including GitHub release binaries, are in [RELEASING.md](RELEASING.md).

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
