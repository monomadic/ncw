# Fixtures

Every `.ncw` with a sibling `.wav` is compared sample-for-sample in
`../decode.rs`. Block statistics were gathered by reading every sub-block header.

| File | Channels | Depth | Blocks | Block kinds seen |
|---|---|---|---|---|
| 16-bit-mono | 1 | 16 PCM | 928 | delta, one truncated |
| 16-bit-stereo | 2 | 16 PCM | 257 | delta |
| 24-bit-mono | 1 | 24 PCM | 257 | delta |
| 24-bit-stereo | 2 | 24 PCM | 257 | delta. **No reference WAV.** |
| 32-bit-mono-float | 1 | 32 float | 928 | delta, truncated |
| 32-bit-stereo-float | 2 | 32 float | 270 | delta, truncated |
| testfile-onezero-16-bit-stereo | 2 | 16 PCM | 1 | delta (single, padded block) |
| testfile-onezero-16-bit-stereo-multiblock | 2 | 16 PCM | 2 | delta (last block padded) |

Not represented by any real file: mid/side blocks, raw `bits == 0` blocks,
8-bit files, more than two channels, the `30` file signature. See the
repository README under *Help wanted*.
