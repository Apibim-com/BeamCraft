# Inter 4.1 UI font admission (FONT)

Admitted on 2026-10-16 for desktop UI text, following the owner's choice of Inter for
Latin, Cyrillic and Greek coverage. This admission includes only the unmodified Regular
and Bold TrueType files below. UI integration and rendering verification belong to V01.

## Provenance

Original publisher: The Inter Project Authors, https://github.com/rsms/inter.
Official release: [v4.1](https://github.com/rsms/inter/releases/tag/v4.1), published
2024-11-16. Source archive:
[Inter-4.1.zip](https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip).

Archive SHA-256:
`9883FDD4A49D4FB66BD8177BA6625EF9A64AA45899767DDE3D36AA425756B11E`.

The orchestrator supplied these files extracted from that archive. The worker verified
their SHA-256 values and byte sizes locally before admission; it did not download or
independently rehash the source archive. No font or licence bytes were changed.

| Repository file | Archive member | Bytes | SHA-256 |
|-----------------|----------------|-------|---------|
| `assets/fonts/inter/Inter-Regular.ttf` | `extras/ttf/Inter-Regular.ttf` | 411640 | `40D692FCE188E4471E2B3CBA937BE967878F631AD3EBBBDCD587687C7EBE0C82` |
| `assets/fonts/inter/Inter-Bold.ttf` | `extras/ttf/Inter-Bold.ttf` | 420428 | `288316099B1E0A47A4716D159098005EEF7C0066921F34E3200393DBDB01947F` |
| `licenses/OFL-1.1-Inter.txt` | `LICENSE.txt` | 4380 | `262481E844521B326F5ECD053E59B98C8B2DA78C8EE1BDBB6E8174305E54935A` |

## Licence and redistribution

The files are licensed under the SIL Open Font License 1.1 (`OFL-1.1`), an allowed
asset licence under `AGENTS.md` section 1.2. The authoritative, unmodified licence
and copyright notice are in [licenses/OFL-1.1-Inter.txt](../../../licenses/OFL-1.1-Inter.txt).
Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter).

OFL-1.1 permits use, study, copying, embedding, modification and redistribution,
including bundling with commercial software. Keep the copyright notice and full
licence with every redistributed copy of the font. The font and its components may
not be sold by themselves, and the font, including modified versions, must remain
under OFL-1.1. Documents produced using the font do not inherit that licence.

Modified versions must not use a Reserved Font Name without the relevant copyright
holder's explicit written permission. Reading this release's licence confirms that
it declares no Reserved Font Name after the copyright statement; the phrase occurs
only in the standard definition and conditions. Do not use author or copyright
holder names to promote a modified version without permission, beyond acknowledgement.
BeamCraft uses these font files unmodified.

## Admission boundary

A replacement font, a new version, an additional weight or a modified font requires
its own admission: original-source provenance, exact hashes and sizes, licence and
notice review, and an `ATTRIBUTION.md` entry. This admission does not enable egui's
rejected `default_fonts` feature.

No general repository asset hash or licence check exists to extend. Existing checks
cover generated F07 fixtures, OpenUSD wheel inventories and Cargo dependency licences;
they do not verify these font assets. No new check is introduced by FONT.
