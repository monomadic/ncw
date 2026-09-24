# Releasing ncw

## Before publishing

Run the workspace checks from README.md, then:

```sh
./scripts/check-packages.sh
```

The script requires Bash, Cargo, tar and cmp. It packages both crates together,
extracts their actual publication archives into a temporary directory, verifies
that both license texts are included, and runs each package's tests. The CLI uses
an explicit local patch pointing to the extracted library, so this works before
the new library version exists on crates.io. Nothing is uploaded. For testing
uncommitted changes, pass `--allow-dirty`; release checks should use a clean tree.

This replaces Cargo's build-only verification with isolated package tests. Cargo
1.96.1 (Homebrew) produced an internal "no hash listed" error when verifying the
workspace's temporary registry during this review. The explicit local patch avoids
that temporary-registry verification path. It does not verify registry availability.

Keep the root and per-crate LICENSE-MIT / LICENSE-APACHE copies in sync; the script
checks this. The CLI's synthetic tests must remain independent of sibling workspace
paths. Reference-WAV and template byte-identity tests remain in the library package.

Confirm the MSRV job (Rust 1.85) and the Linux, macOS and Windows CI checks pass.
Codec changes require the independent Kontakt PCM validation described in
WRITER_VALIDATION.md; internal roundtrips alone are insufficient.

## Publish in dependency order

Publishing is a separate, explicitly authorized step. For version 0.4.0:

```sh
cargo publish -p ncw --dry-run
cargo publish -p ncw
```

Wait until `ncw` 0.4.0 is available from the crates.io index. Then verify and publish
the CLI against that registry dependency, without a local patch:

```sh
cargo publish -p ncw-convert --dry-run
cargo publish -p ncw-convert
```

A CLI dry run before the library is available will fail dependency resolution.
Inspect each dry-run result before performing the corresponding upload.
