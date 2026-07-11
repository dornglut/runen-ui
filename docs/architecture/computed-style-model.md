# Computed Style Model

This document defines the current `ComputedStyle`, style-resolution, provenance, and runtime inspection models in RunenUI.

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
  contains no provenance or diagnostics
  contains no recipe indirection
  contains no renderer-specific material data
```

The current model covers the small implemented style surface:

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
ColorToken    -> Color
SpacingToken  -> EdgeInsets
RadiusToken   -> Radius
```

`resolve_style` accepts `StyleIntent` and `StyleTokens`. Literal values copy into `ComputedStyle`. Token-backed values look up through `StyleTokens`. Missing tokens remain in `UnresolvedStyleToken` diagnostics.

## Style provenance

`StyleFieldProvenance<Token>` explains the source and resolution outcome of one style field:

```text
Absent
  no value was authored for the field

Literal
  a concrete literal was authored and copied into ComputedStyle

ResolvedToken(Token)
  a token reference was authored and resolved successfully

MissingToken(Token)
  a token reference was authored but no value existed in StyleTokens
```

`StyleProvenance` carries one typed provenance entry for each current style field:

```text
foreground: StyleFieldProvenance<ColorToken>
background: StyleFieldProvenance<ColorToken>
padding: StyleFieldProvenance<SpacingToken>
radius: StyleFieldProvenance<RadiusToken>
```

Provenance remains separate from `ComputedStyle`. Renderer and layout consumers can read concrete values without carrying authoring history, while inspectors and diagnostics can explain how each value was produced.

## Resolution product

`StyleResolution` is the complete product of pure style resolution:

```text
StyleResolution
  ComputedStyle
  StyleProvenance
  Vec<UnresolvedStyleToken>
```

The provenance product answers a per-field question. The unresolved-token list preserves the existing aggregate diagnostic path. A missing token therefore appears as `MissingToken` for the affected field and as the corresponding `UnresolvedStyleToken` entry.

`computed_style()` and `unresolved_tokens()` retain their existing behavior. `provenance()` exposes the parallel per-field explanation.

## Surface style debug report

`SurfaceStyleReport` is the runtime-visible proof that computed style and provenance can be inspected next to surface node identity.

```text
Element tree + SurfaceFrame + StyleTokens
  -> SurfaceStyleReport
       SurfaceStyleNode
         RuntimeNodeId
         authored ElementId
         StyleResolution
           ComputedStyle
           StyleProvenance
           unresolved style token diagnostics
```

Each `SurfaceStyleNode` owns the complete `StyleResolution` for its corresponding runtime node and delegates access to the computed style, provenance, and unresolved-token diagnostics.

This is a debug and inspection seam. It is not the final renderer model and does not make renderers responsible for token resolution.

The transitional `resolve_surface_style_report(root, frame, tokens)` path has been removed. `publish_surface` now prepares one resolved runtime tree and derives both `SurfaceFrame` and `SurfaceStyleReport` from it; see [Computed Style Runtime Integration](computed-style-runtime-integration.md).

## Ownership

`runenui_core` owns primitive style values, token maps, pure style resolution, provenance, and unresolved-token diagnostics.

`runenui_runtime` may expose the complete resolution product next to surface node identity for debug output, inspection, tests, and future host tooling.

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

`ComputedStyle` is concrete resolved output.

`StyleProvenance` is field-level authoring and resolution explanation.

`StyleResolution` is the core bridge product that keeps concrete output, provenance, and unresolved-token diagnostics together.

`SurfaceStyleReport` is the runtime debug product that aligns complete style-resolution facts with surface node IDs. It is a diagnostic projection of the same per-node `StyleResolution` product used to place concrete `ComputedStyle` on `SurfaceFrame`.

## Deferred extensions

Recipe, variant, interaction-state, fallback, and external-theme provenance may extend the provenance model later. They must not be introduced until those resolution layers exist and their precedence rules are defined.
