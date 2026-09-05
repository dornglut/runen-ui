# `runenui_text`

`runenui_text` is RunenUI's renderer-neutral production text boundary.

Its accepted M8B responsibility is to own explicit font-source/fallback configuration, Parley-backed shaping and line breaking, logical text metrics/artifacts, reusable private text-layout state, and immutable logical shaped-resource bindings behind RunenUI-owned public contracts.

It must not own mounted/runtime/publication state, general layout topology or scheduling, renderer/GPU/SDF-MSDF atlas state, native host integration, semantics/accessibility, application state, or editable-text behavior.

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

## Accepted M8B/M8C integration

Runtime owns the live `TextSystem` orchestration and topology-aligned reusable `TextLayoutState`. M8C's private Taffy layout requests are lowered through runtime into the smaller renderer-neutral `TextConstraints` seam. Intrinsic/compute-size text requests remain transient; the exact text state produced for Taffy's final `PerformLayout` request is the state retained for publication. The resulting immutable artifact supplies exact text measurement and the same shaped run origins and immutable `ResourceRef -> ShapedTextResource` bindings later used for paint. Publication retains explicit shaped-resource leases so renderer retry remains valid after runtime destruction or cache/device loss.

`runenui_render_wgpu` consumes those exact already-shaped font/glyph bindings and owns only disposable per-glyph SDF/MSDF generation, quality classes, atlas pages, GPU textures, reconstruction, and cache lifetime. It does not shape, line-break, discover fonts, or alter logical identity. Supported outline glyphs have no hidden alpha-raster fallback; unsupported intrinsic COLR/SVG/bitmap breadth diagnoses explicitly.

M8C owns the accepted production general-layout and exact available-space feedback path into these same text requests. M8D owns integrated responsive/text-heavy closure across runtime text/layout, semantics, paint, and the real renderer. Production editing, selection, and clipboard behavior remain M10 work; intrinsic color-glyph rendering remains separate future breadth rather than changing the accepted outline-text authority.

See ADR 0009 and `docs/conformance/m8-conformance-matrix.md` for the durable architecture and permanent proof obligations.
