# ADR 0009: Production style, layout, and text foundation

> **Category:** ADR
>
> **Status:** Accepted target architecture on exact-head owner acceptance
>
> **Decision date:** 2026-08-29
>
> **Milestone:** M8
>
> **Reviewed baseline:** `1a5af89c1886654d859f56d1d8afe3e46abdcf95`
>
> **Acceptance:** this ADR is the accepted M8 target architecture after the exact
> M8A0 package containing it was explicitly accepted by the repository owner,
> squash-merged, and accepted-main validated. Acceptance freezes target decisions;
> it does not claim any M8 production implementation or dependency adoption.

## Context

M7 closed the reference production spine at proof maturity. M8 must replace three
coupled proof-level limitations without moving live authority out of
`runenui_runtime`:

- style lacks production themes, recipes, state layers, preference policy, and
  property breadth;
- layout remains a small linear proof engine;
- text measurement is not bound to one production shaped artifact that supplies both
  final layout metrics and the exact glyph resources later rendered.

Production text must not be measured by one system and independently reshaped during
paint. The same logical shaping/line-breaking result must supply layout metrics and
the exact shaped resources later realized by the renderer.

## Relationship to accepted authority

This ADR preserves these accepted ownership decisions:

- M3 / ADR 0002: mounted topology, layout orchestration, invalidation, and final
  logical geometry remain runtime-owned; an adopted algorithm cannot become a second
  UI tree;
- M4: canonical interaction and scheduling;
- M5: semantic identity/publication/action and deterministic public testing;
- M6: immutable renderer-neutral paint/hit publication, revision/damage, and opaque
  `ResourceRef` identity;
- M7: renderer/resource-provider, raster-scale, host, accessibility, retained
  publication retry, and external-host boundaries;
- M9/M10/M11/M13: broad composition/animation, editable text, standard controls, and
  broad platform/multi-window profiles remain later owners.

**On acceptance, this ADR deliberately supersedes the ADR 0008 shaped-run provider
contract for the M8 production outline-text path:** M7's caller-provided,
scale-specific alpha coverage remains truthful proof-era behavior only until the M8B
cutover is accepted. M8 runtime-shaped text instead retains the exact immutable
logical shaped binding with the paint publication and lets the renderer derive
disposable SDF/MSDF realizations from that binding. External caller-owned resource
providers remain the edge for external resources such as images. All other M7
complete-`ResourceRef`, raster-scale, renderer, host, accessibility, and retained-
publication rules remain inherited.

## Decision

### One staged style/layout/text dependency graph

```text
style environment + authored state
    -> resolved typed style
    -> layout available-space facts
    -> text shaping / line breaking / measurement
    -> final layout geometry
    -> paint / hit / semantics
```

Invalidation flows back through the same graph. Runtime remains the live owner of
stage execution, dirty mounted nodes, cache compatibility, and aligned publication.
No dependency or renderer owns a parallel mounted tree, dirty graph, publication
lifecycle, semantic tree, or scheduler.

### RunenUI owns production style semantics

`runenui_core` keeps host-neutral typed authored/resolved style vocabulary. M8 adds:

- typed literals and token references;
- theme environment and preference-sensitive defaults;
- typed recipes and explicit variants;
- transient interaction-state layers derived from canonical mounted interaction
  state;
- authored per-node overrides;
- an explicit bounded set of inheritable properties, principally foreground and
  typography;
- high-contrast and reduced-motion inputs;
- exact per-property provenance and diagnostics.

Resolution precedence, low to high, is:

1. framework defaults;
2. theme recipe base;
3. selected variants in stable authored order;
4. interaction-state recipe layers in one documented framework order;
5. authored token/literal overrides;
6. mandatory preference policy overrides for governed properties.

Properties are classified by downstream effect so paint-only changes do not force
text/layout work while metric/layout changes invalidate every dependent product.

### Taffy provides algorithms inside runtime

M8 adopts Taffy `0.14.x` for Block, Flexbox, and Grid through its low-level/custom-
tree interfaces over exact runtime-owned topology and resolved RunenUI style.
`TaffyTree` must not become a second retained UI tree.

Taffy state is derived/disposable. Runtime keeps mounted identity, child order,
dirty propagation, measurement dispatch, final logical geometry, cache
compatibility, and publication commit. Public RunenUI APIs expose no Taffy types.

A separate `runenui_layout` crate is not justified by M8A0; ADR 0002 remains the
layout ownership authority.

### `runenui_text` is the renderer-neutral text boundary

M8 introduces `runenui_text` because production text has independent dependency,
resource, computation, reuse, and conformance pressure. It:

