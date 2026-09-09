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
* Mid/side encoded blocks are converted to left/right. Previously the stored
  mid and side channels were returned as if they were left and right.
* Header fields are validated on read: channel count, bits per sample, and
  offset ordering.
* Allocations are bounded by the block table rather than header fields, so a
  malformed header cannot request gigabytes up front.
* The sample format flag is checked on every block, not only the first.

### Added
* `SAMPLES_PER_BLOCK` is exported.
* In-memory synthetic fixtures covering mid/side (PCM and float), raw
  `bits == 0` blocks at every depth, delta blocks, and several rejection cases.
* `FORMAT.md` with container notes and a list of open questions.
* CI: tests, clippy, rustfmt, and an MSRV build.

## 0.2.0
* Decode truncated, 32-bit, and block-aligned files correctly.
* Flags split into channel encoding and sample format.
* `ncw-convert` writes float WAVs for float sources.
