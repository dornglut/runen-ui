# Computed Style Runtime Integration

This document defines how RunenUI turns authored style intent into one runtime-owned surface-publication pipeline.

It is the implementation contract that follows the computed-style, token-resolution, provenance, and surface-style debug proofs. It does not introduce component recipes, interaction-state styling, external themes, renderer backends, accessibility output, or new crates.

## Implementation status

Slice 1 is implemented. `SurfaceBuildContext`, `SurfacePublication`, and `publish_surface` now provide the single public publication path. One prepared runtime tree supplies runtime identity, per-node `StyleResolution`, frame `ComputedStyle`, and style diagnostics. The former public `layout_surface`, `layout_surface_with_metrics`, `resolve_surface_style_report`, `AppRuntime::surface_frame`, and `AppRuntime::surface_frame_with_metrics` paths have been removed.

Slice 2 remains next: computed padding must affect measurement and child placement.

## Problem

The current implementation has two independent paths over the same element tree:

```text
Element tree
  -> layout_surface
  -> SurfaceFrame

Element tree + SurfaceFrame + StyleTokens
  -> resolve_surface_style_report
  -> SurfaceStyleReport
```

The layout path does not consume `ComputedStyle`. The debug path resolves style separately and aligns its result with surface nodes by traversal order.

That arrangement was sufficient to prove token resolution and diagnostics, but it must not become the permanent runtime contract. It has four structural problems:

1. style can be resolved independently from the frame that consumes it;
2. node alignment depends on two traversals remaining identical;
3. layout cannot consume geometry-affecting computed values such as padding;
4. render-facing nodes cannot consume concrete foreground, background, or radius values.

## Decision

Surface publication must perform one runtime-owned preparation pass and derive layout, render-facing computed style, and style diagnostics from that same product.

```text
Element<Action>
  + StyleTokens
  + surface build parameters
  -> runtime surface preparation
       runtime node identity
       parent/child structure
       borrowed element facts
       one StyleResolution per node
  -> layout
  -> SurfaceFrame
       bounds
       renderer-facing kind
       ComputedStyle
  -> SurfaceStyleReport
       StyleProvenance
       unresolved-token diagnostics
```

Style resolution remains a pure `runenui_core` operation. `runenui_runtime` owns orchestration for a mounted tree and guarantees that every downstream product uses the same resolution result.

## Authoritative products

### Authored input

`Element<Action>` remains the authored structural input.

Each element provides:

```text
ElementKind<Action>
LayoutStyle
StyleIntent
optional ElementId
optional ElementKey
```

`StyleIntent` may contain literal values or typed token references. It is never passed directly to a renderer.

### Core resolution product

`StyleResolution` remains the complete pure result for one element:

```text
StyleResolution
  ComputedStyle
  StyleProvenance
  Vec<UnresolvedStyleToken>
```

`ComputedStyle` contains only concrete values. Provenance and diagnostics remain parallel inspection data.

### Runtime preparation product

The runtime should introduce one internal surface-preparation tree. The target name is `ResolvedSurfaceTree`; an equivalent private name is acceptable if the ownership and invariants remain the same.

Each prepared node carries, directly or through an internal arena:

```text
RuntimeNodeId
parent and child structure
borrowed Element<Action>
StyleResolution
```

This product is ephemeral for one publication call. It is not a retained widget tree, a renderer object, or a new public crate boundary.

The preparation pass assigns `RuntimeNodeId` once in deterministic pre-order. Layout and diagnostics must not independently regenerate node IDs or zip unrelated traversal results.

### Surface frame

`SurfaceFrame` remains the public renderer-facing snapshot.

Each `SurfaceNode` should carry:

```text
RuntimeNodeId
parent RuntimeNodeId
optional authored ElementId
LogicalRect
SurfaceNodeKind
ComputedStyle
```

The frame does not carry `StyleIntent`, token references, provenance, unresolved-token diagnostics, recipes, or theme data.

A renderer therefore consumes the same concrete style facts regardless of whether the value originated as a literal or a token.

### Style diagnostics

`SurfaceStyleReport` remains the runtime inspection product.

It is generated from the prepared resolution data, not by resolving the element tree again and not by joining against an independently constructed frame.

Each diagnostic node continues to expose:

```text
RuntimeNodeId
optional authored ElementId
StyleResolution
```

