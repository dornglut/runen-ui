# Computed Style Model

This document defines the first `ComputedStyle` data model and literal-only resolution proof in RunenUI.

It is not a theme registry, recipe system, runtime integration, surface-frame change, or renderer integration.

## Decision

`ComputedStyle` is the concrete resolved counterpart to `StyleIntent`.

```text
StyleIntent
  may contain literal values
  may contain token-backed values

ComputedStyle
  contains only concrete resolved values
  contains no token references
  contains no recipe indirection
  contains no renderer-specific material data
```

The first model covers the current small style surface:

```text
foreground: Option<Color>
background: Option<Color>
padding: Option<EdgeInsets>
radius: Option<Radius>
```

## Literal-only resolver

`resolve_literal_style` converts literal `StyleIntent` values into `ComputedStyle` and reports token-backed values as unresolved.

```text
StyleIntent
  -> StyleResolution
       computed_style: ComputedStyle
       unresolved_tokens: Vec<UnresolvedStyleToken>
```

Token-backed values are not guessed or replaced with placeholders.

## Ownership

`runenui_core` owns this proof because it is pure host-neutral data conversion.

It does not depend on runtime, renderer, host, ECS, theme loading, or external files.

A later token-aware resolver may move into `runenui_runtime` or a future style/theme crate after enough pressure exists.

## Non-goals

This slice does not add:

- token resolution,
- theme maps,
- component recipes,
- variant resolution,
- interaction-state style layers,
- computed layout behavior,
- surface-frame changes,
- renderer output changes.

## Boundary

`StyleIntent` is authored input. It preserves whether a value was literal or token-backed.

`ComputedStyle` is resolved output. Renderers and layout code should eventually consume it without resolving tokens or inspecting recipes.

`StyleResolution` is the bridge product for this stage. It carries concrete computed output plus unresolved-token diagnostics.

## Next step

The next code slice should add an in-memory token map:

```text
StyleIntent with token-backed values
  -> token lookup
  -> ComputedStyle
```

That should remain host-neutral and should not introduce renderer behavior.
