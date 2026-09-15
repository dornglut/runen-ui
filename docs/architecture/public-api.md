# Public API Contract

This document describes the **conceptual ownership and invariants** of RunenUI's current public surface. Exact Rust signatures, trait bounds, visibility, and documentation are authoritative in source/Rustdoc.

## `runenui_core`

`runenui_core` owns host-neutral public values and protocols that must be usable without a live runtime or platform/backend dependency. Its responsibilities include:

- `UiApp` application state/action/update and host-neutral effect/subscription protocol values;
- validated authored identity, host-neutral style properties/tokens/themes/recipes/variants/preferences/resolution vocabulary, geometry, normalized layout container/sizing/positioning/overflow values, transient `View`/`Element` authoring, and typed built-in view vocabulary;
- state-aware open widget/lifecycle/event contracts, bounded renderer-neutral `WidgetMeasure` input/result vocabulary, geometry-neutral child-bearing participation, and typed action mapping;
- runtime-local opaque protocol identity types such as mounted/semantic/surface/work identities, without allocation authority;
- host-neutral pointer/keyboard/text/composition/focus/semantic command and semantic contribution/action vocabulary;
- renderer- and host-neutral paint/hit contribution values, logical scene-composition geometry, opaque neutral resource identity/kind values, and shaped-run placement values used by the accepted scene protocol;
- accepted M9 structural visual/composition vocabulary: validated rectangle/rounded-rectangle/ellipse/path shapes, generic fill/stroke and stroke-style facts, solid/linear/radial brushes, exact gradient-stop semantics, image descriptor/fit/crop/alignment/nine-slice values, item/group clips and opacity, snapshot-local paint-group authoring values, ordinary-shadow/style visual values, and presentation transforms;
- accepted M9 authored deterministic-motion vocabulary and pure sampling rules: owner-local `AnimationId`, closed `MotionTarget`/typed `MotionValue`, transition policy/specification, keyframes/timelines, checked duration/delay/repeat, deterministic easing, explicit reduced-motion strategy, and the exact continuous/discrete interpolation compatibility defined by the accepted M9 motion contract.

Core must not own persistent mounted/semantic storage, live layout topology/algorithm/cache state, live queue/scheduler state, live motion/declaration lifecycle or sampling time, live interaction/focus/activation authority, runtime identity allocation, native window/accessibility objects, renderer backend handles, resource-provider/lookup/payload/cache authority, text shaping/line breaking, renderer tessellation/raster/mask realization, application product state, or testing-only mutation seams.

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
- production measurement/layout execution through private low-level Taffy Block/Flex/Grid algorithms over exact mounted topology, transaction-local disposable Taffy caches, runtime-owned root constraints, bounded custom measurement dispatch, overflow/extents, and one final logical geometry authority;
- live `TextSystem` orchestration and topology-aligned reusable text-layout state, including lowering each Taffy leaf request into renderer-neutral text constraints and retaining the exact text state associated with the final `PerformLayout` request;
- text measurement from immutable `runenui_text` artifacts and exact projection of those same shaped-resource facts into paint, including publication-owned shaped-resource leases needed for retained renderer retry;
- accepted M9 live deterministic-motion authority: exact mounted-generation transition/declaration reconciliation, one candidate `MonotonicClock` instant per staged publication attempt, current-sample replacement/precedence, reduced-motion and mandatory-preference decisions, deterministic effective-value sampling, differential invalidation/cache/group decisions, active redraw/wake demand, motion diagnostics/trace, and atomic motion/declaration/group commit with surface publication;
- canonical renderer-neutral transformed/clipped/ordered paint-scene composition plus `RasterScale` and `PaintPublication` revision/base/damage/alignment authority;
- accepted M9 publication of common node background/outline decoration from final owner-local layout-box/radius facts, sampled presentation correlation across paint/hit/focus/semantic geometry and presentation-relative clips, exact inherited M6 pre-group ordering, explicit owner-local plus node-effect composition groups, runtime-resolved image mapping, conservative recursive paint/effect bounds, and alpha-independent neutral ordinary-shadow support/order facts;
- canonical transformed/clipped/ordered displayed `HitTestScene` composition, mounted-target/membership injection, retained displayed-generation lookup, and point/resolved-target authority;
- scene requirements derived from canonical paint content and neutral consumer capability checks without backend-specific rewriting;
- independent semantic publication/update/diagnostics and exact semantic-action admission/resolution.

Runtime must not depend on testing convenience, concrete native platforms, concrete renderer implementations, product state, external resource-provider/payload/cache ownership, font/shaping/line-breaking algorithm authority, SDF/MSDF realization, Lyon/backend tessellation, renderer shadow-mask/raster authority, or a second interaction/style/semantic/layout/motion/paint/hit/testing authority. Taffy types, node identity, topology, and caches remain private derived implementation state rather than public or retained framework authority.