- depends on `runenui_core` and the reviewed text/font stack;
- owns font collection/discovery/fallback configuration and immutable font data;
- owns shaping, Unicode/bidi analysis, line breaking, paragraph layout, metrics,
  and deterministic text fixtures;
- owns immutable logical shaped-text resource bindings;
- exposes RunenUI-owned request/result/resource contracts and the immutable
  glyph/font/outline facts needed to realize exact already-shaped resources;
- owns no mounted/runtime/publication authority, SDF/MSDF atlas, GPU state, host,
  semantics, application state, or editing model.

`runenui_runtime` may depend on `runenui_text`; `runenui_text` must not depend on
runtime or `runenui_render_wgpu`.

### Parley provides shaping and paragraph text layout

M8 adopts Parley `0.11.x` with its Fontique/HarfRust/Skrifa/ICU stack for font
selection/fallback, shaping, font data, Unicode analysis, bidi, segmentation, and
line breaking.

No Parley types enter RunenUI's public style/text protocol. Parley's optional
AccessKit integration is not used, and its editing/cursor/selection facilities do
not become M10 behavior.

Deterministic construction must support bundled-font-only operation. Production
profiles may additionally use system-font discovery. Font-source policy, identity,
and revision remain explicit and cache-visible.

Exact patch versions/features, dependency convergence, MSRV, and licenses are
revalidated by the implementation PR that first adds the dependencies.

### Measurement and paint share one logical text artifact

A text request contains RunenUI-owned text/style facts and text-specific logical
constraints. One immutable result supplies:

- final paragraph size and required line/baseline metrics;
- text/line/run/cluster ranges needed for inspection and later semantic/editing
  integration;
- exact shaped `ResourceRef`s and owner-local run origins;
- logical glyph identities/positions and exact immutable font/variation bindings;
- enough style association to keep foreground and other paint-only state outside
  shaped identity.

Metrics and resources come from the same shaping/line-breaking result. Runtime passes
that exact artifact from measurement into paint; widgets do not independently remint
or reshape text during paint.

Paint-only changes such as foreground color preserve shaped identity when glyph
geometry is unchanged. Width-only changes may re-linebreak without reshaping when
the text stack permits it; the resulting artifact/resource grouping may change when
line placement changes without implying a second shaping authority.

### Generic constraints remain runtime-owned

`LayoutConstraints` stays in runtime. `runenui_text` owns only a smaller neutral
text-specific constraint projection. Runtime lowers Taffy's known/available-space
facts into text requests.

There is no open-ended framework "measure until stable" loop. Taffy's bounded
algorithm drives explicit leaf measurements and runtime commits one aligned final
layout/publication candidate.

### SDF/MSDF is the primary production outline-text realization

For supported outline glyphs, `runenui_render_wgpu` owns:

- SDF/MSDF-family field generation from exact already-shaped outlines;
- atlas/page allocation, packing, cache/eviction, GPU textures, and device lifetime;
- field resolution/range/quality and scale/zoom realization tiers;
- shader reconstruction, antialiasing, foreground application, and renderer-owned
  text effects.

The logical shaped resource is not an alpha bitmap and is not tied to one
`RasterScale`. One logical `ResourceRef` may have multiple disposable renderer
realizations across devices, atlas pages, quality tiers, and raster scales without
changing shaping, line breaking, logical metrics, or identity.

The renderer may select SDF, MSDF, or MTSDF-style field representation when it
preserves the accepted output/identity contract. Small-size quality is proved with
real pixel/golden evidence. Supported outline glyphs must not silently fall back to
a separate alpha-raster production path.

M8A0 does **not** freeze an SDF/MSDF generator dependency. M8B performs a bounded
adopt-versus-build evaluation on one shared corpus/benchmark, including custom,
maintained pure-Rust, established reference-algorithm/FFI, and GPU approaches where
applicable. The selected implementation stays behind a narrow renderer-owned seam.

### Resource identity remains opaque across internal and external resource domains

`ResourceRef` remains the complete opaque logical identity. Text and renderer must
not derive lookup identity from debug text, kind, font name, mounted identity, or
backend handles.

`ResourceKind::ShapedTextRun` may remain if it continues to mean one immutable
logical shaped glyph resource. For runtime-shaped text, the retained paint scene
owns the exact `ResourceRef` to immutable shaped-resource binding through explicit
lifetime leases. The renderer resolves that binding from the publication and derives
only disposable scale/quality/atlas/device realizations; it does not ask the caller's
external `ResourceProvider` to recreate, rasterize, or rebind shaped text. This
retained binding is publication lifetime state rather than a second paint identity.

Caller-owned resource providers remain appropriate for external resource domains,
currently images. The complete opaque ref remains their lookup key. `ResourceKind`
may validate a requested domain but never selects a provider or internal resolver.

