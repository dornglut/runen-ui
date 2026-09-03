# Public API Contract

This document describes the **conceptual ownership and invariants** of RunenUI's current public surface. Exact Rust signatures, trait bounds, visibility, and documentation are authoritative in source/Rustdoc.

## `runenui_core`

`runenui_core` owns host-neutral public values and protocols that must be usable without a live runtime or platform/backend dependency. Its responsibilities include:

- `UiApp` application state/action/update and host-neutral effect/subscription protocol values;
- validated authored identity, host-neutral style properties/tokens/themes/recipes/variants/preferences/resolution vocabulary, geometry, transient `View`/`Element` authoring, and typed built-in view vocabulary;
- state-aware open widget/lifecycle/event contracts and typed action mapping;
- runtime-local opaque protocol identity types such as mounted/semantic/surface/work identities, without allocation authority;
- host-neutral pointer/keyboard/text/composition/focus/semantic command and semantic contribution/action vocabulary;
- renderer- and host-neutral paint/hit contribution values, logical scene-composition geometry, opaque neutral resource identity/kind values, and image/shaped-run primitive placement values used by the accepted M6 scene protocol.

Core must not own persistent mounted/semantic storage, live queue/scheduler state, live interaction/focus/activation authority, runtime identity allocation, native window/accessibility objects, renderer backend handles, resource-provider/lookup/payload/cache authority, text shaping/line breaking, renderer realization, application product state, or testing-only mutation seams.

## `runenui_text`

`runenui_text` is the renderer-neutral production text boundary. It owns:

- explicit bundled-only versus system-and-bundled font-source policy and cache-visible font-source revision;
- RunenUI-owned text request, paragraph/style, language/direction, text-specific constraint, cache/reflow diagnostic, immutable artifact, and shaped-resource contracts;
- Parley-backed font selection/fallback, Unicode/script/bidi/grapheme analysis, shaping, line breaking, alignment, baseline/paragraph metrics, and reusable logical text-layout state;
- immutable logical text artifacts that supply measurement and exact line/run/cluster/glyph/font facts from one shaping/line-break result;
- immutable scale-independent `ResourceRef -> ShapedTextResource` bindings containing the exact already-shaped font/glyph facts required for later outline realization.

Parley/Fontique/HarfRust/Skrifa/ICU types are implementation details and do not become RunenUI API authority. `runenui_text` must not own mounted/runtime/publication identity or scheduling, general layout topology, native/semantic/application state, renderer/GPU/SDF-MSDF atlas state, or editable-text behavior.

## `runenui_runtime`

`runenui_runtime` owns live framework authority:

- runtime namespace and generational mounted/semantic storage;
- reconciliation, lifecycle execution, canonical focus/interaction state, staged activation, production style-resolution orchestration/cache compatibility/invalidation, and capability/invalidation caches;
- one generalized sequenced work queue, bounded pump, tasks/timers/subscriptions/host requests, clocks, wake/redraw, and shutdown;
- exact routed command/input processing and defaults;
- bounded canonical trace, deterministic export, and inert replay projections;
- generic measurement/layout execution, live `TextSystem` orchestration, topology-aligned reusable text-layout state, and staged surface publication;
- text measurement from immutable `runenui_text` artifacts and exact projection of those same shaped-resource facts into paint, including publication-owned shaped-resource leases needed for retained renderer retry;
- canonical renderer-neutral transformed/clipped/ordered paint-scene composition plus `RasterScale` and `PaintPublication` revision/base/damage/alignment authority;
- canonical transformed/clipped/ordered displayed `HitTestScene` composition, mounted-target/membership injection, retained displayed-generation lookup, and point/resolved-target authority;
- scene requirements derived from canonical paint content and neutral consumer capability checks without backend-specific rewriting;
- independent semantic publication/update/diagnostics and exact semantic-action admission/resolution.

Runtime must not depend on testing convenience, concrete native platforms, concrete renderer implementations, product state, external resource-provider/payload/cache ownership, font/shaping/line-breaking algorithm authority, SDF/MSDF realization, or a second interaction/style/semantic/paint/hit/testing authority.

## `runenui_render_wgpu`

