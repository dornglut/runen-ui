# Computed Style Model

This document defines the first `ComputedStyle` data model and style-resolution proofs in RunenUI.

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

Token-backed values are not guessed or replaced with placeholders.

## In-memory token resolver

`StyleTokens` is a small typed in-memory token container.

It resolves:

```text
ColorToken   -> Color
SpacingToken -> EdgeInsets
RadiusToken  -> Radius
```

`resolve_style` accepts `StyleIntent` and `StyleTokens`. Literal values copy into `ComputedStyle`. Token-backed values look up through `StyleTokens`. Missing tokens remain in `UnresolvedStyleToken` diagnostics.

## Ownership

`runenui_core` owns these proofs because they are pure host-neutral data conversion.

They do not depend on runtime, renderer, host, ECS, external theme files, or shader/material APIs.

## Non-goals

This stage does not add:

- external theme files,
- component recipes,
- variant resolution,
- interaction-state style layers,
- surface-frame changes,
- renderer output changes.

## Boundary

`StyleIntent` is authored input.

`ComputedStyle` is resolved output.

`StyleResolution` is the bridge product for this stage. It carries concrete computed output plus unresolved-token diagnostics.

## Next step

The next styling slice should connect computed style to an observable surface or debug path, still without committing to a final renderer model.
