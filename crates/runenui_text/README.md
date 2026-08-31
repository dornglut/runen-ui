# `runenui_text`

`runenui_text` is RunenUI's renderer-neutral production text boundary.

Its accepted M8B responsibility is to own explicit font-source/fallback configuration, Parley-backed shaping and line breaking, logical text metrics/artifacts, and immutable logical shaped-resource bindings behind RunenUI-owned public contracts.

It must not own mounted/runtime/publication state, general layout topology or scheduling, renderer/GPU/SDF-MSDF atlas state, native host integration, semantics/accessibility, application state, or editable-text behavior.

The dependency stack is an implementation detail. Parley, Fontique, HarfRust, Skrifa, ICU, and their public types do not become RunenUI API authority.

## Font-source policy

Construction is explicit:

- `FontSourcePolicy::BundledOnly` disables ambient system-font discovery and is the deterministic conformance/headless mode;
- `FontSourcePolicy::SystemAndBundled` permits production system discovery while retaining explicit bundled-font registration.

Bundled font registration advances a cache-visible `FontSourceRevision`. Deterministic tests must use controlled redistributable font data and must not rely on the host's installed fonts.

## Current M8B implementation state

The package currently establishes the ownership boundary, explicit font-source policy/revision, and renderer/runtime-neutral text constraints. Production shaping, immutable logical text artifacts, runtime measurement cutover, logical shaped-resource lifetime, and renderer SDF/MSDF realization are still active M8B work and must not be inferred from this package foundation alone.

See ADR 0009 and `docs/conformance/m8-conformance-matrix.md` for the target contract and permanent proof obligations.
