# Computed Style Model

This document defines the first `ComputedStyle` data model and style-resolution proofs in RunenUI.

It is not a theme registry, recipe system, runtime theme selector, renderer backend, or material/shader API.

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

## Surface style debug report

`SurfaceStyleReport` is the first runtime-visible proof that computed style can be inspected next to surface node identity.

```text
Element tree + SurfaceFrame + StyleTokens
  -> SurfaceStyleReport
       SurfaceStyleNode
         RuntimeNodeId
         authored ElementId
         ComputedStyle
         unresolved style token diagnostics
```

This is a debug and inspection seam. It is not the final renderer model and does not make renderers responsible for token resolution.

## Ownership

`runenui_core` owns primitive style values, token maps, and pure style resolution.

`runenui_runtime` may expose resolved style facts next to surface node identity for debug, inspection, tests, and future host tooling.

Neither layer owns a real renderer backend, shader/material model, external theme loader, or component recipe system yet.

## Non-goals

This stage does not add:

- external theme files,
- component recipes,
- variant resolution,
- interaction-state style layers,
- final render primitives,
- raster backend behavior,
- SDF backend behavior.

## Boundary

`StyleIntent` is authored input.

`ComputedStyle` is resolved output.

`StyleResolution` is the core bridge product for pure style resolution. It carries concrete computed output plus unresolved-token diagnostics.

`SurfaceStyleReport` is the runtime debug product that aligns resolved style facts with surface node IDs.

## Next step

The next styling slice should add style provenance and diagnostics, so tools can explain whether a value came from a literal, token, missing token, or later recipe layer.
