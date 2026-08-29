# M8 Conformance Matrix

> **Category:** Target architecture
>
> **Status:** Accepted M8 target contract; all production rows blocked
>
> **Milestone:** M8
>
> **Reviewed baseline:** `1a5af89c1886654d859f56d1d8afe3e46abdcf95`
>
> This matrix is the normative M8 target contract after the exact M8A0
> architecture/conformance package was explicitly accepted by the repository owner,
> squash-merged, and accepted-main validated. Acceptance freezes M8 obligations; it
> does not promote any production row or claim M8 implementation capability.

[ADR 0009](../adr/0009-production-style-layout-text-foundation.md) owns M8
architecture. M3 owns mounted runtime/layout authority and invalidation; M4 owns
interaction/scheduling; M5 owns semantics/testing; M6 owns renderer-neutral paint/hit
publication and `ResourceRef`; M7 owns real renderer/resource/host/accessibility
integration. This matrix references inherited contracts rather than duplicating them.

```text
33 total unique rows
0 owner-accepted
0 implementation-complete
0 proof-complete
33 blocked
0 duplicate IDs
0 invalid statuses
0 invalid schemas
```

All rows are `Required`. Serial delivery after accepted M8A0 is M8A style
resolution, M8B logical text plus SDF/MSDF realization, M8C production runtime
layout/text feedback, then M8D integrated closure.

A second retained UI tree, renderer-owned shaping/layout, core/runtime renderer
registries, private expected-layout models, ambient-system-font deterministic tests,
proof scalar text metrics presented as production behavior, hidden alpha-raster text
fallback, or compatibility shims preserving replaced pre-1.0 authority cannot
satisfy M8.

## M8A — production style environment and resolution

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| M8STYLE-01 | Public authored/resolved style remains RunenUI-owned typed host-neutral vocabulary; no Taffy, Parley, renderer, native, CSS-parser, mounted, or backend type becomes public style authority. | Public-API/dependency corpus | Forbidden-type/dependency and second-style-authority audit | Style schema/provenance inspection | M8A | blocked | Required |
| M8STYLE-02 | Resolution precedence is deterministic and property-local across framework defaults, theme recipe base, ordered variants, canonical interaction-state layers, authored overrides, and mandatory preference overrides. | Precedence permutation corpus | Hash/order-dependent, hidden cascade, or last-writer ambiguity corpus | Exact per-property provenance records | M8A | blocked | Required |
| M8STYLE-03 | Themes, typed recipes, and variants resolve through explicit token environments; missing/invalid tokens diagnose without silent fallback, provider selection, or mutation of authored intent. | Theme/recipe/token corpus | Missing-token fallback/rebinding corpus | Token/provenance diagnostics | M8A | blocked | Required |
| M8STYLE-04 | Hover/focus/active/disabled and other accepted transient style states derive from canonical mounted interaction state; applications do not maintain a second hidden interaction-style state machine. | Routed-state/style corpus | Duplicated app-state/direct-widget-state audit | Interaction-to-style provenance | M8A | blocked | Required |
| M8STYLE-05 | High-contrast and reduced-motion inputs are explicit preference facts with deterministic policy effects; relevant preference changes invalidate only required dependent stages. | Preference matrix | Ignored preference, platform-global read, and invalidate-all audit | Preference revision/invalidation records | M8A | blocked | Required |
| M8STYLE-06 | Only an explicit typed set of properties inherits through runtime topology; geometry/layout properties never inherit accidentally. | Parent/child inheritance corpus | Implicit CSS-like cascade and geometry-inheritance audit | Inheritance provenance | M8A | blocked | Required |
| M8STYLE-07 | Resolved properties classify downstream effects so paint-only changes avoid reshaping/layout when safe while metric/layout changes invalidate every dependent text/layout/paint/hit/semantic fact required for correctness. | Differential invalidation corpus | Under-invalidation and unconditional invalidate-all corpus | Property-effect/invalidation reports | M8A | blocked | Required |

