# NCW container notes

Everything here was learned by reading files, not from a specification. Where a
claim has been verified against real Kontakt output it says so; where it rests on
a formula or a description from someone else, it says that too. See the *Help
wanted* section of the README for what would firm this up.

All integers are little-endian except the two magic numbers, which are given
here as big-endian byte sequences.

## Layout

```
offset 0     file header        120 bytes
offset 120   block offset table 4 × (blocks + 1) bytes
data_offset  blocks             data_size bytes
```

## File header

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0 | 8 | signature | `01 A8 9E D6 31 01 00 00` (seen) or `01 A8 9E D6 30 01 00 00` (accepted, never seen) |
| 8 | 2 | channels | 1 and 2 seen |
| 10 | 2 | bits per sample | 16, 24, 32 seen. 8 accepted. |
| 12 | 4 | sample rate | |
| 16 | 4 | number of samples | frames, not bytes |
| 20 | 4 | blocks offset | always 120 so far |
| 24 | 4 | data offset | `blocks_offset + 4 × (blocks + 1)` |
| 28 | 4 | data size | equals the last table entry and `file length − data_offset` in every fixture |
| 32 | 88 | unknown | zero in every fixture |

## Block offset table

One 32-bit entry per block, relative to `data_offset`, followed by one sentinel
equal to `data_size`. The first entry is always 0. The decoder derives the block
count from `(data_offset − blocks_offset) / 4 − 1` rather than trusting either
`num_samples` or `data_size`.

## Blocks

Each block covers 512 sample frames. The last block is padded to 512 and the
excess is discarded using `num_samples` from the file header.

**Channels are interleaved per block, not per file.** At each table offset there
is one sub-block per channel, in channel order. A stereo file with 257 blocks
therefore contains 514 sub-block headers. A description circulating online
claims all of channel 0's blocks precede all of channel 1's; that is wrong for
every stereo fixture in this repository.

### Sub-block header (16 bytes)

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | signature `16 0C 9A 3E` |
| 4 | 4 | base value (signed) |
| 8 | 2 | bits (signed) |
| 10 | 2 | flags |
| 12 | 4 | reserved, zero |

### `bits`

* `bits > 0`: delta coded. The body holds 512 values of `bits` width. The first
  output sample is `base_value`; each delta is added to produce the next
  sample. The 512th delta is consumed but its result is never emitted.
  Verified against reference WAVs across hundreds of blocks.
* `bits < 0`: bit truncated. The body holds 512 raw samples of `|bits|` width,
  sign-extended. Verified for 16 and 32 bit widths.
* `bits == 0`: raw samples at the file's bit depth. Never seen in real output;
  covered by synthetic tests only.

Values are packed LSB-first: the first value occupies the low bits of the first
byte. Widths that are not a multiple of 8 have been seen only below 8 bits
(delta blocks), but the unpacker accepts 1 to 32.

### `flags`

| Bit | Meaning |
|---|---|
| 0 | mid/side: sub-block 0 is mid, sub-block 1 is side |
| 1 | samples are IEEE-754 single precision, stored as raw bit patterns |

Mid/side reconstruction is `left = mid + side`, `right = mid − side`, using
wrapping integer arithmetic for PCM and float arithmetic for float files. The
encoder is understood to store `mid = (l + r) / 2` and `side = (l − r) / 2`.
**This has not been verified against a real file**, as no fixture sets bit 0.
The flag is per sub-block; the decoder treats a block as mid/side if either of
its two sub-blocks sets it, and rejects the flag on files that are not stereo.

Delta coding operates on the raw sample or bit pattern regardless of format, so
float files are delta coded on their integer representation. Verified against
the two float fixtures.
