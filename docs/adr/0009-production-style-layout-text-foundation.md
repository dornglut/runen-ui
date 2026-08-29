# ADR 0009: Production style, layout, and text foundation

> **Category:** ADR
>
> **Status:** Proposed target architecture; acceptance requires exact-head owner acceptance
>
> **Decision date:** 2026-08-29
>
> **Milestone:** M8
>
> **Reviewed baseline:** `1a5af89c1886654d859f56d1d8afe3e46abdcf95`
>
> **Acceptance:** this ADR becomes accepted target architecture only when the exact
> M8A0 package containing it is explicitly accepted by the repository owner,
> squash-merged, and accepted-main validated. Acceptance freezes the decisions below;
> it does not claim any M8 production implementation or dependency adoption.

## Context

M7 closed the reference production spine at proof maturity. Accepted `main` has one
runtime-owned staged publication authority, renderer-neutral paint/hit products,
opaque `ResourceRef` identity, a real wgpu renderer/resource edge, native winit and
AccessKit adapters, and a winit-free external-host proof.

M8 must replace three coupled proof-level limitations:

- style resolution is typed and inspectable but lacks production themes, recipes,
  state layers, preference policy, and property breadth;
- layout is runtime-owned and deterministic but remains a small linear proof engine;
- text measurement is proof-level and is not bound to one production shaped artifact
  that later supplies the exact glyph resources rendered by the real renderer.

The text gap is architectural. Production text must not be measured by one system and
independently reshaped during paint. The same logical shaping/line-breaking result
must supply layout metrics and the exact shaped resources later realized by the
renderer.

M7's `ShapedRunRaster` scale-specific alpha coverage is accepted proof behavior, not
the M8 production target. For ordinary supported outline glyphs, M8 targets an
SDF/MSDF-family wgpu realization rather than preserving alpha-raster text as a
parallel production path.

## Inherited authority

This ADR composes rather than redefines accepted contracts:

- M3 and ADR 0002 own mounted-runtime layout authority, mounted invalidation, and
  the rule that an adopted layout algorithm cannot become a second UI tree;
- M4 owns canonical interaction and scheduling;
- M5 owns semantic identity/publication/action and deterministic public testing;
- M6 owns immutable renderer-neutral paint/hit publication, revision/damage, and
  opaque `ResourceRef` identity;
- M7 owns renderer/resource-provider, raster-scale, host, accessibility, retained
  publication retry, and external-host boundaries;
- M9 owns broad visual composition/animation;
- M10 owns complete editable-text behavior;
- M11 owns the standard control library;
- M13 owns broad platform and multi-window profiles.

M8 may make clean pre-1.0 cutovers required by these decisions. It must not retain a
parallel proof-era style/layout/text authority for compatibility convenience.

## Decision

### Treat style, layout, and text as one staged dependency graph

The production dependency loop is:

```text
style environment + authored state
    -> resolved typed style
    -> layout available-space facts
    -> text shaping / line breaking / measurement
    -> final layout geometry
    -> paint / hit / semantics
```

Invalidation flows back through the same graph. `runenui_runtime` remains the live
owner of when stages execute, which mounted nodes are dirty, which derived caches are
compatible, and which aligned products commit in one surface publication.

No style engine, layout library, text library, renderer, or host may own an
independent mounted tree, publication lifecycle, dirty graph, semantic tree, or UI
scheduler.

### Keep the production style model RunenUI-owned

`runenui_core` continues to own host-neutral typed authored and resolved style
vocabulary. M8 broadens this vocabulary rather than adopting a CSS parser/cascade as
framework authority.

The model includes:

- typed literals and token references;
- a theme environment containing token values and preference-sensitive defaults;
- typed recipes with explicit variants;
- runtime-owned interaction-state layers derived from canonical mounted interaction
  state rather than duplicated application state;
- authored per-node overrides;
- an explicit bounded set of inheritable properties, principally foreground and
  typography, rather than implicit inheritance of geometry properties;
- high-contrast and reduced-motion preference inputs;
- exact per-property provenance and structured diagnostics.

Resolution precedence is deterministic and inspectable. From lower to higher
priority:

1. framework property defaults;
2. theme recipe base;
3. selected recipe variants in stable authored order;
4. applicable interaction-state recipe layers in one documented framework order;
5. authored token/literal overrides;
6. mandatory user-preference policy overrides for governed properties.

Missing or invalid references diagnose explicitly. Theme selection can change token
values, but token identity never becomes runtime or renderer authority.

Resolved properties are classified by downstream effect. At minimum runtime can
distinguish style-only, text-metric, layout, paint, hit, semantics, and
preference-sensitive changes so, for example, foreground color does not force
reshaping while font size or line height does.

