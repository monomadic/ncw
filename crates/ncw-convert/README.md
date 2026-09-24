# ncw-convert

<p>
<a href="https://crates.io/crates/ncw-convert" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/crates/v/ncw-convert.svg" alt="crates.io"></a>
</p>

## Description

NCW (Native Instruments Compressed Wave) is the lossless audio container used by
Kontakt libraries. It is essentially DPCM plus bit truncation, with optional
mid/side stereo.

This is a command-line frontend to the [ncw crate](https://crates.io/crates/ncw)
that decodes NCW files to standard WAV files, encodes PCM WAV files to NCW, and
performs byte-identical template roundtrips. It is part of a
[wider reverse engineering effort](https://github.com/open-sound) of proprietary
audio formats.

## Installation

```bash
cargo install ncw-convert
```

## Usage

```
ncw-convert <INPUT> [OUTPUT] [OPTIONS]
ncw-convert decode <INPUT.ncw> [OUTPUT.wav]
ncw-convert encode <INPUT.wav> [OUTPUT.ncw] [OPTIONS]
ncw-convert roundtrip <INPUT.ncw> [OUTPUT.ncw]
```

The bare form looks at the input's leading bytes (falling back to its extension)
and decodes an NCW file to WAV or encodes a WAV file to NCW. `OUTPUT` is
optional: it defaults to the input path with the extension replaced by `.wav`
or `.ncw`, and to `INPUT.roundtrip.ncw` for `roundtrip`. Existing output files
are never overwritten. `--help` prints usage and `--version` prints the
installed version.

### Decoding

```sh
ncw-convert sample.ncw              # writes sample.wav
ncw-convert sample.ncw out.wav
ncw-convert decode sample.ncw out.wav
```

PCM sources produce integer WAVs at the file's bit depth; float sources produce
32-bit float WAVs.

### Encoding

```sh
ncw-convert input.wav               # writes input.ncw
ncw-convert input.wav output.ncw --mode direct
ncw-convert encode input.wav output.ncw --mode mid-side
```

Writing supports mono/stereo integer PCM16/24 only. Automatic mode chooses a
smaller exactly representable mid/side encoding, otherwise direct channels.
Forced mid/side rejects opposite-parity L/R pairs rather than rounding. Kontakt's
20-byte PCM `fmt ` chunk variant is accepted. Fresh encoding promises lossless
PCM, not byte identity with NI's encoder.

### Template roundtrips

```sh
ncw-convert roundtrip original.ncw  # writes original.roundtrip.ncw
ncw-convert roundtrip original.ncw rebuilt.ncw
ncw-convert decoded.wav rebuilt.ncw --template original.ncw
```

`roundtrip` decodes to PCM and regenerates the NCW using the original's encoding
metadata, then requires byte identity before writing. `encode --template` does
the same reconstruction from a supplied WAV; parameters and deltas must fit the
template. Active compressed payloads are never copied: the template supplies
otherwise lost headers, block choices, terminal deltas and tail padding.

Float, 8-bit and 32-bit integer writing are unsupported. See the repository's
[WRITER_VALIDATION.md](https://github.com/monomadic/ncw/blob/master/WRITER_VALIDATION.md)
for independent Kontakt evidence and remaining limits.

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