The text resource owner preserves each immutable binding while any live
`ResourceRef` may be retained by measurement/cache/publication, including renderer
retry after publication acknowledgement. If lifetime-safe pruning needs an opaque
weak companion to `ResourceRef`, it may expose only liveness—not alternate keys,
serialization identity, or backend lookup authority.

### Non-outline/color glyph formats are explicit

The primary M8 path covers supported outline glyphs. COLR/SVG/bitmap/intrinsic-color
glyphs must not silently flatten into foreground SDF/MSDF or alpha semantics. Until
a separately accepted resource/paint representation handles them truthfully, they
remain explicit unsupported breadth with structured diagnostics.

### Caching and diagnostics expose dependencies without new authority

Runtime cache compatibility accounts for every fact that can change style/text/
layout results: topology, authored contributions, resolved metric-affecting style,
available dimensions, text/span facts, font-source/text-system revision, and relevant
preference/theme revision.

Text diagnostics expose cache/re-linebreak/reshape/fallback/resource decisions.
Renderer diagnostics expose field generation, atlas reuse/eviction, quality tier,
upload, and draw realization. Neither becomes a second runtime trace authority.

### Deterministic headless and real-wgpu proof remain required

Deterministic tests use controlled bundled fonts, explicit locale/language/
preference inputs, fixed logical constraints, and ordinary public runtime/text
contracts. Expected geometry/text facts do not come from a private expected runtime
or alternate layout engine.

Real renderer closure uses the accepted M7 wgpu offscreen/readback path with bundled
outline fonts and SDF/MSDF-specific pixel/golden evidence.

### Clean cutover and serial delivery

M8 replaces the proof scalar-count text measurement, linear-only layout authority,
and M7 scale-specific alpha shaped-run renderer payload rather than preserving them
as parallel production paths.

1. **M8A — style environment/resolution:** production style foundation.
2. **M8B — logical text + SDF/MSDF realization:** production logical-text and
   renderer-realization foundation.
3. **M8C — runtime layout:** production runtime layout and exact text-feedback
   foundation.
4. **M8D — integrated closure:** integrated style/layout/text/semantics/renderer
   closure and milestone exit evidence.

Exact permanent observations, proof ownership, and delivery-slice assignment live in
[`docs/conformance/m8-conformance-matrix.md`](../conformance/m8-conformance-matrix.md).
This ADR defines the architecture and serial dependency only; it does not
independently reassign conformance rows.

No successor implementation issue is activated before its accepted predecessor is
owner-accepted, squash-merged, reconciled, and accepted-main validated.

## Consequences

Benefits:

- standardized hard algorithms are adopted without transferring RunenUI authority;
- measured and rendered text cannot silently diverge;
- outline text remains consistent with RunenUI's SDF-oriented renderer direction;
- text dependency weight has one justified crate boundary;
- field-generator/atlas technology can evolve behind stable neutral contracts;
- paint-only changes can avoid unnecessary text/layout work.

Costs/risks:

- clean pre-1.0 style/measurement/resource API changes are required;
- text resource lifetime and font-source revisioning must be explicit;
- Taffy integration must not retain a second tree;
- small-size SDF/MSDF quality, atlas churn, generation cost, and device caches need
  explicit benchmarks/goldens;
- generator choice remains an M8B implementation gate;
- color-font rendering remains explicit later breadth.

## Rejected alternatives

- **Extend the handwritten general layout engine:** high correctness/conformance
  cost with little RunenUI-specific value.
- **Use `TaffyTree`:** duplicates retained topology/identity/cache authority.
- **Move `LayoutConstraints` to core:** text only needs a smaller neutral projection.
- **Use Cosmic Text as the complete text subsystem:** overlaps renderer and future
  M10 editing authority more than the chosen composable shaping/layout stack.
- **Shape/line-break in wgpu:** creates a second authority after layout already used
  those metrics.
- **Keep alpha-raster text as production:** binds logical text to scale-specific
  renderer realization and defeats the SDF/MSDF target.
- **Freeze one field generator in A0:** generator maturity/build/CPU-GPU trade-offs
  require implementation evidence, not architectural guessing.
- **Let widgets measure/shape independently:** allows metric/pixel divergence,
  duplicate caches, inconsistent fallback, and lifetime defects.

## Acceptance proof

M8A0 acceptance requires the accompanying M8 matrix to have unique valid rows, the
accepted baseline and relevant unchanged authority to be cold-reviewed, current
Taffy/Parley/SDF-MSDF candidates to be checked for release/MSRV/license/support and
build implications, no production dependency or Rust implementation in A0,
canonical `cargo validate`, exact-head hosted CI, no unresolved review debt, explicit
owner acceptance, squash merge, and accepted-main validation before M8A.