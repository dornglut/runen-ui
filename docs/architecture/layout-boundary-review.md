# Layout Boundary Review

This document reviews whether the current layout and surface-frame code should be extracted from `runenui_runtime` into a dedicated `runenui_layout` crate.

## Decision

Do not extract `runenui_layout` yet.

The current layout code is still a small runtime publication pass. It is useful and should stay explicit, but it is not yet an independently valuable crate boundary. Extracting it now would mostly move names around without improving dependency enforcement, optionality, or testability.

Keep the implementation in `runenui_runtime` until layout becomes more than simple row/column surface-frame construction.

## Current implementation

The current surface module owns three responsibilities:

1. renderer-facing surface-frame data types;
2. simple row/column layout from `Element<Action>` into `SurfaceFrame`;
3. bounds-based hit testing on the published frame.

Current public surface vocabulary:

```text
LogicalSize
LogicalRect
SurfaceNodeKind
SurfaceNode
SurfaceFrame
SurfaceLayoutMetrics
layout_surface
layout_surface_with_metrics
```

`SurfaceFrame` is not a renderer backend. It is an ordered, host-neutral frame snapshot containing runtime node identity, optional authored IDs, logical bounds, and renderer-facing node kinds.

The current layout pass is intentionally simple:

```text
Element<Action>
  -> SurfaceLayoutBuilder
  -> Vec<SurfaceNode>
  -> SurfaceFrame
```

It assigns bounds to containers, text, and buttons using placeholder intrinsic metrics. It understands only row/column stacking, authored gap, text length approximation, button label approximation, and button minimum size.

## Current ownership

### `runenui_core`

Owns the neutral authored UI model:

```text
Element<Action>
ElementKind<Action>
LayoutStyle
Axis
Px
ElementId
```

Core should continue to own authored layout intent such as axis and gap while those concepts are part of element description.

### `runenui_runtime`

Currently owns surface publication because it already owns:

```text
AppRuntime<App>
Runtime<State, Action>
RuntimeNodeId
RuntimeTreeIndex
input targeting
focus policy
activation policy
trace targets
```

`AppRuntime::surface_frame` is the current publication seam. It builds a renderer-facing frame from the current root tree and a surface size.

### Debug rendering

`debug.rs` consumes `SurfaceFrame` and renders deterministic text for tests and diagnostics. It is not a pixel renderer and does not define a renderer backend abstraction.

## Why extraction is premature

### 1. The algorithm is still a placeholder

The current layout algorithm does not yet have constraints, flex, grid, stack, text shaping, wrapping, min/max sizes, alignment, percentage units, intrinsic measurement contracts, overflow, clipping, or style-driven layout.

A crate boundary would imply a stable layout API before there is enough layout behavior to justify that stability.

### 2. Surface-frame data and layout are still coupled

`surface.rs` currently combines:

```text
geometry types
surface-frame node data
hit testing
simple layout
surface kind extraction
```

This is acceptable as a module. It is not yet clean enough to split into `runenui_layout` and `runenui_render` without deciding which crate owns `LogicalRect`, `SurfaceFrame`, and hit testing.

### 3. Runtime node identity is part of layout output

Surface nodes carry `RuntimeNodeId` and parent runtime IDs. That makes the current layout pass tightly coupled to runtime tree indexing and input targeting.

A future layout crate can still use runtime node identity, but that contract should be made explicit only after the runtime/render/input relationship is more stable.

### 4. Text measurement is fake

`SurfaceLayoutMetrics` currently approximates text and button sizes from character counts. A real layout crate needs a measurement seam for text, font metrics, text runs, wrapping, and possibly host/backend-provided measurement.

Until that seam exists, a layout crate would mostly encode temporary measurements as public API.

### 5. Style does not exist yet

Real layout will need style input:

```text
padding
margin
border
width/height
min/max sizes
alignment
positioning
overflow
display/layout mode
```

Those are not in the current style model. Extracting layout before style risks creating the wrong API shape.

## Future `runenui_layout` ownership

A dedicated layout crate should own layout behavior once the boundary is real:

```text
constraints
measurement requests
measurement responses
intrinsic sizing
row/column/flex/grid/stack algorithms
computed layout boxes
layout diagnostics
layout tests/conformance cases
```

It should not own app state, action dispatch, focus, activation, concrete renderers, native windows, or product UI.

## Future `runenui_render` ownership

A render protocol crate should own renderer-neutral output once the frame protocol is larger than the current surface snapshot:

```text
surface frames
paint primitives
clips
transforms
z-order
text runs
image references
resource handles
frame metadata
```

Backends such as WGPU, SDF, or Runenwerk adapters would consume this protocol. They must not own UI behavior.

## What should stay in runtime for now

Keep these in `runenui_runtime` for now:

```text
LogicalSize
LogicalRect
SurfaceNodeKind
SurfaceNode
SurfaceFrame
SurfaceLayoutMetrics
layout_surface
layout_surface_with_metrics
hit testing on SurfaceFrame
AppRuntime::surface_frame
AppRuntime::surface_frame_with_metrics
DebugSurfaceRenderer
```

This keeps the current interaction pipeline simple:

```text
AppRuntime
  -> current root Element tree
  -> surface frame
  -> debug renderer / future backend
  -> hit test
  -> target RuntimeNodeId
  -> input policy / activation
```

## Extraction criteria

Create `runenui_layout` only when at least three of these are true:

1. Layout has multiple algorithms beyond row/column stacking.
2. Layout accepts explicit constraints instead of only a root frame size.
3. Measurement is abstracted behind a real text/control measurement contract.
4. Style or computed style materially affects layout.
5. Layout diagnostics or conformance tests are valuable outside runtime tests.
6. A renderer, host adapter, or Runenwerk integration needs to consume layout output independently.
7. Moving layout to a crate enforces a dependency boundary Cargo should protect.

Create `runenui_render` only when at least three of these are true:

1. `SurfaceFrame` grows into a richer renderer-neutral frame protocol.
2. There are paint primitives beyond text/button/container node descriptions.
3. At least two backends or debug renderers consume the same protocol.
4. Render resources, clips, transforms, z-order, or text runs need stable ownership.
5. Runtime publication and backend consumption need a Cargo-enforced boundary.

## Recommended next implementation slice

Do not extract crates next.

The next implementation slice should introduce a small style token vocabulary in the existing crates, then route those style values into surface-frame publication. That will expose the real pressure between core, runtime, layout, and render.

Recommended next PR:

```text
PR #43: Add initial style token vocabulary
```

Initial scope:

```text
- add explicit style/property vocabulary without CSS cascade
- keep it host- and renderer-neutral
- attach style intent to elements or element arguments
- keep layout behavior mostly unchanged
- add tests proving style metadata survives element construction
```

Non-goals:

```text
- no CSS parser
- no theme system yet
- no layout extraction
- no render extraction
- no backend work
- no accessibility extraction
```

## Review cadence

Revisit this boundary after style tokens and at least one more layout-affecting feature exist.

The expected progression is:

```text
style token vocabulary
  -> computed style shape
  -> layout-affecting style fields
  -> layout boundary review update
  -> possible runenui_layout extraction
  -> render protocol boundary review
  -> possible runenui_render extraction
```