## `runenui_render_wgpu`

`runenui_render_wgpu` is the accepted reusable concrete renderer edge over ordinary public paint publication. It owns:

- wgpu instance/adapter/device/queue and surface/offscreen target state;
- exact renderer-local successful-publication lineage and update/full-resync classification;
- caller-facing complete-`ResourceRef` provider requests for external image resources plus disposable image realization/cache state;
- consumption of exact retained `ShapedTextResource` bindings from `PaintPublication` and disposable renderer-private per-glyph SDF/MSDF generation, quality classes, atlas pages, GPU textures, cache lifetime, shader reconstruction, and antialiasing;
- private subordinate Lyon tessellation and disposable realization of accepted generic shape fill/stroke coverage, gradients, generic clips, runtime-published atomic groups, and ordinary-shadow masks/composition;
- renderer-private bounded requested-live-payload accounting and reusable scratch for ordinary-shadow mask realization, without exposing a public CPU-budget or scene-wide memory authority;
- native-surface presentation, offscreen readback, and immutable renderer observation records.

It consumes public core/runtime/text contracts only where required by paint realization. Caller-owned `ResourceProvider` remains the edge for external resources such as images; runtime-shaped text is resolved from the retained publication binding and is never recreated by that provider. M9 motion reaches the renderer only as already-sampled immutable publication products. Retained sampled publications may be retried or re-realized after cache loss or through a fresh renderer/device without a runtime resample or resource remint. The renderer must not own a native event loop, widget/semantic/mounted/layout authority, runtime mutation, application resource identity/bindings, shaping/line-breaking/font-discovery authority, style/theme/transition resolution, logical path containment/bounds, image fit policy, group ordering/scope, neutral shadow-support semantics/painter order, animation clock/timeline/easing/interpolation/lifecycle/reduced-motion policy, or AccessKit/winit behavior.

## `runenui_winit`

`runenui_winit` is the reusable native adapter edge proven by the second real M7 winit consumer. It owns only rebuildable translation/projection state:

- host-session mapping from native winit device identity to neutral `InputDeviceId` values;
- loss-preserving native keyboard lifetime, key, repeat, location, modifier, and cancellation translation;
- native mouse pointer/button lifetime translation, including multi-button and point-authority cancellation semantics;
- AccessKit tree projection over ordinary semantic publication, adapter-owned stable native identity, and exact AccessKit-action to semantic-action translation.

It consumes public core/runtime contracts plus winit/AccessKit types. It has no renderer dependency and must not own or hide a native window/event loop, runtime pump or mutation policy, wake/redraw/publication acknowledgement, displayed-frame authority, renderer configuration/recovery, presentation lifecycle, application behavior, or hidden style/theme authority. Explicit platform preference facts may be supplied through ordinary host-neutral style inputs without making the adapter the policy owner.

## `runenui_testing`

`runenui_testing` is a downstream public convenience crate. `TestHarness<App>` composes ordinary public core/runtime APIs with deterministic logical time, bounded pumping/settling, deterministic surface publication, read-only observation of the latest ordinary public paint/hit publication products and exact input context, synthetic public interaction, and semantic queries/targets. Accepted M9 integration proof uses that same public `ManualClock` path to observe exact timing, replacement, preference, structural layout/text, hit membership, and semantic results without a private expected-motion engine.

It owns no live runtime queue, mounted/semantic store, identity allocation, publication state, trace authority, resource provider, style/text/layout/motion authority, or private mutation bridge. A test target retains exact public surface/semantic scope; testing must not reconstruct private mounted routing identity, fabricate scene/publication lineage, duplicate hit resolution, or guess a surface from a bare semantic ID.

## Accepted native and external application integration

The accepted M7 native path remains host-owned rather than a generic framework runner. `examples/reference_winit` and the native Counter application each visibly own their winit window/event loop, runtime pumping and redraw driving, native coordinate mapping, renderer presentation/recovery policy, and displayed-frame authority. Both consume `runenui_winit` for the substantial native input/AccessKit mechanics that are independent of those application policies.

Native winit/AccessKit types remain outside `runenui_core` and `runenui_runtime`; `runenui_render_wgpu` remains winit-free; and neither native application becomes framework-owned loop authority. The second consumer justifies the bounded adapter crate, not a generic native application facade.

The accepted `tests/external_host` proof independently consumes ordinary public core/runtime contracts plus `runenui_render_wgpu` without importing winit, AccessKit, or testing convenience. Its caller-owned sequence visibly controls submit, pump, redraw consumption, publication, acknowledgement, render/retry, and presentation, including retained-publication retry after caller-owned external resource failure. This confirms the reusable external-host boundary is the existing public runtime/publication/renderer/resource contract rather than a framework-owned host facade.