`SurfaceStyleReport` may duplicate the small `ComputedStyle` value already copied into `SurfaceNode`. That duplication is acceptable because both values are derived from the same `StyleResolution`; independent resolution is not.

## Public publication seam

The runtime should expose one authoritative surface-publication operation.

Target conceptual API:

```rust
let tokens = StyleTokens::new();
let context = SurfaceBuildContext::new(&tokens);
let publication = runtime.publish_surface(size, &context);

let frame = publication.frame();
let style_report = publication.style_report();
```

The exact constructors may change during implementation, but the following decisions are fixed:

- style tokens are explicit input;
- layout metrics are explicit build context, not global state;
- one call produces the frame and its aligned style diagnostics;
- the runtime resolves each node style once per publication;
- normal renderer consumers read `SurfaceFrame`;
- inspectors and tests may additionally read `SurfaceStyleReport`.

The target context shape is:

```text
SurfaceBuildContext<'a>
  &'a StyleTokens
  SurfaceLayoutMetrics
```

The target output shape is:

```text
SurfacePublication
  SurfaceFrame
  SurfaceStyleReport
```

`SurfaceBuildContext::new(&tokens)` should use default placeholder layout metrics. A separate constructor or builder may accept explicit metrics.

Do not introduce a process-global token registry. Do not store an implicit mutable theme in `Element`. Runtime-owned theme selection may be added later when a real theme model exists.

## API cutover

The final public path must not preserve separate styled and unstyled publication pipelines.

The implementation slice should replace or internalize these current functions:

```text
layout_surface
layout_surface_with_metrics
resolve_surface_style_report
AppRuntime::surface_frame
AppRuntime::surface_frame_with_metrics
```

A temporary private adapter is acceptable inside one implementation PR. Public compatibility wrappers are not the target because the workspace has not declared these APIs stable and parallel paths would preserve the architectural split this design removes.

The replacement path is conceptually:

```text
publish_surface
AppRuntime::publish_surface
```

## Node identity invariant

For one built root tree:

```text
RuntimeNodeId = deterministic pre-order index
```

The following products must agree on that identity:

```text
runtime tree lookup
surface preparation
SurfaceFrame
SurfaceStyleReport
hit testing
focus and activation targets
trace targets
```

Surface preparation should reuse the runtime traversal rules rather than reimplementing a second subtly different element walk. A shared private traversal helper or a runtime tree product may enforce this.

A frame/report length mismatch must be impossible through public constructors. Do not silently truncate with `Iterator::zip`, invent empty styles, or recover by re-resolving a node.

## Layout contract

Layout consumes concrete `ComputedStyle`, never token-backed `StyleIntent`.

For the current model, layout reads:

```text
ComputedStyle::padding
```

It continues to read structural authored facts that are not yet part of computed style:

```text
container axis
LayoutStyle::gap
control kind
text or button content
```

This is an intentional transitional boundary. Moving gap, sizing, alignment, border width, or typography into computed style requires separate style-field designs and must not be smuggled into this cutover.

## Padding box model

The first layout-affecting computed style field is padding.

Every surface node has an outer border box and an inner content box:

```text
outer bounds
  inset by computed padding
  -> content bounds
```

Required behavior:

1. The root receives the requested surface size as its outer bounds.
2. Root children are positioned inside the root content bounds.
3. Text intrinsic content size is measured first; padding expands its outer size.
4. Button label content is measured first; padding expands its desired outer size.
5. Button minimum width and height remain minimum outer-size policy.
6. Container content size is derived from children and gap; padding expands its outer size.
7. Container children start at the content-box origin.
8. Asymmetric top, right, bottom, and left padding must be preserved exactly.
9. Hit testing continues to use outer bounds.

When `ComputedStyle::padding()` is `None`, layout uses zero insets.

When a padding token is missing, `ComputedStyle::padding()` remains `None`, layout therefore uses zero insets, and the missing token remains visible in provenance and unresolved-token diagnostics. This is deterministic absence handling, not a token fallback.

## Placeholder metric cleanup

`SurfaceLayoutMetrics::button_horizontal_padding` conflicts with computed padding because it embeds hidden control chrome into intrinsic measurement.

When computed padding begins affecting layout, button measurement should use this model:

```text
label intrinsic width/height
  + computed padding
  -> desired outer size
  constrained by min_button_width and button_height
```

