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
