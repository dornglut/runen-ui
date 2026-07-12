# Layout and Measurement Architecture

> **Category: Target architecture**
>
> Current implementation facts and production targets are separated below.

## Current contract

The current headless proof provides:

- normalized independent minimum/maximum constraints with finite and unbounded maxima;
- a borrowed synchronous renderer-neutral `MeasurementProvider` for text and control labels;
- deterministic Unicode-scalar-count measurement for tests and headless examples;
- computed padding applied through outer/content box constraints;
- one `RuntimeNodeId`-aligned measured result per node per publication;
- one intrinsic-measurement snapshot per node and one child-layout snapshot per
  child-bearing node per publication;
- measurement-free arrangement from those exact publication-local snapshots;
- intrinsic main-axis row/column sizing and loose finite cross-axis maxima;
- aligned desired/constrained size and overflow diagnostics.
- open intrinsic widget measurement for fixed size, text, and unsupported
  capabilities, independent from open linear child layout;

Surface preparation queries `Widget::measure()` exactly once per node and
`ChildLayoutWidget::child_layout()` exactly once per child-bearing node per
publication. The resolved tree owns both values. Intrinsic sizing, child
measurement, arrangement, and layout diagnostics reuse them; text descriptors
are not regenerated. A later publication queries each capability once again.
These snapshots are transient publication data, not persistent mounted state.

For a child-layout widget, the M2 proof combines its intrinsic minimum with its
measured child-layout content using component-wise maximum. It then constrains
content, expands padding, and applies outer constraints. A default container has
zero intrinsic minimum; fixed and text minimum panels can enlarge child content.
This deterministic rule is not the final M7 custom-layout policy.

`WidgetMeasure::Unsupported` produces a deterministic
`runenui.measurement.unsupported` layout diagnostic. The runtime's required
cross-crate wildcard produces `runenui.measurement.unrecognized` for a newer
unknown capability. Both use explicit zero fallback geometry only alongside the
diagnostic; unknown behavior is never silently treated as ordinary zero size.
The core and runtime measurement vocabularies both call generic control text
`ControlLabel`; no button-specific alias is retained.

`ChildLayout::Linear { axis }` is the current child policy. A future unknown
variant produces `runenui.child-layout.unrecognized`, uses a vertical linear
fallback, and still measures, arranges, frames, styles, and publishes every
child. Layout diagnostics are the ordered `SurfaceLayoutNode::diagnostics()`
collection.

The box order is:

```text
outer constraints
  -> subtract computed padding
  -> content constraints
  -> max(intrinsic widget minimum, measured child-layout content)
  -> constrain content
  -> add padding
  -> constrain outer size
```

Provider sizes are structurally finite and non-negative. Authored invalid geometry is rejected; valid finite arithmetic that overflows during measurement, padding expansion, arrangement cursors, constraint subtraction, or derived rectangle-edge calculation saturates at a finite boundary. Overflow is diagnostic only: the current algorithm does not clip or scroll. Button minimum dimensions are temporary private runtime policy, not a production control recipe.

The deterministic measurement provider is explicitly a test/headless proof.
Character counting is not production text geometry. `WidgetMeasure` is the
bounded M2 participation proof; it does not freeze the M7 production custom
layout contract.

## Current limitations

There is no complete sizing/min/max/fill/shrink vocabulary, flex/grid, main/cross alignment, baseline use, wrapping, stack/absolute/overlay layout, margin/border box, aspect ratio, clipping, scrolling, transforms, virtualization, retained cache, or incremental invalidation. M1 now enforces finite non-negative `LogicalLength`/`LogicalSize` values, fallible finite signed points, normalized constraints, finite saturating current-layout arithmetic (including generated bounds and hit-test edges), and validated baselines; broader sizing behavior remains M7 work.

## Production contract

RunenUI owns public layout semantics, logical geometry, constraints, measurement inputs/results, diagnostics, custom layout extension points, and conformance tests. Hosts/resource providers may supply measurement facts; renderers do not own layout policy.

The production layout system must support normal responsive applications and tools: sizing and min/max, fill/shrink, flex/alignment/baselines/wrap, stack and absolute/overlay positioning, full box model, clipping, scrolling and extents, transforms, incremental invalidation, and custom layouts.

Text measurement evolves through the M8 text subsystem with typography, shaping, wrapping, baselines, resource identity, and invalidation inputs. No cache may hide inputs that are not explicit.

## Algorithm decision

Before flex/grid expansion, an ADR must compare a custom engine, a mature algorithm such as Taffy behind adapters, and a hybrid approach. RunenUI’s public tree and vocabulary must remain independent of an internal dependency.

## Crate boundary

Layout remains in `runenui_runtime`; see [ADR 0002](../adr/0002-keep-layout-in-runtime.md). Extraction requires identity-independent inputs/outputs, explicit geometry ownership, real typography/resource inputs, a conformance harness independent of full application publication, and hard pressure from an independent consumer, dependency rule, multiple substantial algorithms, or meaningful optionality.

Do not genericize identity or split publication solely to manufacture a crate.
