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
> squash-merged, and accepted-main validated. Acceptance freezes the decisions
> below; it does not claim any M8 production implementation or dependency adoption.

## Context

M7 closed the reference production spine at proof maturity. Accepted `main` now has
one runtime-owned staged publication authority, renderer-neutral paint/hit products,
opaque `ResourceRef` identity, a real wgpu renderer/resource edge, native winit and
AccessKit adapters, and a winit-free external-host proof. M8 must replace the
remaining proof-level style, measurement/layout, and text assumptions without
moving authority into those edges.

The accepted M8 baseline has three material limitations that cannot be corrected as
independent rewrites:

- style resolution is typed and inspectable but covers only a small proof property
  set and has no production theme/recipe/state/preference model;
- layout is runtime-owned and deterministic but is a small linear proof engine with
  proof-only text metrics;
- text measurement returns only size/baselines, while paint receives only final size
  and computed style. There is no accepted production artifact binding the text
  metrics used by layout to the shaped text resource later rendered.

That last gap is architectural. Production text must not be measured by one system
and independently shaped again for paint. The exact text artifact whose metrics
participate in layout must also own the shaped resource identities consumed by paint
and retained renderer retry.

## Inherited authority this ADR does not supersede

This ADR composes rather than redefines accepted contracts:

- M3/ADR 0002 own mounted-runtime layout authority, mounted invalidation, and the
  rule that a dependency may provide algorithms without becoming a second UI tree;
- M4 owns canonical routed interaction and scheduling;
- M5 owns semantic identity/publication/action and deterministic public testing;
- M6 owns immutable paint/hit publications, exact scene composition, revision/damage,
  and opaque `ResourceRef` identity;
- M7 owns renderer/resource-provider, raster-scale, host, accessibility, retained
  publication retry, and external-host boundaries;
- M9 owns broad animation/composition;
- M10 owns complete editable-text behavior;
- M11 owns the standard control library;
- M13 owns broad platform/multi-window/recovery profiles.

M8 may make clean pre-1.0 cutovers required by these decisions, but it must not
create a parallel legacy style/layout/text authority.

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

Invalidation flows back through the same graph. Runtime remains the live owner of
when the stages execute, which mounted nodes are dirty, which derived caches remain
compatible, and which aligned products commit in one surface publication.

No style engine, layout library, text library, renderer, or host may own an
independent mounted tree, publication lifecycle, dirty graph, semantic tree, or UI
scheduler.

### Keep the style model RunenUI-owned

`runenui_core` continues to own host-neutral typed authored and resolved style
vocabulary. M8 broadens it to production property families rather than adopting a
CSS parser/cascade as framework authority.

The model includes:

- typed literals and token references;
- a theme environment containing token values and preference-sensitive defaults;
- typed recipes with explicit variants;
- runtime-owned interaction-state layers derived from canonical mounted interaction
  state rather than duplicated application state;
- authored per-node overrides;
- a bounded explicit set of inheritable properties, principally foreground and
  typography, rather than implicit inheritance of geometry properties;
- user high-contrast and reduced-motion policy inputs;
- exact per-property provenance and structured diagnostics.

Resolution precedence is deterministic and inspectable. From lower to higher
priority it is:

1. framework property defaults;
2. theme recipe base;
3. selected recipe variants in stable authored order;
4. applicable interaction-state recipe layers in one documented framework order;
5. authored token/literal overrides;
6. mandatory user-preference policy overrides for properties governed by that
   preference.

Theme selection may change token values, but token identity never becomes runtime
or renderer authority. Missing/invalid references diagnose explicitly.

Properties are classified by downstream effect. At minimum the runtime distinguishes
style-only, text-metric, layout, paint, hit, semantics, and preference-sensitive
changes so a foreground-color change does not force reshaping while a font-size or
line-height change does.

### Adopt Taffy algorithms inside runtime, not `TaffyTree`

