# Layout and Measurement Architecture

> **Category: Current architecture**

This document describes the layout and measurement behavior that exists in the current headless framework foundation. Production layout expansion belongs to the roadmap and requires its own accepted design decision before replacing this contract.

## Current ownership

`runenui_core` owns host-neutral logical geometry, constraints, measurement inputs/results, authored layout intent, and the public widget participation vocabulary. `runenui_runtime` owns mounted capability caching, measurement orchestration, arrangement, invalidation, publication-local layout products, diagnostics, and hit/semantic/paint dependencies derived from layout.

Renderers do not own layout policy. Measurement providers may supply host/resource-dependent measurement facts through the public renderer-neutral provider seam; they do not gain mounted or runtime mutation authority.

## Current proof behavior

The current implementation provides:

- normalized independent minimum/maximum constraints with finite and unbounded maxima;
- validated finite logical geometry and saturating derived arithmetic at finite boundaries;
- a borrowed synchronous `MeasurementProvider` with explicit cache identity and behavior revision;
- deterministic Unicode-scalar-count text/control measurement for tests and headless examples only;
- state-aware widget intrinsic measurement and open linear child-layout participation;
- persistent selective measurement and child-layout capability caches;
- one publication-local measurement snapshot reused by arrangement;
- intrinsic row/column main-axis sizing, loose finite cross-axis maxima, gaps, padding, and deterministic overflow/unsupported diagnostics;
- aligned mounted-order layout products used by hit testing, semantic bounds, directional focus geometry, and paint placement.

Provider identity or behavior revision must change whenever provider behavior changes. Publication context changes may recompute layout from clean widget capability facts; ordinary clean publication does not re-enter widget measurement merely to reproduce an unchanged capability description.

`LAYOUT` invalidation clears measurement and child-layout capability caches and schedules dependent geometry work. Compatible authored style/layout changes are read from current mounted state; retained topology does not own stale layout/style authoring values.

## Current box rule

The proof-level box calculation is:

```text
outer constraints
  -> subtract computed padding
  -> content constraints
  -> max(intrinsic widget minimum, measured child-layout content)
  -> constrain content
  -> add padding
  -> constrain outer size
```

For child-layout widgets, a default container has zero intrinsic minimum; a widget's intrinsic minimum can enlarge its child content. Unsupported or unrecognized capability produces a deterministic diagnostic with an explicit fallback rather than being silently interpreted as ordinary zero-size behavior.

## Current limitations

This is not a production general-purpose layout engine. It does not currently provide complete sizing/min/max/fill/shrink semantics, flex/grid, full alignment/baselines/wrapping, stack/absolute/overlay positioning, full box model, scrolling/extents, transforms, virtualization, or the retained incremental production layout system targeted later.

The deterministic scalar-count provider is test/headless infrastructure, not production text geometry. Production typography and shaping belong to the text subsystem rather than being hidden inside this proof provider.

## Extraction rule

Layout remains in `runenui_runtime`; see [ADR 0002](../adr/0002-keep-layout-in-runtime.md). A future crate extraction requires a real ownership/dependency/consumer boundary such as identity-independent inputs/outputs, independently valuable conformance, substantial algorithms, meaningful optionality, or independent consumers. File size alone is not sufficient.

The [roadmap](../roadmap.md) owns production layout/style sequencing. Any adopt-versus-build decision for a broader layout engine requires an accepted ADR/design rather than being pre-decided by this current-architecture document.