## Core invariants

### Transient authoring, persistent runtime

`View`/`Element` values are owned transient descriptions derived from application state and consumed by reconciliation. Persistent identity, local widget state, lifecycle, focus/interaction state, work ownership, live motion/declaration state, and publication authority remain in the mounted runtime.

### Distinct identities

Authored IDs/keys, owner-local animation IDs, mounted IDs, semantic IDs, work/trace sequences, and surface identities have separate meanings. An `AnimationId` is declaration identity within one mounted owner, not a runtime-global clock/sample identity; a semantic ID is not a mounted-arena alias. Runtime-issued identities are runtime-local and must not be serialized or forged into live authority.

### One processing authority

Accepted application actions, routed commands/input, semantic actions after admission, effect/work transitions, timer/subscription events, and derived work converge through the runtime's canonical sequenced processing path. Motion sampling uses the accepted runtime clock/redraw/publication path and does not add one FIFO application action per sample. No public direct-dispatch or second event/action/animation queue may silently bypass ordering/default/trace semantics.

### Independent semantics

Widgets contribute platform-neutral owner-local semantic descriptions. Runtime validates/reconciles them into independently allocated semantic lifetimes and a renderer-independent surface-scoped publication. Public semantic consumers receive semantic identity/content, not mounted routing authority.

Exact semantic action requests are admitted against current published semantic authority and then converge on the canonical runtime command/default path. No second accessibility callback engine exists.

### One style computation model

Authored `StyleIntent`, explicit `StyleEnvironment` inputs, canonical transient interaction facts, and bounded parent inheritance converge through the core production resolver. Target-keyed transition policy is a non-inherited property-local policy in that same cascade; it is not a second transition cascade. Runtime supplies live interaction/activation facts, retains compatibility inputs, and drives invalidation; it does not maintain a second style tree or policy engine. Renderers and platform adapters consume or supply explicit neutral facts only and cannot reinterpret the style cascade or reduced-motion policy.

### One motion authority

Transient authored transition policy and explicit timelines describe motion; runtime alone owns live transition/declaration state and sampling time. One staged surface candidate observes one exact runtime monotonic instant, reconciles declarations/target changes and preferences at that instant, samples typed effective values, computes exact downstream effects, and commits motion state only with the existing surface transaction. Pure easing/interpolation is history-independent; time alone is not an invalidation/revision authority. Reduced motion derives only from explicit `StylePreferences::reduced_motion`; active visible motion uses existing redraw/wake scheduling rather than a framework frame rate or second timer queue. Retained renderer retry reuses the exact already-sampled publication.

### One layout authority

RunenUI-owned layout values and exact mounted topology are interpreted only by runtime. Taffy supplies private low-level Block/Flex/Grid algorithms and transaction-local disposable cache state; it does not own a retained UI tree, mounted identity, cross-frame layout authority, or public layout vocabulary. M9 sampled structural-layout overrides feed this same runtime/Taffy/text path and do not create an animation-specific layout engine. The final runtime-owned logical geometry and logical overflow/extents are the facts consumed by paint, hit testing, directional focus, and semantic bounds.

### One logical text computation model

RunenUI-owned text requests are resolved by `runenui_text`; runtime owns when that computation participates in mounted measurement/publication. One immutable logical artifact supplies both paragraph measurement and the exact shaped resource facts later painted. During production layout, intrinsic/compute-size text results are transient while the exact `TextLayoutState` produced for Taffy's final `PerformLayout` request is retained for publication. Paint does not independently reshape, line-break, discover fonts, or mint alternate shaped identity. Foreground remains paint-only when glyph geometry is unchanged.

### One integrated M8 production path

The accepted M8 closure preserves those separate ownership seams while proving their correlation. Exact Taffy known/available-space facts drive deterministic text requests inside the bounded layout transaction; the exact retained artifact/resource facts used for measurement are projected into paint; final semantic text and bounds use the same runtime-owned geometry; deterministic public tests use bundled fonts and controlled inputs; and the real wgpu renderer consumes the retained shaped resources through SDF/MSDF realization, including retry after renderer-cache loss and raster-scale/quality re-realization. None of those observations creates a second layout loop, text system, semantic tree, software expected renderer, or renderer-owned shaping authority.

### One visual/composition authority