`runenui_render_wgpu` is the accepted reusable concrete renderer edge over ordinary public paint publication. It owns:

- wgpu instance/adapter/device/queue and surface/offscreen target state;
- exact renderer-local successful-publication lineage and update/full-resync classification;
- caller-facing complete-`ResourceRef` provider requests for external image resources plus disposable image realization/cache state;
- consumption of exact retained `ShapedTextResource` bindings from `PaintPublication` and disposable renderer-private per-glyph SDF/MSDF generation, quality classes, atlas pages, GPU textures, cache lifetime, shader reconstruction, and antialiasing;
- native-surface presentation, offscreen readback, and immutable renderer observation records.

It consumes public core/runtime/text contracts only where required by paint realization. Caller-owned `ResourceProvider` remains the edge for external resources such as images; runtime-shaped text is resolved from the retained publication binding and is never recreated by that provider. The renderer must not own a native event loop, widget/semantic/mounted/layout authority, runtime mutation, application resource identity/bindings, shaping/line-breaking/font-discovery authority, style/theme resolution, or AccessKit/winit behavior.

## `runenui_winit`

`runenui_winit` is the reusable native adapter edge proven by the second real M7 winit consumer. It owns only rebuildable translation/projection state:

- host-session mapping from native winit device identity to neutral `InputDeviceId` values;
- loss-preserving native keyboard lifetime, key, repeat, location, modifier, and cancellation translation;
- native mouse pointer/button lifetime translation, including multi-button and point-authority cancellation semantics;
- AccessKit tree projection over ordinary semantic publication, adapter-owned stable native identity, and exact AccessKit-action to semantic-action translation.

It consumes public core/runtime contracts plus winit/AccessKit types. It has no renderer dependency and must not own or hide a native window/event loop, runtime pump or mutation policy, wake/redraw/publication acknowledgement, displayed-frame authority, renderer configuration/recovery, presentation lifecycle, application behavior, or hidden style/theme authority. Explicit platform preference facts may be supplied through ordinary host-neutral style inputs without making the adapter the policy owner.

## `runenui_testing`

`runenui_testing` is a downstream public convenience crate. `TestHarness<App>` composes ordinary public core/runtime APIs with deterministic logical time, bounded pumping/settling, deterministic surface publication, read-only observation of the latest ordinary public paint/hit publication products and exact input context, synthetic public interaction, and semantic queries/targets.

It owns no live runtime queue, mounted/semantic store, identity allocation, publication state, trace authority, resource provider, style/text authority, or private mutation bridge. A test target retains exact public surface/semantic scope; testing must not reconstruct private mounted routing identity, fabricate scene/publication lineage, duplicate hit resolution, or guess a surface from a bare semantic ID.

## Accepted native and external application integration

The accepted M7 native path remains host-owned rather than a generic framework runner. `examples/reference_winit` and the native Counter application each visibly own their winit window/event loop, runtime pumping and redraw driving, native coordinate mapping, renderer presentation/recovery policy, and displayed-frame authority. Both consume `runenui_winit` for the substantial native input/AccessKit mechanics that are independent of those application policies.

Native winit/AccessKit types remain outside `runenui_core` and `runenui_runtime`; `runenui_render_wgpu` remains winit-free; and neither native application becomes framework-owned loop authority. The second consumer justifies the bounded adapter crate, not a generic native application facade.

The accepted `tests/external_host` proof independently consumes ordinary public core/runtime contracts plus `runenui_render_wgpu` without importing winit, AccessKit, or testing convenience. Its caller-owned sequence visibly controls submit, pump, redraw consumption, publication, acknowledgement, render/retry, and presentation, including retained-publication retry after caller-owned external resource failure. This confirms the reusable external-host boundary is the existing public runtime/publication/renderer/resource contract rather than a framework-owned host facade.

## Core invariants

### Transient authoring, persistent runtime

`View`/`Element` values are owned transient descriptions derived from application state and consumed by reconciliation. Persistent identity, local widget state, lifecycle, focus/interaction state, work ownership, and publication authority remain in the mounted runtime.

### Distinct identities

Authored IDs/keys, mounted IDs, semantic IDs, work/trace sequences, and surface identities have separate meanings. A semantic ID is not a mounted-arena alias. Runtime-issued identities are runtime-local and must not be serialized or forged into live authority.

