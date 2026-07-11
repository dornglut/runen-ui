# Layout Boundary Review

This document reviews whether the current layout and surface-frame code should be extracted from `runenui_runtime` into a dedicated `runenui_layout` crate.

## Decision

Do not extract `runenui_layout` yet.

The current layout code is still a small runtime publication pass. It is useful and should stay explicit, but it is not yet an independently valuable crate boundary. Extracting it now would mostly move names around without improving dependency enforcement, optionality, or testability.

Keep the implementation in `runenui_runtime` until layout becomes more than simple row/column surface-frame construction.

## Current implementation

The current surface module owns four responsibilities:

1. explicit surface build inputs and unified publication;
2. one per-node style-resolution preparation for frame and diagnostics;
3. simple row/column layout into `SurfaceFrame`;
4. bounds-based hit testing on the published frame.

Current public surface vocabulary:

```text
LogicalSize
LogicalRect
SurfaceNodeKind
SurfaceNode
SurfaceFrame
LayoutConstraints
MeasurementProvider
TextMeasurementRequest
TextMeasurement
SurfaceBuildContext
SurfacePublication
publish_surface
```

`SurfaceFrame` is not a renderer backend. It is an ordered, host-neutral frame snapshot containing runtime node identity, optional authored IDs, logical bounds, renderer-facing node kinds, and concrete `ComputedStyle`. `SurfacePublication` carries the frame together with aligned style diagnostics.

The current layout pass is intentionally simple:

```text
Element<Action> + StyleTokens + root constraints + MeasurementProvider
  -> resolved runtime surface tree
  -> SurfaceLayoutBuilder
  -> SurfaceFrame + SurfaceStyleReport
  -> SurfacePublication
```

It assigns bounds to containers, text, and buttons using the provider-backed measurement contract. It understands only row/column stacking, authored gap, provider-measured standalone text, provider-measured button labels, computed padding, root constraints, and a private button minimum-size policy.

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

`AppRuntime::publish_surface` is the current publication seam. It accepts an explicit `SurfaceBuildContext`, then produces one aligned `SurfacePublication`.

### Debug rendering

`debug.rs` consumes `SurfaceFrame` and renders deterministic text for tests and diagnostics. It is not a pixel renderer and does not define a renderer backend abstraction.

## Why extraction is premature

### 1. The algorithm is still a placeholder

The current layout algorithm does not yet have child constraint propagation, flex, grid, stack, text shaping, wrapping, alignment, percentage units, overflow diagnostics, clipping, or broad style-driven layout.

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

### 4. Text measurement is intentionally minimal

The runtime now has a measurement seam for text and button labels, with a deterministic fallback provider for headless tests and examples. A real layout crate still needs richer text inputs, font metrics, text runs, wrapping, and possibly host/backend-provided measurement conformance.

Until those behaviors exist, a layout crate would mostly encode a still-small runtime publication algorithm as public API.

### 5. Computed style now affects geometry

The runtime now prepares one resolved surface tree. `SurfaceFrame` receives concrete `ComputedStyle`, while `SurfaceStyleReport` receives provenance and unresolved-token diagnostics from the same per-node `StyleResolution`.

Computed padding now affects measurement, container content origins, root child placement, outer bounds, and hit testing. This is the first style-driven geometry rule. The implementation remains small and tightly coupled to runtime identity and surface publication, so it does not by itself justify crate extraction. The accepted box-model contract is documented in [Computed Style Runtime Integration](computed-style-runtime-integration.md).

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
LayoutConstraints
MeasurementProvider
TextMeasurementRequest
TextMeasurement
SurfaceBuildContext
SurfacePublication
publish_surface
hit testing on SurfaceFrame
AppRuntime::publish_surface
SurfaceStyleReport
DebugSurfaceRenderer
```

This keeps the current interaction pipeline simple:

```text
AppRuntime
  -> current root Element tree + SurfaceBuildContext
  -> one resolved surface tree
  -> SurfacePublication
  -> frame -> debug renderer / future backend
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

Unified surface publication, computed padding geometry, root constraints, and provider-backed text measurement are implemented. The next step is a measured layout result plus child content-box constraint propagation and overflow diagnostics, not an automatic crate extraction:

```text
resolved ComputedStyle::padding
  -> provider-measured text and button label content
  -> container outer size and child content origin
  -> constrained root outer size
  -> hit testing over padded outer bounds
```

Required scope:

```text
- one measured result shared by measurement and placement
- content-box child constraints
- deterministic overflow diagnostics
- no additional style fields or layout algorithms
```

Non-goals remain:

```text
- no component recipes or variants
- no interaction-state styling
- no external theme format
- no renderer backend
- no layout or render crate extraction
```

## Review cadence

Revisit this boundary now that computed padding is implemented.

The completed and expected progression is:

```text
style token vocabulary
  -> computed style and provenance
  -> unified runtime surface publication
  -> computed padding affects layout
  -> root constraints and provider-backed measurement
  -> measured layout result and child constraints
  -> layout boundary review update
  -> possible runenui_layout extraction
  -> render protocol boundary review
  -> possible runenui_render extraction
```
