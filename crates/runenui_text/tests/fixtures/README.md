# Deterministic text fixtures

These fonts are checked-in test data for deterministic `runenui_text` shaping and renderer-realization evidence. Conformance tests must not depend on host-installed fonts.

## Cantarell Regular

`Cantarell-Regular.ttf` is the existing Latin fixture. It is covered by the SIL Open Font License 1.1; the retained notice is `Cantarell-Regular.OFL.txt`.

## RunenUI Fixture Devanagari

`RunenUIFixtureDevanagari-Regular.ttf` is a renamed, corpus-minimized derivative of Noto Sans Devanagari UI Regular. It retains the OpenType shaping tables and glyph closure needed by the controlled Devanagari corpus while avoiding a full-font test payload.

Source record:

- upstream repository: `google/fonts`;
- upstream path: `ofl/notosansdevanagariui/NotoSansDevanagariUI-Regular.ttf`;
- source content SHA-256: `c0b105d248337435b876d3a3d19b380c9782e452b15f142a0cf7b10f5d29a280`;
- source family/version: Noto Sans Devanagari UI Regular 2.001;
- derivative family name: `RunenUI Fixture Devanagari`.

The derivative is covered by the SIL Open Font License 1.1; the retained notice is `RunenUIFixtureDevanagari-Regular.OFL.txt`.

## RunenUI Fixture Arabic

`RunenUIFixtureArabic-Regular.ttf` is a renamed, corpus-minimized derivative of Noto Kufi Arabic Regular. It retains the OpenType shaping tables and glyph closure needed by the controlled Arabic joining/bidi corpus while avoiding a full-font test payload.

Source record:

- upstream project: Noto (`googlei18n/noto-fonts` / `notofonts/noto-fonts` lineage);
- packaged source: Debian `fonts-noto-core` `20201225-2`;
- packaged path: `/usr/share/fonts/truetype/noto/NotoKufiArabic-Regular.ttf`;
- source content SHA-256: `befb9d07506942ba2a7e0e85c65a24a76792c937d812bc80f1095d0f660cf330`;
- source family/version: Noto Kufi Arabic Regular 2.102;
- derivative family name: `RunenUI Fixture Arabic`.

The source embeds SIL Open Font License 1.1 metadata and Debian distributes the package under OFL-1.1. The derivative is covered by the same license; the retained notice is `RunenUIFixtureArabic-Regular.OFL.txt`.

These derivatives are test fixtures only. Their names deliberately do not preserve upstream family names as derivative family identity. Production font selection remains explicit through `TextSystem` font-source policy and registration APIs.
