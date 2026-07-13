# Styling Architecture

> **Category: Target architecture**
>
> Current implementation facts and accepted targets are separated below.

## Current contract

`runenui_core` currently owns a small renderer- and host-neutral style proof:

- literal colors, padding, and corner radii;
- typed color, spacing, and radius token references;
- `StyleIntent`, `StyleTokens`, and pure resolution;
- concrete `ComputedStyle` containing no token references;
- per-field provenance and unresolved-token diagnostics.

`AppRuntime::publish_surface` resolves current mounted authored style for each
publication context. The same resolution product supplies concrete style to
`SurfaceFrame` and provenance/diagnostics to `SurfaceStyleReport`. Computed
padding participates in measurement, placement, and outer-bound hit testing for
built-in and downstream widgets alike.

Missing tokens are non-fatal: the computed field is absent, provenance records the missing token, diagnostics retain it, and render/layout consumers do not invent a fallback.

`StyleTokens::revision()` advances after every successful definition and remains
a diagnostic/change hint. The M3 proof publication cache owns and compares an
exact token-content snapshot, so independent same-revision sets, divergent
clones, and saturated revisions cannot alias. The topology cache owns no
`StyleIntent`; style resolution checks each topology ID and reads current authored
style from the mounted node. Reconciliation separately detects an authored token
reference change even when token content and revision are unchanged. Style
resolution compares old and new computed facts: padding changes schedule layout
and hit testing, while foreground/background/radius-only changes schedule paint
without layout.

This proof does not include typography, borders, shadows, opacity, transforms, themes, recipes, variants, interaction-state layers, inheritance, external theme loading, or renderer materials. M1 removed the unused `LengthToken`/`LengthValue` family, unified geometry on validated `LogicalLength`, and made duplicate token definitions explicit non-overwriting errors. Token identity is Unicode-validated identifier text independent of static or owned storage, so mixed-form lookup and duplicate detection agree. `TokenFamily` is `#[non_exhaustive]`: color, spacing, and radius are inspectable current variants, while typography, borders, shadows, opacity, transforms, themes, recipes, and interaction-state styling make future families plausible.

## Target pipeline

```text
platform and user preferences
  -> theme tokens
  -> control recipe
  -> variant
  -> interaction state
  -> local override
  -> computed style
  -> layout inputs + paint scene + accessibility inspection
```

The resolution order must be explicit, deterministic, and inspectable. A general CSS selector/cascade system is not the initial model.

Application state owns durable meaning such as validation or selection. Mounted
widgets now own persistent proof slots for hover, pressed, focus, capture
placeholder, and scroll offset. Recipes and interaction-state styling still wait
for the M4 interaction contract and M7 styling policy; M3 slots alone do not
define production state layers.

Renderers consume resolved visual facts only. They never resolve tokens, recipes, variants, or themes. Layout consumes resolved geometry-affecting values. Accessibility/testing may inspect contrast, focus indication, disabled state, error state, and provenance without renderer ownership.

## Authoring

Typed Rust expressions remain the accepted token authoring form:

```rust
button("Save")
    .background(color_token!("color.action.primary"))
    .padding(spacing_token!("space.2"))
    .radius(radius_token!("radius.control"))
```

`element!` uses the same typed expressions. Token-specific string shorthand remains deferred until real usage or external-source pressure justifies macro grammar expansion. See [ADR 0001](../adr/0001-typed-token-authoring.md).

## Ownership and extraction

Keep primitive values, token references, pure resolution, computed style, provenance, and missing-token diagnostics in `runenui_core` while the model is narrow. Keep mounted-tree orchestration and invalidation in `runenui_runtime`.

A dedicated style/theme crate requires an independently valuable policy boundary such as external theme loading, recipes/state layers, fallback/inheritance, serialized validation, multiple independent consumers, or a dependency direction Cargo must enforce. Moving existing types alone is not sufficient.

## Milestone boundary

M1 repairs invalid/unused vocabulary. M3 supplies mounted state. M7 owns themes, recipes, variants, interaction state, preferences, the broader property set, and conformance. M8 integrates typography. No current style proof is a production theme system.