M8 adopts the current compatible Taffy 0.14.x family for CSS Block, Flexbox, and Grid
algorithms. Implementation must use Taffy's low-level/custom-tree interfaces over
runtime-owned mounted topology and runtime-owned resolved style. It must not install
`TaffyTree` as a second retained UI tree.

Taffy scratch/layout/cache state is derived and disposable. Runtime remains the
owner of mounted identity, child order, dirty propagation, measurement dispatch,
final `LayoutRect` geometry, surface cache compatibility, and publication commit.
A substantial independent `runenui_layout` crate is therefore not justified by M8A0;
ADR 0002 remains current.

Public RunenUI style/layout APIs do not expose Taffy types. Runtime lowers the
RunenUI-owned property vocabulary into Taffy algorithm inputs and converts algorithm
results back into RunenUI-owned logical geometry.

Exact dependency features and patch versions are implementation evidence, but the
implementation should avoid the ready-made retained tree and enable only the layout
algorithms/features actually required by the accepted M8 contract.

### Introduce one renderer-neutral `runenui_text` boundary

Unlike layout, production text now has demonstrated independent package pressure.
M8 therefore adopts a new `runenui_text` crate with this ownership:

- depends on `runenui_core` plus the adopted text/font/raster stack;
- owns font collection/discovery/fallback configuration and immutable font data;
- owns shaping, bidi/text analysis, line breaking, paragraph layout, line/run
  metrics, and deterministic text fixtures;
- owns immutable shaped-text resource bindings and scale-specific rasterization of
  already-shaped glyph data;
- exposes RunenUI-owned renderer-neutral request/result/resource contracts;
- owns no mounted identity, runtime queue, surface publication, renderer backend,
  host, semantic identity/action authority, application state, or editing model.

`runenui_runtime` may depend on `runenui_text`; `runenui_text` must not depend on
runtime or the wgpu renderer. A renderer-edge adapter may depend on `runenui_text`
only to turn an already-shaped exact text resource into the existing renderer-edge
resource payload. It must not move font selection, shaping, line breaking, or text
layout into `runenui_render_wgpu`.

The package is justified by all of the repository extraction criteria that matter
here: a substantial independent dependency stack, independently owned computation
and resources, reuse by runtime measurement and renderer-edge resource realization,
and a separate deterministic proof surface.

### Adopt Parley for production text layout

M8 adopts Parley 0.11.x as the production text-layout family, including its Fontique,
HarfRust, Skrifa, and ICU4X stack for font discovery/fallback, shaping, font data,
Unicode analysis, bidi, segmentation, and line breaking. The production profile
must enable the international/complex-script behavior required by M8 rather than a
character-break fallback.

RunenUI does not expose Parley types as its public style/text protocol. Parley's
optional AccessKit integration is not used because M5/M7 already own semantic and
native accessibility authority. Parley's editing/cursor/selection facilities are
not adopted as M10 behavior merely because they are present in the dependency.

The caller must be able to construct deterministic bundled-font text systems that
do not depend on host font enumeration. Production profiles may additionally enable
system-font discovery. Exact font-source policy remains explicit and inspectable.

For rasterization of already-positioned glyphs, M8 adopts the compatible Swash 0.2.x
family or an equivalently reviewed raster path over the exact Parley-selected font
and glyph data. Rasterization is text-resource realization, not a second shaping
engine. The implementation PR must revalidate exact patch versions, Rust 1.93
compatibility, features, license inventory, and dependency convergence before
adding any dependency.

### Use one immutable text-layout artifact for measurement and paint

A production text request contains RunenUI-owned text/style facts and text-specific
logical constraints. `runenui_text` returns one immutable text-layout artifact whose
observable contract includes:

- final logical paragraph size;
- first/last and per-line baseline/line metrics required by layout and inspection;
- text/line/run ranges required for deterministic inspection and later semantic or
  editing integration without granting editing authority;
- one or more exact shaped-text `ResourceRef` values with owner-local run origins;
- enough run/style association to keep foreground and other scene paint state
  outside shaped-resource identity where the inherited M6/M7 contract requires it.