## M8B — production logical text and SDF/MSDF realization

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| M8TEXT-01 | `runenui_text` is renderer-neutral and depends only on public core plus reviewed text/font dependencies; it owns shaping/font/logical-resource computation but no mounted/runtime/publication/renderer/host/semantic/editing/GPU-atlas authority. | Cargo/source boundary corpus | Runtime/renderer/private-seam/host/editing/GPU dependency audit | Package ownership report | M8B | blocked | Required |
| M8TEXT-02 | Production shaping/layout uses the accepted Parley 0.11.x / Fontique / HarfRust / Skrifa / ICU stack behind RunenUI-owned contracts; Parley AccessKit/editing types and public upstream style/layout types do not become RunenUI authority. | Dependency/API corpus | Alternate shaper, AccessKit/editing adoption, and upstream-type-leak audit | Text-stack capability/version report | M8B | blocked | Required |
| M8TEXT-03 | Explicit font-source policy supports production discovery/fallback and deterministic bundled-font-only construction; font-source identity/revision participates in text cache compatibility. | Bundled/system/fallback corpus | Ambient-system-font deterministic-test and stale-font-cache corpus | Font selection/fallback/revision records | M8B | blocked | Required |
| M8TEXT-04 | Shaping preserves accepted script/language, Unicode analysis, bidi ordering, grapheme/cluster, normalization, and complex-script behavior under controlled locale/language inputs. | Multiscript/bidi/complex-script corpus | Character-count, codepoint-order, lossy-normalization, and script-fallback corpus | Run/cluster/bidi diagnostics | M8B | blocked | Required |
| M8TEXT-05 | Line breaking, wrapping, alignment, and line/baseline metrics are deterministic for the exact shaped content/style and text-specific available-inline constraints. | Width/wrap/alignment/baseline corpus | Independent metric estimator and unbounded relayout corpus | Line-break/metric records | M8B | blocked | Required |
| M8TEXT-06 | Styled spans separate metric-affecting typography from paint-only state; font/fallback/size/weight/variation/language/features that alter geometry reshape as required while foreground-only changes do not. | Styled-span differential corpus | Color-keyed shaping and metric-change cache-reuse corpus | Reshape/re-linebreak/reuse diagnostics | M8B | blocked | Required |
| M8TEXT-07 | One immutable logical text artifact supplies paragraph size, required line/run/cluster metrics/ranges, exact shaped paint-run `ResourceRef`s/origins, and exact glyph/font/variation bindings from the same shaping/line-break result. | Artifact consistency corpus | Measure-then-reshape and independently minted paint-resource corpus | Artifact/run/glyph/resource records | M8B | blocked | Required |
| M8TEXT-08 | Text uses a renderer-neutral text-specific constraint projection rather than runtime `LayoutConstraints`; equal text/style/font-revision/constraint requests are deterministic and width-only reflow may re-linebreak without reshaping when valid. | Request/cache/reflow corpus | `runenui_text -> runtime` dependency and hidden constraint-state audit | Request key/reflow/reshape records | M8B | blocked | Required |
| M8TEXT-09 | Each live shaped `ResourceRef` retains one immutable logical shaped-content binding independent of raster scale for as long as any cache/measurement/publication may use it; pruning cannot invalidate retained-publication retry. | Resource lifetime/drop/retry/scale corpus | Live-ref eviction, scale-bound identity, rebinding, and split-key corpus | Text-resource lifetime/cache records | M8B | blocked | Required |
| M8TEXT-10 | Supported outline glyphs are realized by `runenui_render_wgpu` through an SDF/MSDF-family atlas/shader path from exact already-shaped outline data; generator, atlas, field quality, device cache, and scale tiers are renderer-owned, while shaping/line breaking/logical metrics remain unchanged and no hidden alpha-raster production fallback is used. | Multi-scale SDF/MSDF pixel/golden/cache corpus | Renderer shaping/layout, logical-scale mutation, and alpha-fallback audit | Field-generation/atlas/quality/resource records | M8B | blocked | Required |
| M8TEXT-11 | COLR/SVG/bitmap/intrinsic-color glyphs are never silently flattened into ordinary foreground SDF/MSDF or alpha semantics; unsupported breadth diagnoses explicitly unless a separately accepted resource/paint revision represents it truthfully. | Color/bitmap glyph capability corpus | Silent monochrome/raster reinterpretation corpus | Unsupported-capability diagnostics | M8B | blocked | Required |

