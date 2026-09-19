# Changelog

## 0.3.0 (unreleased)

### Breaking
* `NcwReader::reader` is private. Use `get_ref`, `get_mut`, or `into_inner`.
* `NcwError::ReadError` is gone; short reads surface as `NcwError::IoError` with
  `ErrorKind::UnexpectedEof`. `NcwError` is `#[non_exhaustive]`.
* Edition 2024, minimum supported Rust 1.85. The previously declared 1.58 was
  never buildable because the manifest used workspace inheritance.
* Packages moved to `crates/ncw` and `crates/ncw-convert`.

### Fixed
* First-channel flag selects mid/side, matching controlled Kontakt 8.9.0 probes.
* Mid/side encoded blocks are converted to left/right. Previously the stored
  mid and side channels were returned as if they were left and right.
* Header fields are validated on read: channel count, bits per sample, and
  offset ordering.
* Allocations are bounded by the block table rather than header fields, so a
  malformed header cannot request gigabytes up front.
* The sample format flag is checked on every block, not only the first.

### Added
* PCM16/24 mono/stereo writer, automatic/direct/forced mid-side modes, template-based
  byte-identical reconstruction and CLI `encode`/`roundtrip` commands.
* Independent Kontakt validation on three private commercial fixtures; evidence in
  `WRITER_VALIDATION.md`. One-bit delta encoding excluded after a failed live probe.
* CLI accepts Kontakt's observed 20-byte PCM fmt chunk and refuses existing outputs.
* `SAMPLES_PER_BLOCK` is exported.
* In-memory synthetic fixtures covering mid/side (PCM and float), raw
  `bits == 0` blocks at every depth, delta blocks, and several rejection cases.
* `FORMAT.md` with container notes and a list of open questions.
* CI: tests, clippy, rustfmt, and an MSRV build.

## 0.2.0
* Decode truncated, 32-bit, and block-aligned files correctly.
* Flags split into channel encoding and sample format.
* `ncw-convert` writes float WAVs for float sources.
