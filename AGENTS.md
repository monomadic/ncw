# Working on ncw

Rust NCW (Native Instruments Compressed Wave) library and CLI. Two crates:
`crates/ncw` (dependency-free decoder and PCM16/24 writer) and
`crates/ncw-convert` (NCW/WAV converter built on it).

Read README.md, FORMAT.md, WRITER_VALIDATION.md and the relevant tests before
changing codec behaviour.

## Checks

Run all of these before committing:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
```

MSRV is 1.85 (edition 2024). Keep the `ncw` library free of dependencies.
Commit completed, scoped changes locally on the active branch. Pushing,
tagging and publishing are separate steps that need explicit approval; see
RELEASING.md.

## Documentation

- FORMAT.md is the format reference. Every claim states its evidence: the
  fixtures, a Kontakt probe, or an unverified formula. Open questions go under
  *Unknowns*; material we need goes under *Wanted*, and moves to *No longer
  needed* once obtained.
- `crates/ncw/tests/data/README.md` describes the fixtures. Recompute its
  statistics from the block headers when fixtures change.
- Research notes and evidence used here are kept privately outside this
  repository. Do not link to private repositories or local paths from tracked
  files.

## Evidence rules

- Do not add commercial audio. Fixtures must be redistributable.
- A writer/reader self-roundtrip is not validation. New encoding choices must
  be checked against PCM decoded by Kontakt.
- Template byte identity reproduces an original file; it says nothing about how
  Native Instruments' encoder chooses widths or flags. Keep fresh encoding and
  template reconstruction distinct.
- Record failed Kontakt probes as evidence, especially for the unresolved width
  0 and width 1 behaviour.