### One processing authority

Accepted application actions, routed commands/input, semantic actions after admission, effect/work transitions, timer/subscription events, and derived work converge through the runtime's canonical sequenced processing path. No public direct-dispatch or second event/action queue may silently bypass ordering/default/trace semantics.

### Independent semantics

Widgets contribute platform-neutral owner-local semantic descriptions. Runtime validates/reconciles them into independently allocated semantic lifetimes and a renderer-independent surface-scoped publication. Public semantic consumers receive semantic identity/content, not mounted routing authority.

Exact semantic action requests are admitted against current published semantic authority and then converge on the canonical runtime command/default path. No second accessibility callback engine exists.

### One style computation model

Authored `StyleIntent`, explicit `StyleEnvironment` inputs, canonical transient interaction facts, and bounded parent inheritance converge through the core production resolver. Runtime supplies live interaction/activation facts, retains compatibility inputs, and drives invalidation; it does not maintain a second style tree or policy engine. Renderers and platform adapters consume or supply explicit neutral facts only and cannot reinterpret the style cascade.

### One logical text computation model

RunenUI-owned text requests are resolved by `runenui_text`; runtime owns when that computation participates in mounted measurement/publication. One immutable logical artifact supplies both paragraph measurement and the exact shaped resource facts later painted. Paint does not independently reshape, line-break, discover fonts, or mint alternate shaped identity. Foreground remains paint-only when glyph geometry is unchanged.

### Staged publication

Surface publication follows a staged transaction with admission, read-only/staged planning, candidate-dependent final preflight, and commit. Recoverable refusal or terminal failure must not expose a partial new RunenUI-owned publication state.

Renderer-facing paint products, hit/input products, semantics, layout, and diagnostics remain distinct authorities even when committed together. Accepted M6 uses immutable `PaintScene`/`PaintPublication` and `HitTestScene` products; scene requirements derive from canonical paint content, while raster scale, base revision, and damage remain paint-publication metadata. Paint revision identity remains distinct from displayed input generation. Independent consumers can reconstruct complete deterministic scene snapshots from those public products without widget-kind, mounted/layout storage, private runtime, or hidden prior-scene authority.

For runtime-shaped text, retained paint publication lifetime also preserves the exact immutable logical shaped-resource bindings referenced by scene items. Renderer scale/quality/atlas/device state is disposable and can be reconstructed from those bindings without runtime republish, external provider lookup, reshaping, or `ResourceRef` reminting.

## Current limitations

The current public surface is pre-1.0 and may change incompatibly when accepted architecture requires a clean cutover. Important missing production capabilities include:

- broader production host/application ergonomics beyond the accepted proof-level native and external-host paths;
- production responsive Block/Flex/Grid/intrinsic layout and exact available-space/text feedback beyond the accepted M8B text seam (M8C/M8D);
- production text editing, selection, clipboard, and related behavior (M10);
- supported rendering for intrinsic COLR/SVG/bitmap glyph formats; current M8B behavior diagnoses that breadth explicitly;
- multi-window lifecycle and supported platform-profile breadth;
- broader style-property/composition breadth beyond the accepted M8A/M8B mechanism;
- a complete standard control library.

M7 is accepted complete at proof maturity through the real wgpu renderer/resource edge, standalone winit host/native-input/presentation path, reusable winit/AccessKit adapter, native Counter showcase, and winit-free downstream external-host proof over the same public contracts. M8A is accepted current behavior at partial styling maturity. M8B is accepted current behavior at partial text maturity through the renderer-neutral production text boundary, exact shared measurement/paint artifacts, retained shaped-resource lifetime, and renderer-owned SDF/MSDF realization. M8C production runtime layout/text feedback is the next durable M8 slice; M8D integrated closure remains later target work. Current maturity is summarized in [status](../status.md). Durable future sequencing belongs in the [roadmap](../roadmap.md). Permanent observable/proof requirements live under [conformance](../conformance/README.md).

Do not infer support from a target ADR, design document, type name, or roadmap entry alone. Code/tests establish current behavior; source/Rustdoc establishes the exact public Rust surface.
