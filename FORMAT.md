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
| 32 | 4 | unknown | 1 in both float fixtures, 0 in every PCM fixture. Possibly a format flag; the decoder ignores it and trusts the block flags. |
| 36 | 84 | opaque bytes | One repository fixture contains a UTF-16LE filename; later live Kontakt outputs contain varying non-text bytes. Not a universal source-name field. Template writer preserves these bytes. |

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
  sign-extended. Only −16 and −32 have been seen. `base_value` is still
  populated in these blocks (818 of 821 are non-zero, presumably the first
  sample) but is not needed to decode them.
* `bits == 0`: raw samples at the file's bit depth. Never seen in real output;
  covered by synthetic tests only.

Values are packed LSB-first: the first value occupies the low bits of the first
byte. Delta widths are chosen freely: every width from 2 to 24 plus 26 occurs
across the fixtures, so non-byte-multiple widths above 8 are normal. Truncated
widths have only been seen at byte multiples. The unpacker accepts 1 to 32 for
both.

### `flags`

| Bit | Meaning |
|---|---|
| 0 | mid/side: sub-block 0 is mid, sub-block 1 is side |
| 1 | samples are IEEE-754 single precision, stored as raw bit patterns |

Mid/side reconstruction is `left = mid + side`, `right = mid − side`, using
wrapping integer arithmetic for PCM and float arithmetic for float files. The
encoder is understood to store `mid = (l + r) / 2` and `side = (l − r) / 2`.
Three private PCM16/24 natural files now match Kontakt 8.9.0's decoded PCM
exactly with this inverse; their authoring builds are unknown. This does not
validate float mid/side. Opposite-parity L/R pairs cannot be represented by this
integer inverse, so the fresh writer falls back to direct encoding (or rejects
forced mid/side). No low-bit correction is invented.

Controlled Kontakt probes with flags 00, 10, 01 and 11 observed that the **first
channel's flag** selects the transform; the reader follows that behavior. Mono
mid/side is rejected. Evidence is described in [WRITER_VALIDATION.md](WRITER_VALIDATION.md).

Delta coding operates on the raw sample or bit pattern regardless of format, so
float files are delta coded on their integer representation. Verified against
the two float fixtures.

## Writer boundaries

Fresh PCM16/24 encoding chooses delta blocks at widths 2..native-depth-minus-one,
otherwise native-depth raw blocks (`bits == 0`). It compares direct and exactly
representable sum/difference payload costs, preferring direct on a tie. Padding
repeats the last stored sample and the final delta is zero. These are this
writer's choices, not a claim about NI's encoder policy.

A one-bit-delta trial passed this decoder but mismatched Kontakt on 2,158 PCM16
sample values; a minimum width of two eliminated those mismatches on the tested
fixtures. Width-one semantics remain unresolved; template writing rejects them.

Template writing also handles negative raw widths, preserves opaque header and
block fields, table-prefix bytes, unused terminal deltas, final padded samples
and trailing bytes. It reconstructs all active payload values from PCM and checks
strict table/sentinel/boundary consistency. Original NCW bytes are needed as the
template: WAV alone cannot supply all of those details. Width/flag selection is
preserved rather than reverse-engineered by a successful template roundtrip.
