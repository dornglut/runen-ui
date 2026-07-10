# Computed Style Model

This document defines the first `ComputedStyle` data model in RunenUI.

It is a data-model checkpoint. It is not a resolver, theme registry, recipe system, runtime integration, or renderer integration.

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

## Ownership

`runenui_core` owns the `ComputedStyle` type because it is host-neutral UI data.

`runenui_core` does not own the resolver that produces it.

A resolver can be introduced later in `runenui_runtime` or in a future style/theme crate after enough pressure exists.

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

```text
authored StyleIntent
  -> resolution/provenance diagnostics
  -> ComputedStyle
```

## Next step

The next code slice should add a literal-only style resolution proof:

```text
StyleIntent with literal values
  -> ComputedStyle
```

Token-backed values should remain unresolved in that first resolver proof.
