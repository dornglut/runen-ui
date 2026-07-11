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

`runenui_runtime::publish_surface` resolves every node once during one publication. The same resolution product supplies concrete style to `SurfaceFrame` and provenance/diagnostics to `SurfaceStyleReport`. Computed padding participates in measurement, placement, and outer-bound hit testing.

Missing tokens are non-fatal: the computed field is absent, provenance records the missing token, diagnostics retain it, and render/layout consumers do not invent a fallback.

This proof does not include typography, borders, shadows, opacity, transforms, themes, recipes, variants, interaction-state layers, inheritance, external theme loading, or renderer materials. `LengthToken`/`LengthValue` are unused public vocabulary and are owned by M1 review.

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

Application state owns durable meaning such as validation or selection. Mounted widgets own ephemeral hover, pressed, focus, disabled mechanics, and animation state. Recipes and interaction-state styling therefore wait for M3 mounted state and M4 interaction contracts.

Renderers consume resolved visual facts only. They never resolve tokens, recipes, variants, or themes. Layout consumes resolved geometry-affecting values. Accessibility/testing may inspect contrast, focus indication, disabled state, error state, and provenance without renderer ownership.

## Authoring

Typed Rust expressions remain the accepted token authoring form:

```rust
button("Save")
    .background(ColorToken::new("color.action.primary"))
    .padding(SpacingToken::new("space.2"))
    .radius(RadiusToken::new("radius.control"))
```

`element!` uses the same typed expressions. Token-specific string shorthand remains deferred until real usage or external-source pressure justifies macro grammar expansion. See [ADR 0001](../adr/0001-typed-token-authoring.md).

## Ownership and extraction

Keep primitive values, token references, pure resolution, computed style, provenance, and missing-token diagnostics in `runenui_core` while the model is narrow. Keep mounted-tree orchestration and invalidation in `runenui_runtime`.

A dedicated style/theme crate requires an independently valuable policy boundary such as external theme loading, recipes/state layers, fallback/inheritance, serialized validation, multiple independent consumers, or a dependency direction Cargo must enforce. Moving existing types alone is not sufficient.

## Milestone boundary

M1 repairs invalid/unused vocabulary. M3 supplies mounted state. M7 owns themes, recipes, variants, interaction state, preferences, the broader property set, and conformance. M8 integrates typography. No current style proof is a production theme system.