### Adopt Taffy algorithms inside runtime, not `TaffyTree`

M8 adopts the compatible Taffy `0.14.x` family for CSS Block, Flexbox, and Grid
algorithms. Integration uses Taffy's low-level/custom-tree interfaces over exact
runtime-owned mounted topology and resolved RunenUI style. It must not install
`TaffyTree` as a second retained UI tree.

Taffy scratch/layout/cache state is derived and disposable. Runtime remains owner of
mounted identity, child order, dirty propagation, measurement dispatch, final
RunenUI logical geometry, surface-cache compatibility, and publication commit.

Public RunenUI APIs expose no Taffy types. Runtime lowers RunenUI-owned style/layout
facts into Taffy algorithm inputs and converts results back into RunenUI-owned
logical geometry. The implementation should disable the ready-made retained-tree
feature and enable only accepted algorithm features.

A separate `runenui_layout` crate is not justified by M8A0: there is no independent
ownership or consumer boundary beyond runtime-owned layout authority.

### Introduce a renderer-neutral `runenui_text` crate

Production text has real independent package pressure. M8 therefore introduces
`runenui_text` with this ownership:

- depends on `runenui_core` and the reviewed text/font stack;
- owns font collection/discovery/fallback configuration and immutable font data;
- owns shaping, Unicode/bidi analysis, line breaking, paragraph layout, line/run
  metrics, and deterministic text fixtures;
- owns immutable logical shaped-text resource bindings;
- exposes RunenUI-owned renderer-neutral request/result/resource contracts;
- exposes enough immutable font/glyph/outline data for an authorized renderer-edge
  resolver to realize the exact already-shaped resource;
- owns no mounted identity, runtime queue, publication lifecycle, SDF/MSDF atlas,
  GPU texture, renderer quality policy, host, semantic identity/action authority,
  application state, or editing model.

`runenui_runtime` may depend on `runenui_text`; `runenui_text` must not depend on
runtime or `runenui_render_wgpu`.

This crate is justified by independent dependency weight, computation/resource
ownership, reuse by runtime measurement and renderer-edge realization, and a
separate deterministic proof surface.

### Adopt Parley for shaping and paragraph text layout

M8 adopts Parley `0.11.x`, including its Fontique, HarfRust, Skrifa, and ICU stack,
for font discovery/fallback, shaping, font data, Unicode analysis, bidi,
segmentation, and line breaking.

RunenUI exposes no Parley types as its public style/text protocol. Parley's optional
AccessKit integration is not used because M5/M7 already own semantic/accessibility
authority. Upstream editing/cursor/selection facilities do not become M10 behavior.

Deterministic callers must be able to construct bundled-font-only text systems that
do not depend on host font enumeration. Production profiles may additionally enable
system-font discovery. Font-source policy, identity, and revision remain explicit
and inspectable.

The production configuration must enable the international/complex-script behavior
required by M8 rather than falling back to scalar-count or character-break
approximations.

Exact patch versions, feature sets, dependency convergence, MSRV compatibility, and
license inventory are revalidated by the implementation PR that first adds the
libraries.

### Use one immutable logical text artifact for measurement and paint

A production text request contains RunenUI-owned text/style facts and text-specific
logical constraints. `runenui_text` returns one immutable text-layout artifact whose
observable contract includes:

- final logical paragraph size;
- first/last and per-line baseline/line metrics required by layout and inspection;
- text/line/run/cluster ranges needed for deterministic inspection and later
  semantic/editing integration without granting editing authority;
- one or more exact shaped-text `ResourceRef` values with owner-local run origins;
- logical glyph identities/positions and exact immutable font/variation binding
  behind those refs;
- enough run/style association to keep foreground and other paint-only state outside
  shaped-resource identity where inherited M6/M7 contracts require it.

Metrics and resource refs come from the same shaping/line-breaking result. Runtime
retains and passes the exact artifact/resource facts from measurement into paint;
widgets must not independently remint or reshape the same text during paint.

A paragraph may yield multiple shaped paint runs across font fallback or
metric-affecting spans. Paint-only changes such as foreground color do not change the
logical shaped resource when glyph geometry is unchanged. Font family, size, weight,
variation, language, OpenType feature, text, or constraint changes that alter glyph
geometry or line breaking produce a new compatible artifact/resource binding.

### Keep general layout constraints in runtime

The existing `LayoutConstraints` vocabulary remains runtime-owned. M8 does not move
it to core merely to satisfy text dependency direction.

`runenui_text` defines only the renderer-neutral text-specific constraint projection
it needs, such as known/available inline extent and bounded block policy. Runtime
lowers Taffy's known-dimension/available-space facts into those requests.

