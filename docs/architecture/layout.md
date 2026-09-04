# Layout and Measurement Architecture

> **Category: Current architecture**

This document describes the layout and measurement behavior in the current framework foundation. Runtime owns layout orchestration and final geometry; private Taffy low-level algorithms provide Block/Flex/Grid computation, and ADR 0009 owns the production layout/text boundary established by M8B.

## Current ownership

`runenui_core` owns host-neutral logical geometry, authored layout/style values, and the bounded public widget measurement/child-bearing vocabulary. `runenui_runtime` owns generic `LayoutConstraints`, mounted topology, Taffy lowering and disposable caches, measurement dispatch, invalidation, publication-local layout products, diagnostics, overflow extents, and hit/semantic/paint dependencies derived from final logical geometry. `runenui_text` owns renderer-neutral text-specific constraints, font/shaping/line-breaking computation, reusable logical text-layout state, and immutable text artifacts/resources.

Renderers do not own layout or text-measurement policy. The wgpu renderer receives already-shaped logical text resources through retained paint publication and owns only disposable SDF/MSDF realization.

## Current behavior

The current implementation provides:

- normalized independent minimum/maximum runtime constraints with finite and unbounded maxima;
- validated finite logical geometry and saturating derived arithmetic at finite boundaries;
- state-aware widget intrinsic measurement through bounded RunenUI-owned `WidgetMeasure` capabilities and geometry-neutral child-bearing participation;
- production text measurement through the runtime-owned `TextSystem`, lowering runtime horizontal availability into renderer-neutral `TextConstraints` and consuming the resulting immutable `TextArtifact` size;
- topology-aligned retained `TextLayoutState` so compatible text requests can reuse shaping or re-line-break through `runenui_text` without becoming mounted authority;
- one logical text artifact/result as the source of both measured text metrics and the exact shaped resources later projected into paint;
- persistent selective layout/text state compatible with staged publication;
- Taffy-backed Block/Flex/Grid sizing, nested normalized layout modes, gaps, padding, positioning, baseline propagation, and deterministic overflow/unsupported diagnostics;
- one-cell Grid lowering for RunenUI Overlay semantics, including inspectable layout/content/scrollable extents;
- aligned mounted-order layout products used by hit testing, semantic bounds, directional focus geometry, and paint placement.

There is no production scalar-count/fixed-width text estimator or caller-owned text measurement provider. Non-text widget measurement remains expressed through the current bounded RunenUI widget capability vocabulary; broader responsive/text-heavy closure remains an M8D concern.

`LAYOUT` invalidation clears or recomputes the required derived layout/text state and schedules dependent geometry work. Compatible authored style/layout changes are read from current mounted state; retained topology does not own stale layout/style authoring values. Paint-only text foreground remains outside shaped identity, while metric typography participates in text compatibility.

## Current box rule

The runtime lowering follows this general box calculation through Taffy's unrounded low-level algorithms:

```text
outer constraints
  -> subtract computed padding
  -> content constraints
  -> request intrinsic widget/text content measurements
  -> constrain content
  -> add padding
  -> constrain outer size
```

For text measurement, finite and semantic minimum/max-content requests are lowered to the corresponding renderer-neutral `TextConstraints`; an unbounded maximum becomes `TextConstraints::unbounded`. The resulting `TextArtifact` supplies exact paragraph size and line baselines. Custom measured sizes are content-box values and custom baselines are translated through resolved padding. Unsupported capabilities produce deterministic diagnostics and a neutral zero intrinsic contribution.

## Current limitations

The current profile intentionally remains bounded: it does not provide virtualization, native scrolling mechanics, browser inline formatting breadth, or the integrated responsive/text-heavy closure owned by M8D. Taffy state is transaction-local and disposable; final logical geometry remains a single runtime-owned publication product.

## Extraction rule

Layout remains in `runenui_runtime`; see [ADR 0002](../adr/0002-keep-layout-in-runtime.md). A future crate extraction requires a real ownership/dependency/consumer boundary such as identity-independent inputs/outputs, independently valuable conformance, substantial algorithms, meaningful optionality, or independent consumers. File size alone is not sufficient.

The [roadmap](../roadmap.md) owns sequencing. ADR 0009 owns the accepted production layout/text architecture; this current-architecture document does not create a second design authority.
