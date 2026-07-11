# Layout Constraints and Measurement Contract

This document defines the next layout boundary for RunenUI after computed padding became real geometry.

It introduces no new crate and no renderer backend. It specifies the neutral inputs and outputs required to replace root-size-only layout and character-count measurement without prematurely stabilizing a full layout engine.

## Problem

The current surface layout pass accepts one root `LogicalSize` and uses fixed placeholder metrics:

```text
surface size
  + text character width/height
  + button character width/minimum size
  -> row/column bounds
```

This is sufficient for deterministic framework proofs, but it cannot represent:

- bounded versus unbounded axes;
- minimum and maximum available sizes;
- intrinsic measurement under width or height pressure;
- text wrapping or backend-neutral text metrics;
- controls whose content measurement is not a character-count approximation;
- diagnostics explaining why a node received a particular size.

The next implementation must introduce these concepts without choosing WGPU, SDF, Winit, a font library, or a dedicated layout crate.

## Decision

Layout should consume explicit constraints and a host-neutral measurement service.

Conceptual pipeline:

```text
resolved surface tree
  + root LayoutConstraints
  + MeasurementProvider
  + placeholder layout policy
  -> measured intrinsic sizes
  -> constrained row/column layout
  -> SurfaceFrame bounds
  -> layout diagnostics
```

The runtime continues to own orchestration and runtime-node alignment. Measurement is supplied through a narrow borrowed interface for one publication call.

## Constraint model

The first constraint type should represent independent minimum and maximum extents:

```text
LayoutConstraints
  min_width
  max_width
  min_height
  max_height
```

Each maximum may be finite or unbounded. Minimum values are finite and non-negative.

A practical target API is:

```rust
pub struct LayoutConstraints {
    min: LogicalSize,
    max: OptionalLogicalSize,
}
```

An equivalent representation using per-axis bounds is acceptable if it preserves the following operations:

```text
constraints.tight(size)
constraints.loose(max_size)
constraints.unbounded()
constraints.constrain(candidate_size)
constraints.constrain_width(width)
constraints.constrain_height(height)
constraints.is_tight_width()
constraints.is_tight_height()
```

The exact public names are not fixed by this document. The semantics are fixed.

## Constraint invariants

For every axis:

```text
0 <= min <= max
```

When maximum is unbounded, every finite candidate remains below it.

Construction must not permit an invalid range. The implementation may normalize or reject invalid input, but the selected policy must be explicit and tested. Silent propagation of `NaN`, negative values, or `min > max` is not acceptable.

The initial implementation should prefer deterministic normalization at the boundary:

- non-finite minimum becomes zero;
- negative minimum becomes zero;
- negative finite maximum becomes zero;
- finite maximum below minimum is raised to minimum;
- positive infinity represents unbounded maximum;
- `NaN` never reaches layout arithmetic.

This policy keeps publication non-panicking while preserving stable geometry. Diagnostics for normalized constraints may be added in the same slice if they remain small; a broad validation framework is not required.

## Root contract

`publish_surface` currently receives a fixed `LogicalSize`. The constraint cutover should replace that root-only assumption with explicit root constraints.

Target conceptual API:

```rust
let constraints = LayoutConstraints::tight(LogicalSize::new(800.0, 600.0));
let context = SurfaceBuildContext::new(&tokens)
    .with_constraints(constraints)
    .with_measurement_provider(&measurement);
let publication = runtime.publish_surface(&context);
```

An intermediate API may keep `size` as a convenience constructor for tight constraints, but there must be one internal constraint path. Do not maintain separate constrained and unconstrained layout algorithms.

The renderer-facing `SurfaceFrame::size()` should be the constrained root outer size selected by layout.

## Measurement contract

Measurement must be renderer-neutral and synchronous for the initial headless runtime.

Target conceptual trait:

```rust
pub trait MeasurementProvider {
    fn measure_text(
        &self,
        request: &TextMeasurementRequest<'_>,
    ) -> TextMeasurement;

    fn measure_control(
        &self,
        request: &ControlMeasurementRequest<'_>,
    ) -> Option<ControlMeasurement>;
}
```

The exact split between text and control methods may change. The fixed decisions are:

- layout requests measurement; it does not inspect renderer internals;
- requests include constraints relevant to the content box;
- responses contain logical sizes and optional baseline facts;
- runtime node IDs are observation metadata, not measurement cache keys;
- the provider must not mutate application state or dispatch actions;
- measurement failure has deterministic fallback behavior;
- the initial interface is borrowed and valid for one publication call.

## Text measurement request

A text request should contain only facts that affect intrinsic text geometry:

```text
text content
available content-box constraints
resolved typography facts, when implemented
wrapping policy, when implemented
optional runtime node ID for diagnostics
```

The current style model does not yet contain typography. The first implementation may therefore request:

```text
content
constraints
node ID
text kind
```

and defer font family, size, weight, line height, shaping options, and locale until those values exist in computed style.

Do not add speculative typography fields solely to make the request appear complete.

## Text measurement response

The minimal response should provide:

```text
logical content size
optional first baseline
optional last baseline
```

Baselines may remain absent in the first code slice if no layout algorithm consumes them. The type should leave a clean extension point without requiring dummy baseline values.

The response describes the content box. Computed padding remains layout-owned and expands the measured content size into the node outer size.

## Control measurement request

Standard controls may eventually need measurement beyond their label text, including indicators, icons, editable regions, or native-feeling minimums.

The first implementation should not invent a generic widget-measurement system. For the existing button proof:

```text
button label
  -> text measurement request
  -> add computed padding
  -> apply explicit minimum outer-size policy
```

A separate control request becomes justified only when a real second control requires non-text intrinsic content. Until then, keep button measurement composed from text measurement plus layout policy.

## Fallback provider