The resource refs and the metrics come from the same shape/line-break operation.
Runtime must retain and pass the exact artifact/resource facts from measurement into
owner-local paint contribution; widgets must not independently remint or reshape the
same text during paint.

A paragraph may yield multiple shaped paint runs, for example across font fallback
or metric-affecting styled spans. Paint-only changes such as foreground color do not
change the shaped resource identity when glyph geometry is unchanged. Font family,
size, weight, variation, language, feature, wrap, or constraint changes that alter
glyph geometry or line breaking produce a new compatible artifact/resource binding.

M8 does not require color-font/emoji paint to become ordinary foreground-alpha
semantics. If a chosen font supplies intrinsic color glyph paint that cannot be
represented truthfully by the accepted shaped-alpha resource contract, it must be
reported as unsupported production breadth or introduced through an explicit later
resource-contract revision; it must not silently discard or reinterpret intrinsic
color.

### Keep general layout constraints in runtime

The existing `LayoutConstraints` vocabulary remains runtime-owned. M8 does not move
it to core merely to satisfy a text dependency direction.

`runenui_text` defines only the neutral text-specific constraint facts it needs,
such as known/available inline extent and any bounded block policy. Runtime lowers
Taffy's known-dimension/available-space inputs into those text constraints. This
keeps `runenui_text` independent of runtime while avoiding a duplicate general
layout authority.

For unchanged text/style/font-provider state, width changes may re-run line breaking
and alignment without reshaping when the adopted text stack permits it. Runtime may
memoize exact text requests within/across publication attempts, but the same request
under the same text-system revision must be deterministic.

There is no open-ended framework "measure until stable" loop. Taffy's bounded
algorithm drives leaf measurement with explicit available-space facts; runtime
translates those calls to deterministic text requests and commits only one final
aligned layout/publication candidate.

### Preserve external resource ownership and retained-publication safety

`ResourceRef` remains the complete opaque logical identity. Neither text nor renderer
may derive provider identity from debug text, kind, font name, mounted identity, or
backend handles.

The text resource owner must preserve an immutable shaped-content binding for every
live shaped-text `ResourceRef`, including refs held only by a retained publication
being retried after renderer failure. Resource eviction must therefore be lifetime
safe. M8 may add an opaque weak-lifetime companion to `ResourceRef` solely so an
external resource owner can detect when no strong logical resource reference remains;
such a value must expose no payload, split key, serialization identity, or lookup
authority.

Renderer-edge resource resolution remains caller-owned. Any adapter that composes
application images with `runenui_text` must dispatch by exact resource ownership /
membership, not by `ResourceKind` as a provider selector. A shaped-text request at a
new `RasterScale` may re-rasterize the same immutable shaped resource without
changing its logical identity.

### Make measurement and invalidation dependencies inspectable

Runtime cache compatibility extends the accepted constraints/style-token/provider
model rather than bypassing it. Production text/layout caches are keyed or revised by
all facts that can change their result, including:

- mounted topology and authored measurement/layout/style contribution;
- resolved metric-affecting style and inherited style inputs;
- exact available/known dimensions;
- text content and styled-span metric facts;
- text-system/font-source identity and revision;
- preference/theme revisions that affect metric or layout properties.

Paint-only changes remain paint-only when safe. Text/resource diagnostics expose
cache hit/miss/relinebreak/reshape/rasterization decisions without becoming a second
runtime trace authority.

### Preserve deterministic headless proof

M8 production contracts must remain testable without a native window or system-font
nondeterminism. Deterministic tests use controlled bundled fonts, explicit locale /
language / preference inputs, fixed logical constraints, and ordinary public runtime
and text-system contracts.

Expected geometry/text facts are derived from accepted public products and frozen
fixtures, not a private expected runtime, alternate layout engine, or software
renderer pretending to be wgpu. Real-renderer closure continues through the accepted
M7 offscreen/readback path.

### Perform clean proof-to-production cutovers