For unchanged text/style/font state, width changes may re-run line breaking and
alignment without reshaping when the adopted stack permits it. There is no
open-ended framework "measure until stable" loop: Taffy's bounded algorithm drives
leaf measurement with explicit available-space facts, and runtime commits one final
aligned layout/publication candidate.

### Make SDF/MSDF the primary production wgpu text realization

For supported outline fonts/glyphs, `runenui_render_wgpu` owns renderer realization:

- SDF/MSDF-family field generation from exact already-shaped outline glyphs;
- atlas/page allocation, packing, cache/eviction, GPU textures, and device lifetime;
- field resolution/range/quality policy and scale/zoom realization tiers;
- shader reconstruction, antialiasing, foreground application, and renderer-owned
  text effects.

The logical shaped resource is not an alpha bitmap and is not tied to one
`RasterScale`. One logical shaped `ResourceRef` may have multiple disposable
renderer realizations across devices, atlas pages, quality tiers, or raster scales
without changing shaping, line breaking, logical metrics, or resource identity.

The renderer may choose SDF, MSDF, or MTSDF-style field representations when they
preserve the accepted visual/identity contract. Small-size quality is proved by real
pixel/golden corpora; the renderer may adjust field resolution/range or field
variant, but supported outline glyphs must not silently use a separate alpha-raster
production path.

M8A0 deliberately does **not** freeze an SDF/MSDF generator dependency. M8B performs
a bounded adopt-versus-build evaluation using the same glyph corpus and renderer
benchmarks. At minimum it compares a custom implementation over the exact font
outlines, a maintained pure-Rust implementation if suitable, the established
msdfgen algorithm as a reference while accounting for FFI/build cost, and a GPU
approach if evidence justifies it. The selected generator remains behind a narrow
renderer-owned seam so it can change without changing public text or scene
contracts.

### Preserve the renderer-neutral resource boundary

`ResourceRef` remains the complete opaque logical identity. Neither text nor
renderer derives provider identity from debug text, resource kind, font name,
mounted identity, or backend handles.

`ResourceKind::ShapedTextRun` may remain the neutral scene kind if it continues to
mean an immutable logical shaped glyph resource. M7's renderer-edge
`ShapedRunRaster` scale-specific alpha payload is proof-era behavior and is cleanly
replaced for the production outline-glyph path; M8 must not retain alpha and
SDF/MSDF as parallel authoritative text realization contracts.

A caller-owned renderer resource provider still resolves the complete `ResourceRef`.
For production shaped text its payload exposes exact immutable already-shaped
glyph/font/outline facts, not scale-specific coverage. The precise API shape belongs
to M8B implementation review, but it must preserve caller-owned provider composition
and must not use `ResourceKind` as a provider selector.

The text resource owner preserves an immutable shaped-content binding for every live
shaped-text `ResourceRef`, including refs held only by a retained publication being
retried after renderer failure. Resource eviction is lifetime-safe. M8 may introduce
an opaque weak-lifetime companion to `ResourceRef` solely to detect that no strong
logical reference remains; it must expose no payload, split key, serialization
identity, or lookup authority.

### Treat color and non-outline glyph formats explicitly

The primary M8 production path is SDF/MSDF realization of supported outline glyphs.
COLR, SVG, bitmap, and intrinsic color-emoji behavior must not silently flatten into
foreground-colored distance-field or alpha semantics.

If these glyphs cannot be represented truthfully by the accepted M8 resource/paint
contract, they remain explicit unsupported breadth with structured diagnostics until
a separately accepted contract revision introduces an honest representation.

### Make caching, invalidation, and realization inspectable

Runtime cache compatibility includes every fact capable of changing production
style/text/layout results, including:

- mounted topology and authored measurement/layout/style contribution;
- resolved metric-affecting/inherited style inputs;
- exact known/available dimensions;
- text content and metric-affecting span facts;
- text-system/font-source identity and revision;
- preference/theme revisions affecting metric or layout properties.

Paint-only changes remain paint-only when safe. Text diagnostics expose cache
hit/miss, re-linebreak, reshape, fallback, and shaped-resource decisions. Renderer
diagnostics separately expose SDF/MSDF generation, atlas hit/miss/eviction, quality
tier, upload, and draw realization without becoming a second runtime trace
authority.

### Preserve deterministic headless and real-renderer proof

M8 production contracts remain testable without a native window or system-font
nondeterminism. Deterministic tests use controlled bundled fonts, explicit
locale/language/preference inputs, fixed logical constraints, and ordinary public
runtime/text contracts.

Expected geometry/text facts come from accepted public products and frozen fixtures,
not a private expected runtime, alternate layout engine, or software renderer
pretending to be wgpu. Real text-renderer closure uses the accepted M7 wgpu
offscreen/readback path with deterministic bundled outline fonts and SDF/MSDF pixel
and golden evidence.

