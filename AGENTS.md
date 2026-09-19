# Working on ncw

This is the maintained Rust NCW library and CLI. Read README.md, FORMAT.md,
WRITER_VALIDATION.md and the relevant tests before changing codec behavior.
The sibling ni-file-reference contains format research; ni-file-sources contains
catalogs, probes and ignored private/vendor assets. The old ni-file is read-only
historical recovery input; do not extend it or import its history.

Keep the Rust library dependency-free. Run cargo test --workspace,
cargo clippy --workspace --all-targets -- -D warnings, cargo fmt --all --check,
and RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps.
Commit completed scoped changes locally on the active branch; publishing is separate.

Do not add commercial audio. Pin private fixture hashes and record original-build
uncertainty. A writer/reader self-roundtrip is insufficient: validate new encoding
choices against Kontakt PCM. Distinguish fresh encoding from template reconstruction;
template byte identity does not establish NI's encoding policy. Preserve failed live
probes as evidence, especially the unresolved width-one behavior.