M8 replaces, rather than indefinitely layers over, the proof-only scalar-count text
measurement and linear-only layout authority when the successor implementations are
accepted. Compatibility shims are not retained merely to avoid updating pre-1.0
callers.

The final implementation sequence is serial:

1. **M8A — style environment and production typed style resolution:** broaden core
   style vocabulary, themes/recipes/variants/state/preferences/provenance and
   invalidation classification without changing renderer ownership.
2. **M8B — production text system:** introduce `runenui_text`, coherent text-layout
   artifacts/resource lifetime/raster source, deterministic bundled-font proof, and
   cleanly replace proof text measurement.
3. **M8C — production runtime layout:** integrate Taffy low-level algorithms over
   mounted topology, production sizing/block/flex/grid/positioning/intrinsic
   measurement, and the exact text available-space feedback path.
4. **M8D — overflow/incremental/integrated closure:** finish clipping/scroll extents,
   incremental cache/invalidation proof, responsive/text-heavy corpora, semantic
   alignment, real wgpu evidence, authority cleanup, and final M8 reconciliation.

No successor implementation issue should be activated from the M8A0 branch. A0 must
first be owner-accepted, squash-merged, and accepted-main validated.

## Consequences

Positive consequences:

- RunenUI gains mature layout and Unicode text algorithms without transferring its
  runtime tree or publication authority to third-party libraries;
- text metrics and rendered glyph resources cannot silently diverge;
- renderer and host edges remain thin and reusable;
- the expensive text dependency stack is isolated behind a real package boundary;
- deterministic headless and real-wgpu proof remain compatible;
- color/state-only changes can avoid unnecessary reshaping.

Costs and risks:

- M8 requires substantial clean-cut public API changes around style and measurement;
- `runenui_text` must solve exact resource lifetime and font-source revisioning,
  not just wrap Parley;
- Taffy integration must translate between RunenUI properties and algorithm inputs
  without accidentally retaining a second tree;
- font availability is inherently profile-sensitive and must remain explicit in
  diagnostics/tests;
- intrinsic color-font rendering remains outside the inherited alpha-shaped-run
  contract unless separately revised.

## Rejected alternatives

### Keep extending the handwritten layout engine

Rejected. M8 requires production block/flex/grid behavior and measurement feedback;
maintaining those algorithms independently adds large correctness and conformance
cost without creating RunenUI-specific value.

### Use `TaffyTree` as the framework layout tree

Rejected. It would duplicate mounted topology/identity/cache ownership and weaken
ADR 0002. Only low-level/custom-tree algorithm integration is accepted.

### Move `LayoutConstraints` to core for text

Rejected. The generic constraint vocabulary is runtime layout policy. Text requires
only a smaller renderer-neutral constraint projection.

### Use Cosmic Text as the complete M8 text subsystem

Rejected for this milestone. It is capable, but its bundled shaping/rendering/editing
surface overlaps accepted renderer ownership and future M10 editing authority more
than Parley's composable text-layout stack.

### Shape in the wgpu renderer

Rejected. The renderer cannot be both glyph-layout authority and a consumer of the
runtime layout that depended on those metrics. It would also violate the accepted M7
resource boundary.

### Let each text widget measure and shape independently

Rejected. That permits metric/pixel divergence, duplicated caches, inconsistent
font fallback, and resource lifetime bugs.

## Acceptance proof

M8A0 acceptance requires:

- the accompanying M8 conformance matrix has unique IDs, valid statuses, and no
  duplicate inherited M3–M7 obligations;
- the exact accepted baseline/current source is reviewed against this target;
- current upstream Taffy/Parley/text-raster candidates are rechecked for release,
  MSRV, license, support and dependency implications;
- no Cargo dependency or production Rust implementation lands in A0;
- canonical `cargo validate` and exact-head hosted CI pass;
- the complete diff and relevant unchanged authority are cold-reviewed;
- owner acceptance, squash merge, and accepted-main validation occur before M8A.