### Perform clean proof-to-production cutovers

M8 replaces rather than layers over the proof-only scalar-count text measurement,
linear-only layout authority, and M7 scale-specific alpha shaped-run renderer payload
when their successors are accepted.

The serial implementation sequence is:

1. **M8A — production style environment/resolution:** broaden typed style,
   themes/recipes/variants/state/preferences/provenance and invalidation
   classification.
2. **M8B — production logical text plus SDF/MSDF realization:** introduce
   `runenui_text`; adopt Parley; establish coherent text artifacts, resource
   lifetime, and logical outline-glyph payloads; complete the bounded generator
   evaluation; implement the accepted wgpu SDF/MSDF atlas/shader path; remove the
   proof alpha-shaped-run production authority.
3. **M8C — production runtime layout:** integrate Taffy low-level/custom-tree
   algorithms over mounted topology, production sizing/block/flex/grid/positioning,
   intrinsic measurement, and exact text available-space feedback.
4. **M8D — overflow/incremental/integrated closure:** finish clipping/scroll extents,
   incremental invalidation/cache proof, responsive/text-heavy corpora, semantic
   alignment, real wgpu proof, authority cleanup, and final M8 reconciliation.

No successor implementation issue is activated from the M8A0 branch. A0 must first
be owner-accepted, squash-merged, and accepted-main validated.

## Consequences

Benefits:

- mature standardized layout and Unicode text algorithms are adopted without
  transferring RunenUI's mounted/runtime authority;
- measurement and rendered glyph identity cannot silently diverge;
- ordinary outline text follows RunenUI's SDF-oriented renderer direction rather
  than becoming a permanent alpha-raster exception;
- text dependency weight is isolated behind a real package boundary;
- SDF/MSDF generator and atlas technology can evolve behind stable renderer-neutral
  text artifacts;
- deterministic headless and real-wgpu proof remain compatible;
- paint-only state changes can avoid unnecessary reshaping/layout.

Costs and risks:

- M8 requires clean-cut public API changes around style and measurement;
- `runenui_text` must solve exact resource lifetime and font-source revisioning, not
  merely wrap Parley;
- Taffy integration must translate RunenUI properties without retaining a second
  tree;
- SDF/MSDF quality at small sizes, atlas churn, field-generation cost, and device
  cache behavior require explicit benchmarks and golden proof;
- generator implementations are less settled than Taffy/Parley, so generator choice
  remains a bounded M8B implementation gate;
- intrinsic color-font rendering remains explicit later breadth.

## Rejected alternatives

### Extend the handwritten general layout engine

Rejected. Production block/flex/grid behavior and measurement feedback add large
correctness/conformance cost without creating RunenUI-specific value.

### Use `TaffyTree` as the framework tree

Rejected. It would duplicate mounted topology/identity/cache ownership. Only
low-level/custom-tree algorithm integration is accepted.

### Move `LayoutConstraints` to core for text

Rejected. Generic constraints remain runtime layout policy. Text needs only a smaller
renderer-neutral projection.

### Use Cosmic Text as the complete M8 text subsystem

Rejected for M8. Its combined shaping/rendering/editing surface overlaps accepted
renderer ownership and future M10 editing authority more than Parley's composable
text-layout stack.

### Shape or line-break in the wgpu renderer

Rejected. Renderer output depends on layout that already consumed these metrics;
renderer-owned shaping would create a second text-layout authority.

### Preserve alpha-raster glyph coverage as production text

Rejected. Scale-specific alpha coverage would make text a permanent raster
exception, bind logical resources to renderer realization, and undermine the target
SDF/MSDF architecture.

### Freeze a specific SDF/MSDF generator in A0

Rejected. The architecture boundary is clear, but generator choices differ in
maturity, FFI/build cost, quality, and CPU/GPU trade-offs. M8B must decide from a
bounded shared corpus/benchmark without changing public text contracts.

### Let each text widget measure and shape independently

Rejected. That permits metric/pixel divergence, duplicated caches, inconsistent font
fallback, and resource lifetime defects.

## Acceptance proof

M8A0 acceptance requires:

- the accompanying M8 conformance matrix has unique IDs, valid statuses, and no
  duplicate inherited M3-M7 obligations;
- exact accepted current source and relevant unchanged authority are reviewed against
  this target;
- current upstream Taffy, Parley, and SDF/MSDF candidates are reviewed for release
  family, MSRV, license, features, build/dependency cost, maintenance, and authority;
- no Cargo dependency or production Rust implementation lands in A0;
- canonical `cargo validate` and exact-head hosted CI pass;
- the complete diff is cold-reviewed with no unresolved review debt;
- owner acceptance, squash merge, and accepted-main validation occur before M8A.
