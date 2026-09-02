# `runenui_text`

`runenui_text` is RunenUI's renderer-neutral production text boundary.

Its accepted M8B responsibility is to own explicit font-source/fallback configuration, Parley-backed shaping and line breaking, logical text metrics/artifacts, reusable private text-layout state, and immutable logical shaped-resource bindings behind RunenUI-owned public contracts.

It must not own mounted/runtime/publication state, general layout topology or scheduling, renderer/GPU/SDF-MSDF atlas state, native host integration, semantics/accessibility, application state, or editable-text behavior.

The dependency stack is an implementation detail. Parley, Fontique, HarfRust, Skrifa, ICU, and their public types do not become RunenUI API authority.

## Font-source policy

Construction is explicit:

- `FontSourcePolicy::BundledOnly` disables ambient system-font discovery and is the deterministic conformance/headless mode;
- `FontSourcePolicy::SystemAndBundled` permits production system discovery while retaining explicit bundled-font registration.

Bundled font registration advances a cache-visible `FontSourceRevision`. Deterministic tests must use controlled redistributable font data and must not rely on the host's installed fonts.

## Logical layout reuse

`TextLayoutState` is caller-owned reusable state for one logical text stream. It deliberately carries no mounted identity or runtime/publication authority.

`TextSystem::layout_text` reports the work performed for each request:

- an exact request reuses the prior immutable artifact and its exact shaped `ResourceRef`s;
- inline-constraint or alignment-only changes re-line-break/re-align retained Parley layout state without rebuilding shaping;
- changes to text, metric typography/spans, language/wrap policy, or font-source identity/revision rebuild shaping.

The returned immutable `TextArtifact` remains the single source for paragraph measurement and exact shaped-run/resource facts. Paint-only foreground state is not part of the text request or shaped identity.

## Current M8B implementation state

The package now establishes the ownership boundary, explicit font-source policy/revision, renderer/runtime-neutral text constraints, Parley-backed logical shaping and line breaking, immutable logical artifacts, caller-owned reusable layout state, cache/reflow diagnostics, and scale-independent shaped-resource lifetime.

Runtime measurement/publication cutover, the bounded SDF/MSDF generator evaluation, renderer SDF/MSDF atlas/shader realization, unsupported intrinsic-color glyph diagnostics, and the remaining M8B conformance corpus are still active work. Do not infer M8B acceptance from this package state.

See ADR 0009 and `docs/conformance/m8-conformance-matrix.md` for the target contract and permanent proof obligations.