`button_horizontal_padding` should therefore be removed or retired in the padding implementation slice. Keeping both as additive padding would make authored style dependent on an undocumented second padding source.

The remaining placeholder metrics are still temporary until a text measurement seam exists.

## Missing values and errors

Style resolution remains non-fatal for missing tokens.

```text
missing token
  -> missing field in ComputedStyle
  -> MissingToken provenance
  -> UnresolvedStyleToken diagnostic
```

The runtime must not:

- substitute arbitrary colors, spacing, or radius values;
- ask the renderer to resolve tokens;
- discard the diagnostic;
- panic because a token is missing.

Geometry validation for negative or non-finite authored lengths is a separate contract. This integration keeps the existing value vocabulary and must not invent a partial validation policy inside surface publication.

## Invalidation and lifetime

The initial implementation may rebuild the resolved surface tree for every explicit publication call.

A future retained runtime may cache style resolution and layout. Cache invalidation would be required when any of these change:

```text
root element tree
StyleIntent
active token values
interaction state
surface constraints
layout metrics
```

No cache should be introduced before those invalidation inputs are represented explicitly. Correct single-source publication comes before optimization.

## Ownership

### `runenui_core`

Owns:

```text
StyleIntent
StyleTokens
ComputedStyle
StyleResolution
StyleProvenance
UnresolvedStyleToken
pure resolve_style
```

Core does not know about mounted surfaces, runtime node IDs, layout traversal, or renderers.

### `runenui_runtime`

Owns:

```text
SurfaceBuildContext
surface preparation orchestration
runtime node alignment
layout consumption of ComputedStyle
SurfacePublication
SurfaceFrame
SurfaceStyleReport
```

Runtime does not draw pixels and does not resolve renderer-specific materials.

### Renderer backends

Consume `SurfaceFrame` and concrete `ComputedStyle` values. They do not consume token maps, recipes, provenance, or missing-token diagnostics as rendering input.

## Crate boundary decision

Do not add `runenui_style`, `runenui_layout`, or `runenui_render` for this cutover.

The integration creates real pressure at those boundaries, but the current implementation still has only one small resolver, one placeholder row/column layout pass, and one surface protocol consumer. Keep the change inside `runenui_core` and `runenui_runtime`, then revisit extraction after computed style materially affects layout and at least one renderer-facing style field is consumed.

## Implementation sequence

### Slice 1: unified surface publication — implemented

The shared runtime preparation and publication seam is implemented without changing layout geometry.

Required outcomes:

```text
- one StyleResolution per node per publication
- one runtime node identity assignment for publication products
- ComputedStyle carried by SurfaceNode
- SurfaceStyleReport derived from the same resolution product
- explicit StyleTokens and layout metrics input
- old parallel public functions removed or internalized
- deterministic frame and report tests
```

### Slice 2: computed padding affects layout

Apply the padding box model and clean up conflicting button metrics.

Required tests:

```text
- literal padding changes text, button, and container bounds
- token-resolved padding produces the same geometry as the literal value
- asymmetric padding changes content origin and outer size correctly
- root padding offsets children inside the fixed root bounds
- missing padding token produces zero insets plus diagnostics
- hit testing uses padded outer bounds
```

### Slice 3: boundary review

After padding is integrated:

- update the layout extraction review with actual dependency pressure;
- decide whether the current runtime module remains sufficient;
- do not extract a crate unless the documented extraction criteria are met.

## Non-goals

This integration does not add:

- component recipes;
- variants;
- hovered, focused, pressed, active, or disabled style layers;
- theme selection or inheritance;
- external theme files or serialization;
- fallback token chains;
- CSS cascade or selectors;
- renderer backend implementations;
- accessibility tree output;
- text shaping or real font measurement;
- layout crate extraction;
- render crate extraction;
- retained style caches.

## Acceptance criteria

The design is implemented when all of these are true:

1. Surface publication has one public runtime path.
2. Every node is style-resolved once per publication.
3. `SurfaceFrame` and `SurfaceStyleReport` share runtime node identity and resolution source.
4. `SurfaceNode` exposes concrete `ComputedStyle` without token or provenance data.
5. Layout receives computed style through runtime preparation.
6. Computed padding affects measurement and child placement according to the documented box model.
7. Missing tokens remain diagnosable and non-fatal.
8. No renderer, recipe, theme, interaction-state, or new-crate scope is introduced.
