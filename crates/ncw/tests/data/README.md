# Fixtures

Every `.ncw` with a sibling `.wav` is compared sample-for-sample in
`../decode.rs`. Every PCM fixture is also rebuilt byte-for-byte from its decoded
samples by the template writer in `../write.rs`. The statistics below were
gathered by reading every sub-block header.

| File | Channels | Depth | Rate | Blocks | Block kinds seen | Delta widths |
|---|---|---|---|---|---|---|
| 16-bit-mono | 1 | 16 PCM | 44100 | 928 | 927 delta, 1 raw at −16 | 2–15 |
| 16-bit-stereo | 2 | 16 PCM | 48000 | 257 | delta | 5–12 |
| 24-bit-mono | 1 | 24 PCM | 48000 | 257 | delta | 5–19 |
| 24-bit-stereo | 2 | 24 PCM | 48000 | 257 | delta. **No reference WAV.** | 2–20 |
| 32-bit-mono-float | 1 | 32 float | 44100 | 928 | 272 delta, 656 raw at −32 | 2, 18–24, 26 |
| 32-bit-stereo-float | 2 | 32 float | 96000 | 270 | 376 delta, 164 raw at −32 (per sub-block) | 2, 23, 24 |
| testfile-onezero-16-bit-stereo | 2 | 16 PCM | 48000 | 1 | delta (single, padded block) | 2 |
| testfile-onezero-16-bit-stereo-multiblock | 2 | 16 PCM | 48000 | 2 | delta (last block padded) | 2 |

Common to all eight: signature `…31…`, blocks offset 120, block flags only 0
(PCM) or 2 (float), the sub-block word at `+0x0c` always zero. The header word
at `0x20` is 1 in the two float files and 0 elsewhere. Only
`32-bit-stereo-float` has readable text in the header's opaque region
(`1_04skank_0_127_AB01.wav` in UTF-16LE); the others are all zero there.

Not represented by any file here: mid/side blocks, zero-width or one-width
blocks, 8-bit files, more than two channels, the `30` file signature. Mid/side
decoding is covered by synthetic tests and by private natural files that cannot
be redistributed. See [FORMAT.md](../../../../FORMAT.md#wanted) for what would
fill these gaps.