## M8C — production runtime layout and text feedback

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| M8LAYOUT-01 | Runtime uses Taffy low-level/custom-tree algorithms over exact mounted topology/resolved style; no `TaffyTree` or other second retained UI topology/identity/lifecycle authority exists. | Custom-tree integration corpus | Retained-Taffy-tree/topology-copy/private-ID audit | Algorithm node/topology mapping records | M8C | blocked | Required |
| M8LAYOUT-02 | Production sizing implements explicit width/height, min/max, intrinsic/auto, fill/grow, and shrink behavior under normalized finite/unbounded root constraints. | Sizing constraint corpus | NaN/inverted/overflow and proof-size fallback corpus | Constraint/used-size diagnostics | M8C | blocked | Required |
| M8LAYOUT-03 | Flex layout preserves authored order, direction, grow/shrink/basis, gaps, and main/cross alignment under nested constraints without ad hoc widget-kind geometry. | Flex corpus | Widget-kind/layout-special-case audit | Flex algorithm/layout records | M8C | blocked | Required |
| M8LAYOUT-04 | Grid layout preserves explicit/implicit tracks, gaps, placement/spans, and intrinsic contribution behavior for the accepted M8 grid subset. | Grid corpus | Ad hoc table/grid reinterpretation and hidden-track-state audit | Grid track/placement records | M8C | blocked | Required |
| M8LAYOUT-05 | Block/flow behavior, wrapping, alignment, and baselines consume exact child/text measurements and remain deterministic across nested mixed layout modes. | Mixed block/flow/wrap/baseline corpus | Scalar-text-metric and independent-baseline audit | Flow/baseline records | M8C | blocked | Required |
| M8LAYOUT-06 | Box model, stack/absolute/overlay placement, and clipping are explicit typed layout/style behavior; transformed/positioned geometry remains aligned with inherited paint/hit semantics rather than becoming renderer policy. | Box/position/clip corpus | Renderer geometry and hidden absolute-position state audit | Box/placement/clip diagnostics | M8C | blocked | Required |
| M8LAYOUT-07 | Overflow produces exact inspectable scroll/content extents independent of later platform scroll mechanics; clipping/extent facts are not guessed from raster output. | Overflow/extent corpus | Raster-derived extent and silent overflow-loss corpus | Overflow/extent reports | M8C | blocked | Required |
| M8LAYOUT-08 | Open widgets can contribute production intrinsic/custom measurement through bounded renderer-neutral contracts; measurement cannot mutate runtime, forge identity, inspect private topology, or install a second layout engine. | Downstream custom-measure corpus | Private-runtime/reentrant/mutating-measure audit | Measurement call/result diagnostics | M8C | blocked | Required |
| M8LAYOUT-09 | Runtime owns incremental layout invalidation/cache compatibility and final logical geometry; derived Taffy/text caches are disposable and the exact final geometry remains one aligned authority for paint, hit, focus, and semantic bounds. | Differential dirty/recompute/alignment corpus | Parallel expected-layout/cache-authority and stale-geometry corpus | Surface phase/cache/invalidation reports | M8C | blocked | Required |

## M8D — integrated production closure

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| M8INTEG-01 | Taffy leaf measurement lowers exact known/available-space facts into deterministic text-layout requests; text metrics feed the same bounded layout computation with no open-ended framework measure-until-stable loop. | Width-feedback/convergence corpus | Independent text/layout loop and unbounded iteration corpus | Measure-call/reflow/layout-phase records | M8D | blocked | Required |
| M8INTEG-02 | The exact logical text artifact/resource facts used for measurement reach owner-local paint contribution; final paint never independently reshapes/rebreaks or remints the same authored text/span set. | Measurement-to-paint identity corpus | Paint-time remint/reshape and stale-artifact corpus | Node/artifact/resource correlation records | M8D | blocked | Required |
| M8INTEG-03 | Deterministic public headless tests use bundled fonts, fixed locale/preferences/constraints, and ordinary public runtime/text contracts; no ambient system font, private expected runtime, alternate layout engine, or software expected renderer is required. | Headless production-contract corpus | Ambient-font/private-model/alternate-engine audit | Fixture/provider/version diagnostics | M8D | blocked | Required |
| M8INTEG-04 | Semantic text/content and bounds remain owned by accepted semantic publication and exact final layout; text/layout integrations neither allocate semantic identity nor introduce a second accessibility tree/action path. | Text semantic/bounds corpus | Parley-AccessKit, glyph-derived semantic identity, and stale-bounds audit | Semantic/layout correlation records | M8D | blocked | Required |
| M8INTEG-05 | Real wgpu offscreen/native proof renders the exact logical shaped resources produced by production measurement/layout through the accepted SDF/MSDF path, including retained-publication retry and raster-scale/quality changes, without renderer-owned shaping/layout. | Integrated responsive/text-heavy SDF/MSDF render corpus | Debug/software/renderer-shaping, alpha-fallback, and resource-rebinding audit | Runtime/text/renderer correlation records | M8D | blocked | Required |
| M8INTEG-06 | Proof-era scalar-count text measurement, linear-only layout authority, and M7 scale-specific alpha `ShapedRunRaster` production authority are removed after cutover; current docs/status/API expose one production path and no compatibility bridge silently preserves replaced authority. | Source/API/authority cleanup corpus | Duplicate-provider/legacy-layout/alpha-text/compatibility-shim audit | Repository authority and deprecation audit | M8D | blocked | Required |

## Closure rule

M8 closes only after all 33 rows are `owner-accepted` on accepted default branch and
final M8 closure reconciliation is itself accepted-main validated. Later M9/M10 work
does not justify weakening M8 style/layout/text ownership; M10 editing builds on the
accepted M8 text artifacts rather than replacing their shaping/layout authority.
