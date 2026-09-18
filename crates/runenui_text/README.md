# `runenui_text`

`runenui_text` is RunenUI's renderer-neutral production text boundary.

Its accepted M8 responsibility is to own explicit font-source/fallback configuration, Parley-backed shaping and line breaking, logical text metrics/artifacts, reusable private text-layout state, and immutable logical shaped-resource bindings behind RunenUI-owned public contracts. M10B extends that same retained layout lineage with immutable caret, hit, selection, navigation, candidate-geometry, and transient preedit-projection mapping.

It must not own mounted/runtime/publication state, general layout topology or scheduling, renderer/GPU/SDF-MSDF atlas state, native host integration, semantics/accessibility, application documents, editing sessions, edit transactions, undo history, or framework services.

The dependency stack is an implementation detail. Parley, Fontique, HarfRust, Skrifa, ICU, and their public types do not become RunenUI API authority.

## Font-source policy

Construction is explicit:

- `FontSourcePolicy::BundledOnly` disables ambient system-font discovery and is the deterministic conformance/headless mode;
- `FontSourcePolicy::SystemAndBundled` permits production system discovery while retaining explicit bundled-font registration.

Bundled font registration advances a cache-visible `FontSourceRevision`. Generic families are configured explicitly through ordered named-family mappings; mapping names are resolved to canonical family identity, aliases are deduplicated without changing order, and the revision advances only when the effective mapping changes. Bundled registration never silently claims a generic-family role.

This makes generic typography deterministic in `BundledOnly` mode once its intended bundled families are registered and mapped. Deterministic tests use controlled redistributable font data and do not rely on the host's installed fonts.

## Logical layout reuse

`TextLayoutState` is caller-owned reusable state for one logical text stream. It deliberately carries no mounted identity or runtime/publication authority.

`TextSystem::layout_text` reports the work performed for each request:

- an exact request reuses the prior immutable artifact and its exact shaped `ResourceRef`s;
- inline-constraint or alignment-only changes re-line-break/re-align retained Parley layout state without rebuilding shaping;
- changes to text, metric typography/spans, language/wrap policy, or font-source identity/revision rebuild shaping.

The returned immutable `TextArtifact` is the single source for paragraph measurement and exact line/run/cluster/glyph/font shaped-resource facts. Paint-only foreground state is not part of the text request or shaped identity.

## Exact caret and preedit mapping

`TextLayoutState::caret_map` derives an immutable document/revision-scoped `TextCaretMap` from the exact retained private Parley layout and correlated `TextArtifact`. `preedit_caret_map` does the same for a `TextPreeditProjection` only when the retained request contains that projection's exact display text. Issued map clones retain their layout lineage; width-only reflow uses copy-on-write, so a retained publication can continue using its old artifact and map without mutation.

Core positions prove exact snapshot, UTF-8 bounds and scalar alignment. The caret map narrows them further through private Unicode grapheme segmentation and the retained shaping layout. Public hit, caret, candidate, selection-rectangle, logical/visual bidi, word and line navigation results contain only RunenUI-owned values. Parley cursor/selection types, glyph indices and native code-unit indices remain private. Selection geometry is derived from visual clusters without adopting Parley's guessed newline-selection width or inline-box behavior; editable maps fail closed if inline boxes are present.

`TextPreeditProjection` is an immutable transient rendering projection over one exact document snapshot, replacement range, preedit string, checked preedit-relative selection and opaque composition generation. Its mapping keeps durable document positions distinct from synthetic preedit positions, including affinity-disambiguated boundaries. It is not a committed document, editor buffer, editing session, or alternate layout.

## Accepted M8 integration

Runtime owns the live `TextSystem` orchestration and topology-aligned reusable `TextLayoutState`. Private Taffy layout requests are lowered through runtime into the smaller renderer-neutral `TextConstraints` seam. Intrinsic/compute-size text requests remain transient; the exact text state produced for Taffy's final `PerformLayout` request is the state retained for publication. The resulting immutable artifact supplies exact text measurement and the same shaped run origins and immutable `ResourceRef -> ShapedTextResource` bindings later used for paint. Publication retains explicit shaped-resource leases so renderer retry remains valid after runtime destruction or cache/device loss.

The accepted M8D closure proves that this is one correlated production path: available-space changes drive deterministic reflow through the bounded runtime/Taffy/text transaction; the artifact/resources used for measurement are the exact resources projected into paint without paint-time reshape, rebreak, or remint; final semantic text and bounds remain aligned with the same runtime-owned geometry; deterministic public tests use controlled bundled fonts; and retained resources survive renderer cache loss and raster-scale/quality re-realization through the real wgpu path.

`runenui_render_wgpu` consumes those exact already-shaped font/glyph bindings and owns only disposable per-glyph SDF/MSDF generation, quality classes, atlas pages, GPU textures, reconstruction, and cache lifetime. It does not shape, line-break, discover fonts, or alter logical identity. Supported outline glyphs have no hidden alpha-raster fallback; unsupported intrinsic COLR/SVG/bitmap breadth diagnoses explicitly.

Transactional document editing, mounted editing sessions, semantic edit actions, clipboard/IME execution and other framework services remain later M10 slices; intrinsic color-glyph rendering remains separate future breadth rather than changing the accepted outline-text authority.

See ADR 0009, ADR 0018, `docs/conformance/m8-conformance-matrix.md`, and `docs/conformance/m10-conformance-matrix.md` for the durable architecture and permanent proof obligations.
