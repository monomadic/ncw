# NCW container notes

There is no public specification. Everything here was learned by reading files
and by comparing output against Kontakt's own decoder. Each claim says what it
rests on: the fixtures in `crates/ncw/tests/data`, controlled Kontakt probes, or
a formula that has not been checked. Kontakt probes were run against Kontakt
8.9.0 (ARM64) using commercial library material; the results are summarised
here, but that audio cannot be published.

Open questions are collected under [Unknowns](#unknowns). Material that would
help answer them is listed under [Wanted](#wanted).

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
| 0x00 | 8 | signature | `01 A8 9E D6 31 01 00 00` (seen) or `01 A8 9E D6 30 01 00 00` (accepted, never seen in a real file) |
| 0x08 | 2 | channels | 1 and 2 seen |
| 0x0a | 2 | bits per sample | 16, 24, 32 seen. 8 accepted. |
| 0x0c | 4 | sample rate | |
| 0x10 | 4 | number of samples | frames, not bytes |
| 0x14 | 4 | blocks offset | 120 in every file seen |
| 0x18 | 4 | data offset | `blocks_offset + 4 × (blocks + 1)` |
| 0x1c | 4 | data size | equals the last table entry and `file length − data_offset` in every fixture |
| 0x20 | 4 | format word | 1 in both float fixtures, 0 in every PCM fixture. The decoder ignores it. See [Unknowns](#unknowns). |
| 0x24 | 84 | opaque | Not a reliable filename field. The decoder ignores it; the template writer preserves it. See [Unknowns](#unknowns). |

## Block offset table

One 32-bit entry per block, relative to `data_offset`, followed by one sentinel
equal to `data_size`. The first entry is always 0. The decoder requires exactly
`ceil(num_samples / 512) + 1` entries, strictly increasing block offsets, and a
sentinel matching `data_size`. It checks file bounds and minimum channel framing
before decoding, and bounds each group's payload reads to its table interval.
Bytes between the table and `data_offset`, and bytes after the data region, are
permitted.

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
| 0x00 | 4 | signature `16 0C 9A 3E` |
| 0x04 | 4 | base value (signed) |
| 0x08 | 2 | bits (signed) |
| 0x0a | 2 | flags |
| 0x0c | 4 | unknown; zero in every fixture |

### `bits`

* `bits > 0`: delta coded. The body holds 512 values of `bits` width. The first
  output sample is `base_value`; each delta is added to produce the next
  sample. The 512th delta is consumed but its result is never emitted.
  Verified against reference WAVs across hundreds of blocks.
* `bits < 0`: raw. The body holds 512 samples of `|bits|` width,
  sign-extended. The fixtures contain −16 and −32; Kontakt probes also
  confirmed −24. `base_value` is still populated in these blocks (818 of 821 in
  the fixtures are non-zero) but is not needed to decode them.
* `bits == 0` and `bits == 1`: not understood. See [Unknowns](#unknowns).

Values are packed LSB-first: the first value occupies the low bits of the first
byte. Delta widths are chosen freely: every width from 2 to 24, plus 26, occurs
across the fixtures, so non-byte-multiple widths above 8 are normal. Raw widths
have only been seen at byte multiples. The unpacker accepts 1 to 32 for both.

### `flags`

| Bit | Meaning |
|---|---|
| 0 | mid/side: sub-block 0 is mid, sub-block 1 is side |
| 1 | samples are IEEE-754 single precision, stored as raw bit patterns |

Mid/side reconstruction is `left = mid + side`, `right = mid − side`, using
wrapping integer arithmetic for PCM and float arithmetic for float files. For
PCM16 and PCM24 this is verified: three natural mid/side files decode to exactly
the same samples as Kontakt does, and 163 mid/side files produced by this
crate's writer also decode identically in Kontakt (see
[WRITER_VALIDATION.md](WRITER_VALIDATION.md)). Float mid/side has not been
checked against any real file.

Kontakt 4.2 stores `mid = (l + r) / 2` and `side = (l − r) / 2` using integer
division that truncates toward zero, and does not check whether `l + r` is odd.
When it is, the pair cannot be recovered: **mid/side blocks written by Kontakt
4.2 are not lossless.** See [Kontakt 4 mid/side is lossy](#kontakt-4-midside-is-lossy).
This crate's writer uses mid/side only for blocks where every pair has an even
sum, so its output is always lossless.

Controlled Kontakt probes with every combination of the two channels' bit 0
showed that **the first channel's flag** selects the transform; the reader
follows that. Mid/side on a mono file is rejected.

Delta coding operates on the raw sample or bit pattern regardless of format, so
float files are delta coded on their integer representation. Verified against
the two float fixtures.

## Writer

The writer handles PCM16 and PCM24, mono and stereo. It produces the same block
format Native Instruments' tools do, and Kontakt decodes its output exactly, but
its choice of widths, mid/side and padding was written from the format rather
than taken from Native Instruments' encoder. Re-encoding three natural files
gave the same file sizes as the originals; one PCM24 file matched byte-for-byte
after the header, and the others differed in some block choices. Kontakt 8.9.0
itself does not reproduce those originals either (see *Encoder choices* under
[Unknowns](#unknowns)).

Fresh encoding chooses delta blocks at widths 2 up to one less than the bit
depth, otherwise raw blocks at the negative bit depth (−16 or −24). For stereo
it compares direct and mid/side costs where mid/side is exactly representable,
preferring direct on a tie. Padding repeats the last stored sample and the final
delta is zero. It never emits widths 0 or 1 (see [Unknowns](#unknowns)).

Template writing takes an original NCW plus PCM and rebuilds the file
byte-for-byte. It keeps the header, the block widths and flags, unused terminal
deltas, padded tail samples and any extra bytes, and recomputes every active
payload value from the PCM. A WAV alone does not contain those choices, so the
original NCW is required. Byte identity from a template says nothing about how
Native Instruments' encoder picks widths or flags.

## Kontakt 4's encoder

A Kontakt 4.2 executable (macOS, SHA-256
`d7183a84bc01b4d17b7bcf5d3ec0282733219e98aa5b06c6d739cb3e8130227b`; its
embedded source paths name a `Kontakt4_2` build tree) contains Native
Instruments' own encoder, `BNCWWrite`, with symbols intact. The findings below
come from reading its x86_64 machine code (`BNCWWrite::SetData` and
`BNCWWrite::Write`); the program was not run.

| Choice | Kontakt 4.2 |
|---|---|
| Input | Integer output: each sample is read as float and converted with `(int)(x × 2^15)`, `2^23` or `2^31` for 16, 24 or 32-bit. Float output keeps the float's bit pattern. There is also an 8-bit path. |
| Signature | `01 A8 9E D6 31 01 00 00` |
| Base value | The first sample of the block |
| Deltas | `s[i+1] − s[i]` for the first 511; the 512th delta is always 0 |
| Delta width | The narrowest width that fits every delta, but never less than 2 |
| Raw blocks | When the delta width reaches the bit depth, the block is stored raw with `bits = −depth` |
| Flags | `(float ? 2 : 0) \| mid_side`; the word at `+0x0c` is always 0 |
| Mid/side choice | Per block: mid/side is used when the mid and side widths add up to less than the left and right widths, and both fit within the bit depth. Ties go to direct. Files above 31 bits per sample, which includes float, never use mid/side. |

Its decoder, `BADPCMDecompress`, has a dedicated routine for each delta width
from 2 to 23 and none for 0 or 1. Together with the encoder never emitting
them, this suggests widths 0 and 1 were never part of the format.

This crate's writer makes the same choices for widths, raw blocks, the terminal
delta and tie-breaking. The difference is mid/side.

### Kontakt 4 mid/side is lossy

Kontakt 4.2 computes `mid = (l + r) / 2` and `side = (l − r) / 2` with C integer
division (truncation toward zero), stores those values, and checks nothing about
parity on the way. The decoder computes `l = mid + side` and `r = mid − side`.

When `l + r` is even the round trip is exact. When it is odd, exactly one
channel comes back one step closer to zero. For example, `(3, 0)` becomes
`(2, 0)` and `(0, −3)` becomes `(0, −2)`. Checking every pair from −64 to 64
gives that result for all 8,320 odd-sum pairs and no other error. In ordinary
audio about half of all sample pairs have an odd sum, so a mid/side block
typically changes the least significant bit of one channel in about half of its
frames.

Native Instruments describes NCW as a lossless codec (for example, the Kontakt
5.7 manual calls it "a proprietary, lossless audio codec"). That holds for
direct blocks but not for these mid/side blocks. Consequences:

* Commercial NCW files that use mid/side, 94% of the scan described under
  [Wanted](#wanted), may differ from their source recordings by one least
  significant bit in places. Without the original WAVs this cannot be detected
  from the NCW file alone, because decoded mid/side output always has an even
  `l + r`.
* Kontakt 8.9.0 never writes mid/side. Dropping it would be consistent with the
  loss having been noticed, though nothing in the binaries says why it changed.
* This decoder reproduces exactly what Kontakt plays. The loss happened when
  the file was made, not when it is read.

Not yet confirmed by running Kontakt 4: encoding a stereo signal with known odd
sums and comparing the decoded result would make this an observed result rather
than a reading of the code.

## Unknowns

Each entry states what has been observed and what is still open. A field this
decoder ignores is not thereby known to be meaningless.

| Location | Observed | Open |
|---|---|---|
| Header `0x20` | 1 in float fixtures, 0 in PCM fixtures. In 40 Kontakt probes, bit 0 selected a 32-bit output type in files with signature `31`, and was ignored in files with signature `30`. Block flag bit 1 separately controls how samples are interpreted. | This decoder infers the format from block flags alone, so a file whose header and block flags disagree can produce a different WAV type from Kontakt's. Other values and bits are untested. |
| Header `0x24..0x77` | One fixture holds a UTF-16LE filename; newer Kontakt output holds varying non-text bytes. Two exports of the same audio differed in 28 header bytes with identical tables and payloads. Kontakt 8.9.0's writer was traced leaving this region uninitialised before writing all 120 bytes, and filling it with zeros or `0xa5` in six probes left Kontakt's output unchanged. | Whether any build stores meaningful data here, and whether any reader uses it. This writer fills it with zeros. |
| Block width 0 | Kontakt 4.2 never writes it and has no decoder routine for it ([details](#kontakt-4s-encoder)). The decoder treats it as raw samples at the file's bit depth, a historical guess. Files written that way decoded incorrectly in Kontakt. In Kontakt 8.9.0's optimised PCM path, widths 0 and 1 branch to an exit that writes no samples, and repeated probes produced output that depended on what had been decoded before. | Whether any encoder produces it, and what other Kontakt versions do with it. The writer rejects it. |
| Block width 1 | Kontakt 4.2 raises widths below 2 to 2 and has no decoder routine for width 1. Treating it as a signed one-bit delta passed this decoder but disagreed with Kontakt on 2,158 PCM16 samples. The dispatch finding above applies. | As for width 0. The writer rejects it. |
| Sub-block `+0x0c` | Zero in every fixture, and always written as zero by Kontakt 4.2. In 22 probes on mono PCM16/24 raw and delta blocks, changing it did not change Kontakt's output. | Float, stereo and mid/side blocks are untested, as is whether any build writes non-zero values. |
| Sub-block base on raw blocks | Kontakt 4.2 writes the block's first sample here for every block type, which explains why it is usually non-zero (818 of 821 raw sub-blocks in the fixtures). Changing it on mono PCM16/24 raw blocks did not change Kontakt's output. | What the encoder puts there, and whether it matters for float or stereo. |
| Flags beyond bits 0 and 1 | Bits 0 and 1 behave as described above. Kontakt 4.2 only ever writes bits 0 and 1, and never sets both together. | What decoders do with the high bits or with float combined with mid/side, and anything beyond two channels. |
| Terminal delta and padding | The 512th delta's result is never emitted, and samples past `num_samples` are discarded. Kontakt 4.2 always writes the 512th delta as 0. | What padding values Kontakt 4.2 writes, and whether any decoder carries state across blocks. |
| Encoder choices | Re-encoding gives identical audio but different widths, flags and bytes. One natural file with 635 mid/side and 38 direct block groups, decoded and re-encoded by Kontakt 8.9.0 (both through its scripting API and by saving an instrument), came back as 673 direct groups and 5.8% larger. | Kontakt 4.2's rules are now known ([details](#kontakt-4s-encoder)). Which versions between 4.2 and 8.9.0 still wrote mid/side, and whether NI's library-production tools used the same encoder. |
| Signature `30`, 8-bit, more than two channels | Accepted by the parser. Kontakt 4.2's encoder can write 8-bit files. | No real file seen. |

## Wanted

Material that would answer an open question. Fixtures do not need to be long; a
few thousand samples is enough. Please open an issue or pull request with the
`.ncw` and, where possible, the `.wav` it was made from.

1. **A redistributable mid/side file with known PCM.** Mid/side PCM is
   understood, but every natural example comes from a commercial library and
   cannot be committed as a test fixture. Making one is not straightforward:
   Kontakt 8.9.0 wrote only direct blocks in every encode tried (21 generated
   signals, and a decoded natural mid/side file re-encoded through its scripting
   API and by saving an instrument), and none of the 111 NCW files installed
   with Kontakt use mid/side. Yet in commercial libraries it is the norm: of
   3,484 standalone NCW files scanned from a set of commercial libraries,
   3,292 (94%) contain at least one mid/side block. *Batch Compress* in Kontakt's interface is
   untested. An example produced in the wild, or a tool or version that
   writes mid/side, is needed.
2. **A float mid/side file**, if one exists. Kontakt 4.2 never writes one, so
   float reconstruction is untested against real data.
3. **A reference WAV for `24-bit-stereo.ncw`.** It decodes, but its test can
   only check the sample count.
4. **A real file with width 0 or 1 blocks**, if any encoder produces them.
5. **An 8-bit file.** Kontakt 4.2's encoder supports 8-bit output, but no real
   file has been seen.
6. **A file with more than two channels**, to learn how block layout and
   mid/side apply beyond stereo.
7. **A file with signature `01 A8 9E D6 30 01 00 00`.**
8. **A stereo file encoded by Kontakt 4 from a known WAV**, to confirm the
   mid/side loss in practice. Exports from versions between 4.2 and 8.9.0 would
   show when mid/side was dropped.

### No longer needed

* **Natural mid/side files.** Until September 2026 no real mid/side file had
  been examined and the reconstruction formula was an untested guess. Natural
  mid/side files turn out to make up 94% of a 3,484-file commercial library
  scan, and three PCM16/24
  examples were checked against Kontakt sample-for-sample.
* **Raw blocks at −24.** Confirmed in Kontakt probes on PCM24 material.
* **How Native Instruments chose widths and mid/side.** Read from Kontakt 4.2's
  encoder (see [Kontakt 4's encoder](#kontakt-4s-encoder)).
