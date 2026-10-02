# Native Instruments NCW Audio File Format

<p>
<a href="https://crates.io/crates/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/crates/v/ncw.svg" alt="crates.io"></a>
<a href="https://docs.rs/ncw" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/docsrs/ncw" alt="docs.rs"></a>
</p>

NCW (Native Instruments Compressed Wave) is the lossless audio container used by Kontakt libraries. It is essentially DPCM plus bit truncation, with optional mid/side stereo.

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

The decoder is verified sample-for-sample against the paired reference WAVs in `crates/ncw/tests/data`, and against Kontakt's own output on private commercial material. Several cases are still covered only by synthetic files, or not at all. The list of files that would help, and what each would settle, is in [FORMAT.md](FORMAT.md#wanted). The most useful is a mid/side encoded file that can be redistributed as a test fixture.

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