M9 extends the accepted M6/M8 publication path rather than creating a second visual model. Core owns neutral visual values and exact logical geometry/brush/stroke/image/group/style semantics. Runtime resolves common node decoration and image mapping, composes sampled node presentation with owner placement, derives the exact M6 pre-group order, publishes snapshot-local group/effect structure and conservative bounds, and owns neutral ordinary-shadow support/order. Motion samples accepted visual/layout targets into that same authority and may retain opacity/shadow node-effect groups according to current sample plus accepted terminal/keyframe requirements; group lifetime remains staged runtime semantics rather than renderer/cache state. The wgpu renderer may tessellate, rasterize, allocate intermediate group targets/masks, sample gradients, and cache disposable resources, but those choices cannot redefine logical containment/bounds, item/group order, resource identity, clip semantics, effect scope/support, shadow spread/order, motion sampling, or style authority. Cache/target/device loss reconstructs from the complete retained publication plus retained/caller-owned resource bindings.

### One integrated M9 production path

M9C proves correlation rather than adding authority. The same sampled presentation transform drives visible paint and presentation-relative clips, exact transformed physical hit geometry, directional-focus geometry, and semantic AABBs while structural layout remains unchanged; singular transforms preserve empty-hit behavior. Canonical hover/focus/active facts feed the ordinary style cascade and transition planner. Public `ManualClock` integration observes exact timing/replacement/preference/layout/text/semantic outcomes through ordinary runtime/testing contracts. Real-wgpu consumes the exact retained sampled publication through retry, cache reconstruction, and fresh renderer/device reconstruction without advancing motion, rebinding resource identity, or creating renderer-side interaction/style/timeline state. The accepted pre-1.0 path retains one brush background, one generic shape/stroke vocabulary, one image mapping authority, one non-`Copy` shared `SceneShape`, and `ShapedTextRun` as the production text-paint authority rather than compatibility duplicates.

### Staged publication

Surface publication follows a staged transaction with admission, one candidate monotonic instant, read-only/staged style/motion/layout/scene planning, candidate-dependent final preflight, and commit. Recoverable refusal or terminal failure must not expose a partial new RunenUI-owned publication or partially advance motion start/replacement/completion/group state.

Renderer-facing paint products, hit/input products, semantics, layout, and diagnostics remain distinct authorities even when committed together. Accepted M6 uses immutable `PaintScene`/`PaintPublication` and `HitTestScene` products; scene requirements derive from canonical paint content, while raster scale, base revision, and damage remain paint-publication metadata. Paint revision identity remains distinct from displayed input generation and from motion time. Independent consumers can reconstruct complete deterministic scene snapshots from those public products without widget-kind, mounted/layout storage, private runtime, hidden prior-scene authority, or animation state.

For runtime-shaped text, retained paint publication lifetime also preserves the exact immutable logical shaped-resource bindings referenced by scene items. Renderer scale/quality/atlas/device state is disposable and can be reconstructed from those bindings without runtime republish, external provider lookup, reshaping, or `ResourceRef` reminting.

M9 composition/effect structure and sampled results are likewise immutable publication content, not a retained renderer tree/timeline. Renderer-private geometry, group targets, shadow masks and caches are disposable and reconstructible; renderer realization failure before submission does not mutate RunenUI publication or motion authority.

## Current limitations

The current public surface is pre-1.0 and may change incompatibly when accepted architecture requires a clean cutover. Important missing production capabilities include:

- broader production host/application ergonomics beyond the accepted proof-level native and external-host paths;
- virtualization and native scrolling mechanics beyond the accepted logical overflow/content/scroll extents;
- production text editing, selection, clipboard, and related behavior (M10);
- supported rendering for intrinsic COLR/SVG/bitmap glyph formats; current behavior diagnoses that breadth explicitly;
- multi-window lifecycle and supported platform-profile breadth;
- a complete standard control library.

M7 is accepted complete at proof maturity through the real wgpu renderer/resource edge, standalone winit host/native-input/presentation path, reusable winit/AccessKit adapter, native Counter showcase, and winit-free downstream external-host proof over the same public contracts. M8 is accepted complete at its production-foundation scope through deterministic production style resolution, renderer-neutral international text, runtime-owned Block/Flex/Grid layout with exact text feedback, retained measurement-to-paint resource identity, semantic content/bounds correlation, deterministic bundled-font public proof, and real-wgpu SDF/MSDF responsive/multiscript integration. M9 is accepted complete at its production visual/motion scope through static visual/composition vocabulary and runtime publication, deterministic transition/timeline semantics, integrated transform/interaction/public-manual-time correlation, real-wgpu motion retry/cache/device reconstruction, and final single-path authority cleanup. Current maturity is summarized in [status](../status.md). Durable future sequencing belongs in the [roadmap](../roadmap.md). Permanent observable/proof requirements live under [conformance](../conformance/README.md).

Do not infer support from a target ADR, design document, type name, or roadmap entry alone. Code/tests establish current behavior; source/Rustdoc establishes the exact public Rust surface.
