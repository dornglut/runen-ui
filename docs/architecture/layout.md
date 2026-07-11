# Layout and Measurement Architecture

> **Category: Target architecture**
>
> Current implementation facts and production targets are separated below.

## Current contract

The current headless proof provides:

- normalized independent minimum/maximum constraints with finite and unbounded maxima;
- a borrowed synchronous renderer-neutral `MeasurementProvider` for text and button labels;
- deterministic Unicode-scalar-count measurement for tests and headless examples;
- computed padding applied through outer/content box constraints;
- one `RuntimeNodeId`-aligned measured result per node per publication;
- measurement-free arrangement;
- intrinsic main-axis row/column sizing and loose finite cross-axis maxima;
- aligned desired/constrained size and overflow diagnostics.

The box order is:

```text
outer constraints
  -> subtract computed padding
  -> content constraints
  -> measure content
  -> constrain content
  -> add padding
  -> constrain outer size
```

Invalid provider sizes are sanitized before geometry use. Overflow is diagnostic only: the current algorithm does not clip or scroll. Button minimum dimensions are temporary private runtime policy, not a production control recipe.

The deterministic measurement provider is explicitly a test/headless proof. Character counting is not production text geometry.

## Current limitations

There is no complete sizing/min/max/fill/shrink vocabulary, flex/grid, main/cross alignment, baseline use, wrapping, stack/absolute/overlay layout, margin/border box, aspect ratio, clipping, scrolling, transforms, virtualization, retained cache, or incremental invalidation. M1 now enforces finite non-negative `LogicalLength`/`LogicalSize` values, fallible finite signed points, normalized constraints, saturating current-layout arithmetic, and validated baselines; broader sizing behavior remains M7 work.

## Production contract

RunenUI owns public layout semantics, logical geometry, constraints, measurement inputs/results, diagnostics, custom layout extension points, and conformance tests. Hosts/resource providers may supply measurement facts; renderers do not own layout policy.

The production layout system must support normal responsive applications and tools: sizing and min/max, fill/shrink, flex/alignment/baselines/wrap, stack and absolute/overlay positioning, full box model, clipping, scrolling and extents, transforms, incremental invalidation, and custom layouts.

Text measurement evolves through the M8 text subsystem with typography, shaping, wrapping, baselines, resource identity, and invalidation inputs. No cache may hide inputs that are not explicit.

## Algorithm decision

Before flex/grid expansion, an ADR must compare a custom engine, a mature algorithm such as Taffy behind adapters, and a hybrid approach. RunenUI’s public tree and vocabulary must remain independent of an internal dependency.

## Crate boundary

Layout remains in `runenui_runtime`; see [ADR 0002](../adr/0002-keep-layout-in-runtime.md). Extraction requires identity-independent inputs/outputs, explicit geometry ownership, real typography/resource inputs, a conformance harness independent of full application publication, and hard pressure from an independent consumer, dependency rule, multiple substantial algorithms, or meaningful optionality.

Do not genericize identity or split publication solely to manufacture a crate.
