# Layout and Measurement Architecture

> **Category: Current architecture**

This document describes the layout and measurement behavior that exists in the current framework foundation. The general layout algorithm remains proof-level; ADR 0009 owns the accepted M8C production-layout target and its integration with the production text boundary established by M8B.

## Current ownership

`runenui_core` owns host-neutral logical geometry, authored layout/style values, and public widget measurement/child-layout participation vocabulary. `runenui_runtime` owns generic `LayoutConstraints`, mounted capability caching, measurement orchestration, arrangement, invalidation, publication-local layout products, diagnostics, and hit/semantic/paint dependencies derived from final logical geometry. `runenui_text` owns renderer-neutral text-specific constraints, font/shaping/line-breaking computation, reusable logical text-layout state, and immutable text artifacts/resources.

Renderers do not own layout or text-measurement policy. The wgpu renderer receives already-shaped logical text resources through retained paint publication and owns only disposable SDF/MSDF realization.

## Current behavior

The current implementation provides:

- normalized independent minimum/maximum runtime constraints with finite and unbounded maxima;
- validated finite logical geometry and saturating derived arithmetic at finite boundaries;
- state-aware widget intrinsic measurement through RunenUI-owned `WidgetMeasure` capabilities and open linear child-layout participation;
- production text measurement through the runtime-owned `TextSystem`, lowering runtime horizontal availability into renderer-neutral `TextConstraints` and consuming the resulting immutable `TextArtifact` size;
- topology-aligned retained `TextLayoutState` so compatible text requests can reuse shaping or re-line-break through `runenui_text` without becoming mounted authority;
- one logical text artifact/result as the source of both measured text metrics and the exact shaped resources later projected into paint;
- persistent selective layout/text state compatible with staged publication;
- intrinsic row/column main-axis sizing, loose finite cross-axis maxima, gaps, padding, and deterministic overflow/unsupported diagnostics;
- aligned mounted-order layout products used by hit testing, semantic bounds, directional focus geometry, and paint placement.

There is no production scalar-count/fixed-width text estimator or caller-owned text measurement provider. Non-text widget measurement remains expressed through the current RunenUI widget capability vocabulary; broader production intrinsic/custom measurement is owned by M8C.

`LAYOUT` invalidation clears or recomputes the required derived layout/text state and schedules dependent geometry work. Compatible authored style/layout changes are read from current mounted state; retained topology does not own stale layout/style authoring values. Paint-only text foreground remains outside shaped identity, while metric typography participates in text compatibility.

## Current box rule

The proof-level general box calculation remains:

```text
outer constraints
  -> subtract computed padding
  -> content constraints
  -> measure intrinsic widget/text and child-layout content
  -> constrain content
  -> add padding
  -> constrain outer size
```

For text measurement, the finite horizontal content maximum is lowered to `TextConstraints::limited`; an unbounded maximum becomes `TextConstraints::unbounded`. The resulting `TextArtifact` supplies the exact paragraph size. For child-layout widgets, a default container has zero intrinsic minimum; a widget's intrinsic minimum can enlarge its child content. Unsupported or unrecognized capabilities produce deterministic diagnostics with explicit fallback rather than being silently interpreted as ordinary zero-size behavior.

## Current limitations

This is not yet a production general-purpose layout engine. It does not provide complete sizing/min/max/fill/shrink semantics, Block/Flex/Grid, full mixed-layout alignment/baselines/wrapping, stack/absolute/overlay positioning, complete box model, scroll/content extents, virtualization, or the accepted retained incremental production layout behavior.

M8C adopts Taffy's low-level/custom-tree algorithms inside runtime without transferring mounted topology or final-geometry authority. It will lower exact known/available-space facts into the same accepted `runenui_text` request seam rather than replacing the M8B text system or introducing an open-ended framework measure-until-stable loop. M8D owns integrated text/layout closure and responsive evidence.

## Extraction rule

Layout remains in `runenui_runtime`; see [ADR 0002](../adr/0002-keep-layout-in-runtime.md). A future crate extraction requires a real ownership/dependency/consumer boundary such as identity-independent inputs/outputs, independently valuable conformance, substantial algorithms, meaningful optionality, or independent consumers. File size alone is not sufficient.

The [roadmap](../roadmap.md) owns sequencing. ADR 0009 owns the accepted production layout/text architecture; this current-architecture document does not create a second design authority.
