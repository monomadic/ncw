# Writer validation — 2026-09-19

The maintained implementation is this repository. Original format research lives
in `../ni-file-reference`; private materials and research tools in `../ni-file-sources`.
These are separate from the historical, read-only `../ni-file`.

Three private natural NCWs were tested against the installed Kontakt 8.9.0 decoder.
Original authoring builds are unknown. Installation provenance is not independently
verified; executable SHA256:
`3d6eef9471668a603b564c6575380107eb0caee3349a1eba3165caf8013a7bce`.

| Source SHA256 | PCM | Frames | Template byte identity | Fresh Kontakt PCM agreement |
|---|---|---:|---|---|
| d94bd7d2bdb640c5375abce12461aa4da1032d1bf706d0090001950aa1263ecc | 24-bit stereo | 344410 | exact | exact |
| aaa4028d15fd2f511027f3c37506dca8d54917b2ef377265b10a7bf46a9e7517 | 16-bit stereo | 348087 | exact | exact |
| ce7f78f25c30dd4a2ef52830a8038e20eeb960980ef5249ba89e10ef3d7687a8 | 24-bit stereo | 37952 | exact | exact |

All are 48 kHz and contain flag-1 blocks. Byte identity was obtained both from
Rust-decoded PCM and from Kontakt-decoded WAV. The template writer reconstructs
active payloads; headers, encoding choices, terminal deltas and padded tails are
preserved. A sample-edit regression verifies that it does not simply copy payloads.
Six existing repository PCM fixtures also rebuild byte-identically in automated tests.

Fresh encoding without a template is not byte-identical, although all three file
sizes match the originals. One PCM24 output matches everything after the main
header; the other also differs in its final block. The PCM16 output uses different
block choices. Independent Kontakt decoding matches every active PCM value in all
three, with no normalization or clipping.

An earlier width-one-delta trial passed internal roundtrips but produced 2,158
mismatched PCM16 values when decoded by Kontakt. The writer now requires at least
two bits for newly encoded deltas, and rejects width-one template blocks. Width-one
semantics remain an open research question, not an established decoder correction.

Detailed hash-pinned reports are in `../ni-file-sources/catalog/ncw-writer-validation.json`;
private outputs/scripts are in its ignored `local/ncw-writer-validation` directory.
No commercial sample assets were added here. Public tests use existing fixtures
and original synthetic data; independent live coverage currently spans these three
files, not all formats. Float/8/32-bit writing remains unsupported. Fresh raw blocks
are covered by synthetic self-tests, not these natural-file Kontakt comparisons.

## Expanded lossless WAV roundtrips

A further 199 cases were tested: 64 additional hash-distinct commercial samples,
111 installed Kontakt samples, and 24 synthetic edge cases. The commercial set
excludes the three original fixtures and samples duration ranges within mono PCM16,
stereo PCM16 and stereo PCM24. Synthetic cases cover silence, extrema, noise, equal,
opposite and opposite-parity channels; lengths 1, 511, 512, 513, 1024 and 1025.
Across the full set: PCM16/24, mono/stereo, 44.1/48/96 kHz.

Workflow: Kontakt decodes natural source NCWs to reference WAV; the Rust writer
freshly encodes those WAVs without a template; both Kontakt and Rust decode the
new NCWs. All 199 cases reproduce PCM and audio parameters exactly after the fix:
126,411,856 sample values, with 163 files using mid/side and 17 using raw blocks.
This is audio identity, not preservation of arbitrary WAV metadata or file bytes.

The initial run failed independent Kontakt comparison on 17 cases (9 commercial,
8 synthetic) while passing all internal roundtrips. First differences occurred in
zero-width raw blocks. The writer now emits observed negative full-depth raw blocks
(-16/-24); a complete rerun passes. Zero-width semantics remain unresolved, and
template writing rejects them. The historical decoder interpretation is retained,
not promoted as validated behavior. The earlier three-fixture results remain valid.

Reports: ../ni-file-sources/catalog/ncw-expanded-validation.json. Failed and fixed
runs, scripts and audio are preserved in ignored local/ncw-expanded-validation.
No commercial audio was added to this repository. Future testing should reuse this
batch and independently decode new encoding variants in Kontakt, not rely only on
self-roundtrips. Build-scoped evidence does not establish all NCW variants.
