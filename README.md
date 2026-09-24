# Native Instruments NCW Audio File Format

<p>
<a href="https://crates.io/crates/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/crates/v/ncw.svg" alt="crates.io"></a>
<a href="https://docs.rs/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/docsrs/ncw" alt="docs.rs"></a>
</p>

NCW (Native Instruments Compressed Wave) is the lossless audio container used by Kontakt libraries. It is essentially DPCM plus bit truncation, with optional mid/side stereo. This repository is part of a [wider reverse engineering effort](https://github.com/open-sound) of proprietary audio formats with maintained research notes in the sibling `ni-file-reference` repository and private evidence/tooling in `ni-file-sources`. The historical `ni-file` repository is retired as an implementation target.

| Crate | What it is |
|---|---|
| [`crates/ncw`](crates/ncw) | Zero-dependency decoder and PCM16/24 writer library ([docs.rs](https://docs.rs/ncw)) |
| [`crates/ncw-convert`](crates/ncw-convert) | NCW/WAV conversion and byte-identical template roundtrips |

Format notes, including what is known and what is still guessed, live in [FORMAT.md](FORMAT.md). Release history is in [CHANGELOG.md](CHANGELOG.md).

## Quick start

```bash
cargo install ncw-convert
ncw-convert sample.ncw          # decodes to sample.wav
ncw-convert sample.wav          # encodes PCM16/24 to sample.ncw
```

Library usage is documented in the [crate README](crates/ncw/README.md).

## Help wanted

The decoder is verified sample-for-sample against the paired reference WAV fixtures in `crates/ncw/tests/data` (one NCW has no WAV), but those fixtures only exercise part of the format. Several code paths are currently covered by synthetic files that follow the documented formulas rather than by real Kontakt output. If you can produce any of the following from a library you own, please open an issue or pull request with the `.ncw` and, where possible, the original `.wav` it was made from:

1. **Redistributable mid/side fixtures with known PCM.** Three private commercial PCM16/24 files now agree with Kontakt decoding and round-trip byte-identically using the template writer. They cannot be committed as test assets. See [writer validation](WRITER_VALIDATION.md).
2. **A reference WAV for `24-bit-stereo.ncw`.** The file decodes, but with no original to compare against its test only checks the sample count.
3. **A real file with zero-width blocks.** The historical reader treats `bits == 0` as raw samples, but independent Kontakt tests did not validate that interpretation. The writer rejects it; semantics remain unresolved.
4. **An 8-bit file**, if Kontakt can produce one at all.
5. **A file with more than two channels**, to learn how (or whether) mid/side and block layout apply beyond stereo.
6. **A file with the alternate signature** `01 A8 9E D6 30 01 00 00`. Both signatures are accepted, but only `31` has been seen.
7. **Raw blocks at additional widths.** Repository fixtures contain negative widths −16 and −32; later live PCM24 probes also validate −24. Redistributable examples at other widths would extend coverage.

Fixtures do not need to be long: a few thousand samples is enough, and the decoder pads the last block anyway.

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
```

The minimum supported Rust version is 1.85 (edition 2024) and is checked in CI.

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

## Writing and roundtrips

```sh
ncw-convert input.wav output.ncw
ncw-convert roundtrip original.ncw rebuilt.ncw
ncw-convert decoded.wav rebuilt.ncw --template original.ncw
```

Writing currently supports mono/stereo integer PCM16/24. Auto mode uses mid/side
only when exactly representable and smaller. `--mode direct` disables it;
`--mode mid-side` requires it and rejects opposite-parity channel samples.
No clipping or rounding is performed. Output files must not already exist.

Byte identity requires preserving choices WAV does not contain. Template mode
retains headers, widths/flags, terminal deltas and padded samples, and **rebuilds
active audio from PCM**. Fresh encoding is deterministic but does not reproduce
all NI encoder choices. Float/8-bit/32-bit writing and one-bit delta semantics
remain unsupported. See [CLI usage](crates/ncw-convert/README.md) and
[validation/limits](WRITER_VALIDATION.md).

## Release checks

Run the development checks above and follow [RELEASING.md](RELEASING.md) to
verify both publication archives. Publish `ncw` before `ncw-convert`, whose
registry dependency must be available first.
