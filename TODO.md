# NCW implementation tasks

- [x] Add PCM16/24 writing and template-preserving byte-identical CLI roundtrips.
- [x] Validate three unique natural mid/side files against Kontakt PCM; keep audio private.
- [x] Validate 24 synthetic WAV edge cases through fresh encoding and independent Kontakt decoding, including negative raw blocks.
- [ ] Turn preserved synthetic Kontakt expectations into a portable regression fixture set.
- [ ] Isolate zero-width raw-block semantics; emitted zero-width blocks failed independent Kontakt tests.
- [ ] Isolate width-one delta semantics with minimal discriminating Kontakt probes before supporting writing them.
- [ ] Expand writing to float and other depths only with independent reference evidence.
- [ ] Investigate fresh encoder tie-breaking and final padding; metadata-free byte identity remains open.
- [ ] Consider a compact metadata sidecar so lossless NCW reconstruction need not retain the original as a template.

Format research belongs in ../ni-file-reference/TODO.md and source preservation
in ../ni-file-sources/TODO.md. See WRITER_VALIDATION.md before repeating experiments.

- [ ] Follow the canonical unknown-field experiments in
  ../ni-file-reference/other/NCW-unknowns.md; promote behavior only with named evidence.

- [ ] Investigate strict reader rejection of unvalidated widths 0/1 after tracing
  NI dispatch: minimal live probes produced history-dependent PCM, while the current
  reader still implements historical hypotheses. Do not silently treat them as validated.

- [ ] Decide how the reader and CLI should honor main-header format bit 0 and
  version 0x130 independently of block float flags; current inference can export
  a different WAV type from Kontakt on contradictory headers. Use the 40-case
  matrix in ../ni-file-reference/other/NCW-field-probes.md, including exact numeric
  conversion expectations, before changing behavior.