RunenUI must retain a deterministic provider for headless tests and examples.

The current character-count metrics should become an explicit fallback implementation rather than hidden layout behavior:

```text
DeterministicMeasurementProvider
  text_char_width
  text_height
  button_char_width, only if button labels intentionally differ
```

Prefer one text measurement path for text and button labels unless a documented reason requires separate metrics.

The fallback provider is not a renderer and must not become the default truth for production typography.

## Missing or failed measurement

The initial synchronous trait should return a concrete result rather than an error for ordinary text.

If a custom provider cannot measure a request, the runtime should use an explicit fallback provider selected in `SurfaceBuildContext`. It must not:

- panic;
- substitute an arbitrary zero size silently;
- call into a renderer through global state;
- resolve fonts asynchronously inside layout;
- retain references beyond the publication call.

A future asynchronous resource pipeline may invalidate and republish after fonts become available. That is deferred until runtime invalidation and scheduling exist.

## Measurement and padding order

The box-model order is fixed:

```text
outer constraints
  -> subtract computed padding
  -> content constraints
  -> measure intrinsic content
  -> constrain content size
  -> add computed padding
  -> constrain outer size
```

For a tight outer constraint smaller than total padding, content constraints collapse to zero rather than becoming negative.

For the root container, root outer constraints determine the frame size. Root padding reduces the content area available to children.

For nested containers, child constraints are derived from the container content box and the active row/column algorithm.

## Row and column behavior

The first constrained algorithm remains intentionally small.

### Column

A column should:

- measure children in order;
- account for gaps only between successfully measured children;
- provide each child the available content width;
- accumulate child outer heights plus gaps;
- use the maximum child outer width;
- add container padding;
- constrain the final outer size.

### Row

A row should:

- measure children in order;
- account for gaps only between successfully measured children;
- provide each child the available content height;
- accumulate child outer widths plus gaps;
- use the maximum child outer height;
- add container padding;
- constrain the final outer size.

The first implementation does not distribute remaining space, flex children, align cross-axis content, wrap rows, or clip overflow.

## Overflow

Constraints may produce children whose accumulated desired size exceeds the available content box.

The first slice should preserve deterministic bounds and record overflow pressure rather than inventing clipping or scrolling policy.

Target diagnostic facts:

```text
node ID
available content size
desired content size
constrained outer size
overflowed width/height flags
```

A public diagnostics type is optional for the first code slice. At minimum, tests must make overflow behavior explicit.

## Surface build context

`SurfaceBuildContext` should evolve from:

```text
StyleTokens
SurfaceLayoutMetrics
```

toward:

```text
StyleTokens
LayoutConstraints
MeasurementProvider
fallback measurement policy
```

`SurfaceLayoutMetrics` should either:

1. become the configuration of `DeterministicMeasurementProvider`; or
2. be retired after all callers use the provider.

Do not keep metrics both in layout and in the provider. That would preserve two measurement sources.

## Ownership

### `runenui_core`

Continue to own authored layout intent and neutral primitive values. Do not place runtime measurement services in core.

### `runenui_runtime`

Own for now:

- `LayoutConstraints`;
- measurement request/response contracts;
- fallback measurement provider;
- constraint propagation;
- row/column layout orchestration;
- layout diagnostics aligned with `RuntimeNodeId`;
- surface publication.

### Future `runenui_layout`

Reconsider extraction after the constrained implementation exists. Extraction becomes more credible if constraints, measurement contracts, layout results, and conformance tests form an independently useful subsystem.

### Hosts and renderers

A host or renderer integration may implement `MeasurementProvider`, but it does not own layout policy. An SDF and a raster backend should be able to supply equivalent logical measurements to the same runtime contract.

## Invalidation

The initial implementation performs measurement during every explicit publication.

Future caching must invalidate when any measurement input changes:

```text
content
computed typography
constraints
measurement provider configuration
font/resource availability
DPI or scale policy, if logical metrics change
```

No cache should be introduced before those inputs are explicit.

## Implementation sequence

### Slice 1: constraint vocabulary

Add and test:

- normalized `LayoutConstraints`;
- finite/unbounded axis representation;
- constraint application helpers;
- tight root constraints as the compatibility path.

Do not change text measurement yet beyond routing current behavior through constraints.

### Slice 2: measurement provider

Add and test:

- text request and response types;
- borrowed provider contract;
- deterministic fallback provider;
- migration of text and button label measurement out of layout internals;
- one authoritative measurement path.

### Slice 3: constrained row/column layout

Apply:

- content-box constraint derivation after padding;
- constrained text and button sizing;
- constrained container measurement;
- explicit overflow behavior;
- root frame size derived from root constraints.

### Slice 4: boundary review

Re-evaluate `runenui_layout` extraction using the implemented contracts, dependency graph, tests, and prospective independent consumers.

## Required tests

The complete constraints/measurement track should prove:

- tight constraints produce the requested root size;
- loose constraints allow intrinsic shrink-to-fit size;
- unbounded constraints preserve intrinsic size;
- invalid constraints normalize deterministically;
- padding is subtracted before content measurement;
- content constraints never become negative;
- measured text replaces character-count logic in the layout algorithm;
- buttons compose label measurement, padding, and minimum outer size;
- row and column propagate the correct constrained axis;
- overflow behavior is explicit and deterministic;
- custom and deterministic providers produce aligned runtime node geometry;
- no renderer-specific type appears in the public contract.

## Non-goals

This contract does not add:

- flex, grid, wrap, absolute positioning, or alignment;
- clipping, scrolling, or viewport virtualization;
- typography style fields not already required by a caller;
- font loading or shaping implementation;
- asynchronous measurement;
- renderer primitives or backend APIs;
- a `runenui_layout` crate;
- a `runenui_render` crate.
