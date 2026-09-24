#!/usr/bin/env bash
# Test the actual publication archives without requiring an unpublished ncw on crates.io.
set -euo pipefail
cd "$(dirname "$0")/.."
package_check_dir=$(mktemp -d "${TMPDIR:-/tmp}/ncw-packages.XXXXXX")
trap 'rm -rf "$package_check_dir"' EXIT

# Package both together so Cargo can resolve their matching, unpublished versions.
# Explicit tests below replace Cargo's build-only verification.
cargo package --workspace --no-verify --target-dir "$package_check_dir/build" "$@"
mkdir "$package_check_dir/extracted"
for archive in "$package_check_dir/build/package/"*.crate; do
    tar -xzf "$archive" -C "$package_check_dir/extracted"
done
library_packages=("$package_check_dir/extracted/"ncw-[0-9]*)
cli_packages=("$package_check_dir/extracted/"ncw-convert-[0-9]*)
[[ ${#library_packages[@]} == 1 && ${#cli_packages[@]} == 1 ]]
for package in "${library_packages[0]}" "${cli_packages[0]}"; do
    cmp LICENSE-MIT "$package/LICENSE-MIT"
    cmp LICENSE-APACHE "$package/LICENSE-APACHE"
done
cargo test --manifest-path "${library_packages[0]}/Cargo.toml"
cargo test --manifest-path "${cli_packages[0]}/Cargo.toml" \
    --config "patch.crates-io.ncw.path=\"${library_packages[0]}\""
