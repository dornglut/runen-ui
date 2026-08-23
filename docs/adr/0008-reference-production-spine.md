# ADR 0008: Reference production spine

> **Category: ADR**
>
> **Status:** Accepted target architecture on owner acceptance
>
> **Decision date:** 2026-08-23
>
> **Milestone:** M7
>
> **Reviewed baseline:** `42df29bc68cfec97c13f80f0f59c209db512152c`
>
> **Acceptance:** This ADR becomes accepted target architecture only when the exact
> M7A0 package containing it is explicitly accepted by the repository owner and
> merged. Acceptance freezes the reference-spine ownership and dependency
> decisions below; it does not claim that a renderer, native host, resource
> provider, accessibility adapter, screenshot path, or external host integration
> is implemented. Observable implementation state remains owned by source/tests,
> accepted default-branch state, and the M7 conformance matrix.

## Context

M6 completed the renderer-neutral paint/hit protocol at proof maturity. The
accepted default branch now exposes immutable `PaintPublication`, `PaintScene`,
`HitTestScene`, exact displayed `SurfaceInputContext`, symbolic `ResourceRef`
identity, independently typed semantic publication, public semantic-action
submission, wake/redraw handshakes, and explicit bounded pumping. Two
independent scene consumers prove that these products can be interpreted without
widget-kind, mounted-storage, layout-storage, semantic-role, resource-provider,
or private-runtime authority.

That is deliberately not yet a production UI path. There is no concrete GPU
renderer, native window/event-loop host, native accessibility adapter, resource
payload/realization path, screenshot output, or real-time/engine-owned frame-loop
consumer. M7 must add one thin real vertical spine without moving those edge
concerns back into the neutral kernel.

The accepted source already contains most host-neutral coordination required by
that spine:

- `AppRuntime::pump` remains the sole explicit application/runtime work driver;
- `WakeTransport` is a narrow thread-safe notification seam and never owns work;
- `take_redraw_request` / `acknowledge_redraw` expose revision-aware redraw
  coordination without native window types;
- `publish_surface` produces the complete aligned public publication;
- pointer, keyboard, committed-text, composition, surface-command, and semantic
  action ingress already converge through canonical runtime queues;
- `SurfacePublication` exposes paint, hit/input, semantics, layout/debug, and
  diagnostics as distinct immutable siblings;
- `ResourceRef` is already an opaque externally owned immutable logical-resource
  identity and is usable as a map key, while core/runtime intentionally expose no
  provider lookup, payload, decoder, shaper, or backend cache;
- `SemanticPublication` already supports complete snapshots, exact consecutive
  deltas, and full resynchronization, and `SemanticActionRequest` provides
  ordinary surface-scoped action ingress.

M7 therefore needs edge-owned integration, not a new runtime loop, second scene
model, second semantic tree, or core-owned resource subsystem.

## Inherited authority this ADR does not supersede

This ADR composes accepted contracts. It does not redefine:

- ADR 0004 mounted lifetime, reconciliation, lifecycle, or mounted identity;
- ADR 0005 canonical event routing, focus/input semantics, `SurfaceId`,
  `SurfaceInputContext`, retained displayed generations, stale/foreign/retired
  rejection, or no-retargeting behavior;
- ADR 0006 canonical queue, explicit pump, host-request, clock, wake/redraw,
  trace, shutdown, or transaction causality;
- accepted M5 semantic identity, contribution, publication, action, or testing
  authority;
- ADR 0007 paint/hit scene ownership, `PaintRevision`/base/damage semantics,
  resource-reference identity, logical scene geometry, order, opacity,
  source-over, hit testing, or renderer-neutral consumer rules;
- M8 production style/layout/international-text ownership;
- M10 editable-text behavior;
- M13 complete platform/multi-window/resource/recovery support.

M7 conformance references inherited M4/M5/M6 observations rather than
re-stating them under new IDs.

## Decision

### Adopt a thin reference stack at the edge

M7 adopts the following upstream families for the reference spine:

- **winit 0.30.x** for the standalone reference window/event-loop host;
- **wgpu 30.x** for the conventional reference GPU renderer;
- **AccessKit 0.24.x** plus **accesskit_winit 0.33.x** for the native
  accessibility bridge used by the winit reference application;
- **image 0.25.x**, with default features disabled and PNG enabled, only in the
  reference resource owner/fixture for decoding deterministic image fixtures;
- **ab_glyph 0.2.x**, with only the features needed for ordinary font loading and
  glyph rasterization, only in the reference resource owner/fixture for the
  bounded M7 shaped-run proof.

These version lines are implementation baselines, not RunenUI public protocol.
Dependency updates that preserve this ADR and conformance are maintenance work;
a change that alters ownership, event-loop semantics, resource identity,
renderer/publication semantics, or accessibility round-tripping requires renewed
architecture review.

At the decision date the selected winit, wgpu, AccessKit, and image releases all
fit below RunenUI's Rust 1.93 compatibility floor and use Apache-2.0 and/or
MIT-compatible licensing. `ab_glyph` is Apache-2.0 and deliberately narrower
than a paragraph layout/shaping stack. Every implementation PR must still run the
repository's dependency, license, MSRV, and feature audits against the exact
versions it adds.

M7 explicitly does **not** adopt `cosmic-text`, `glyphon`, `fontdue` layout, or
another production shaping/layout stack. M8 owns the reviewed production text
stack. M7 needs only enough deterministic font/glyph realization to put the
already-shaped M6 resource path on screen.

### Preserve neutral core/runtime dependency direction

No winit, wgpu, AccessKit, raw native-window, image-decoder, or font-rasterizer
type may appear in `runenui_core` or `runenui_runtime` public or private behavior
authority merely to serve the reference path.

The intended dependency direction is:

```text
runenui_core <- runenui_runtime
                     ^
                     |
            runenui_render_wgpu
               ^             ^
               |             |
      reference_winit   external_host fixture
            |  |
          winit AccessKit
```

The diagram names dependency pressure, not a requirement to create every concept
as a crate.

M7 justifies exactly one new reusable production edge crate at A0:

- **`runenui_render_wgpu`** — an independently consumable renderer package over
  ordinary `runenui_core`/`runenui_runtime` paint publications. It owns wgpu
  device/queue/surface/texture/pipeline state, resource realization caches,
  render/update classification, offscreen readback, and renderer-side
  instrumentation. It has no winit or AccessKit dependency.

The standalone winit host and AccessKit glue remain in a reference application
package until a second real consumer demonstrates a reusable host/adapter crate
boundary. A monolithic `runenui_platform` crate and a crate-per-concept graph are
rejected.

The external-host proof is a separate downstream fixture that consumes the same
`runenui_render_wgpu` package without winit. That independent consumer is the
Cargo-enforced reason renderer and host ownership must remain separate.

### Renderer consumes only ordinary public paint products

`runenui_render_wgpu` consumes the exact ordinary `PaintPublication` and its
complete `PaintScene`. It may use public scene requirements/capability checks and
renderer-owned resource payloads. It must not import or inspect:

- `WidgetTypeId` or concrete built-in controls;
- semantic roles/state/tree facts;
- mounted or layout storage;
- private runtime modules or mutation seams;
- a second renderer-specific scene authored by widgets.

The renderer tracks only its own realized renderer state, including the exact
`(SurfaceId, PaintRevision)` it has successfully realized. It follows M6
revision/base semantics:

- same revision is already current;
- an exact realized base may make publication damage incrementally eligible;
- first observation, skipped revision, another surface, renderer reset, target
  rebuild, or lost cache requires full reprocessing of the complete publication;
- damage is never used when the realized base does not match;
- RunenUI runtime receives no renderer acknowledgement protocol beyond ordinary
  host redraw acknowledgement already defined by ADR 0006.

The first M7 renderer may conservatively redraw the complete target even when
incremental damage is eligible. M7 requires correct revision/base/damage
interpretation and observation, not a premature partial-present optimization.

GPU device/queue/surface/pipeline handles and command submission are renderer
state. They never become runtime state.

### One render core supports window and offscreen targets

The same primitive/clip/resource implementation must render both:

1. a wgpu window surface supplied by the standalone reference host; and
2. a renderer-owned offscreen texture that can be copied to a CPU-visible
   readback buffer for deterministic screenshot/golden proof and the external
   host fixture.

There is no second software renderer used as expected behavior. CPU reference
calculations may continue to test logical geometry/color contracts inherited
from M6, but screenshot output must come from the actual wgpu renderer.

Reference targets use an sRGB-capable 8-bit format when available. Literal M6
colors retain their accepted unpremultiplied sRGB8 plus linear-alpha semantics;
the renderer performs blending in a way consistent with the accepted linearized
source-over contract. Resource-backed image payload color semantics are frozen
below rather than delegated to decoder defaults.

### Resource ownership stays external to core/runtime

M7 introduces no `ResourceRef -> payload` registry in core/runtime.

`runenui_render_wgpu` defines the minimum **renderer-edge provider interface**
needed by this renderer. The caller owns an object implementing that interface
and supplies it when rendering. The interface is keyed by the complete
`ResourceRef`; consumers never split, serialize, guess, or replace its opaque
identity.

The provider contract has two conceptual resource products:

- **image source** — immutable decoded width/height plus tightly defined
  unpremultiplied RGBA8 sRGB pixels for an image-kind `ResourceRef`;
- **shaped-run raster source** — immutable resource-local logical bounds plus an
  alpha8 coverage raster produced for a requested `RasterScale` from the same
  shaped-run logical resource. Scene foreground remains outside resource identity
  and comes from `ShapedTextRunPrimitive`.

Exact Rust names/signatures are implementation details, but the ownership is
normative:

- the application/reference provider owns logical payload storage and preserves
  the immutable-content binding for the life of each `ResourceRef`;
- image decode and test-font/glyph rasterization occur in the provider/reference
  application, not core/runtime;
- the wgpu renderer owns texture/atlas/upload/cache realization and may drop and
  rebuild it at any time without changing logical resource identity;
- a scale-sensitive shaped-run raster may be regenerated for the same logical
  `ResourceRef`; renderer cache identity therefore includes realization scale as
  needed without issuing a new logical reference;
- missing, wrong-kind, malformed, or unavailable payloads produce deterministic
  structured render diagnostics/errors and never silently substitute widget
  semantics or another provider;
- provider callback/lookup must not be retained as live runtime authority.

A generic `runenui_resources` crate is deliberately rejected until another
renderer or independent consumer proves that the payload vocabulary itself is a
shared product boundary.

### Image decoding is a reference-provider concern

The reference application uses `image` only to decode controlled PNG resources,
with default image-format features disabled. Decoder output is explicitly
normalized into the renderer-edge RGBA8 sRGB payload contract; M7 does not rely
on ambiguous decoder color-space defaults as protocol semantics.

Image fit/crop/repeat behavior remains exactly the M6 scene contract: the complete
normalized image domain maps to the primitive destination. The provider does not
invent fit policy.

### Baseline text proves resource realization without stealing M8

M7's text goal is real pixels from the existing M6 `ShapedTextRun` resource path,
not production shaping.

The reference resource owner uses a deterministic bundled redistributable test
font and a small fixed authored glyph sequence. `ab_glyph` is used only to load
that font, address already-selected glyphs, and rasterize glyph coverage. The
reference provider produces immutable resource-local bounds/coverage for a
`ResourceRef::ShapedTextRun`; it does not become layout authority.

M7 does not add:

- font discovery or fallback;
- script/language shaping;
- bidi;
- line breaking/wrapping;
- paragraph layout;
- production text measurement;
- editable text or selection.

Those remain M8/M10. A later production text stack may replace the reference
provider's glyph-production mechanism without changing M6 scene identity or the
renderer/provider ownership boundary.

### winit owns native event-loop mechanics, not UI work authority

The standalone reference application uses winit's application/event-loop model.
Its responsibility is translation and orchestration only:

- install a `WakeTransport` backed by a winit event-loop proxy/user event;
- call `AppRuntime::pump` only from the host-owned event loop;
- take runtime `RedrawRequest`s and translate them to native redraw requests;
- on redraw, publish the current RunenUI surface, render/present it, then
  acknowledge the consumed redraw request only after successful publication and
  render handoff under the accepted redraw contract;
- translate winit physical size/scale to validated logical surface size and
  `RasterScale` supplied through the existing `SurfaceBuildContext`;
- translate pointer/keyboard/text/composition/focus events into existing
  host-neutral RunenUI event values;
- keep native window/device IDs, `PhysicalPosition`, `PhysicalSize`, winit keys,
  and event-loop objects out of core/runtime.

The host never dispatches widget callbacks directly and never runs a second UI
queue.

### Displayed-surface state is an explicit host invariant

Native point input requires both a physical-to-logical mapping and an exact
RunenUI displayed `SurfaceInputContext`. A resize or scale-factor event can
change the native mapping before a newly published RunenUI frame is actually
shown. The host must not combine those facts from different displayed states.

The reference host therefore keeps one edge-owned **displayed-surface record**
containing the successfully presented publication's input context together with
the physical extent and native scale used to present it.

Point-based native ingress is admitted only while that record matches the current
native coordinate mapping. After a resize/scale transition invalidates the
mapping, point ingress is withheld until a new publication has been successfully
rendered/presented and becomes the new displayed record. Events are never
retargeted through current runtime geometry, and the host does not forge a
replacement input context. The implementation must expose a diagnostic for this
transition rather than silently delivering stale point input.

Keyboard, committed-text, and composition ingress continue through their existing
focus-bound host-neutral APIs; they do not gain a fake surface context.

This rule is the native-host preservation of accepted M4 displayed-generation
semantics.

### Native key/text/IME translation is loss-preserving

winit physical keys that have named RunenUI equivalents map to those variants;
other physical codes map losslessly to the owned `PhysicalKey::Code` form.
Logical character meanings map to `LogicalKey::Character`; other named meanings
map to the existing named variants or `LogicalKey::Named` without pretending key
meaning is committed text.

Committed Unicode text uses `submit_text`. Platform composition/preedit events
use the existing start/update/end/cancel composition lifetime. The host must not
double-deliver IME text as both key meaning and committed text, and focus loss
must preserve accepted cancellation/cleanup behavior.

M7 does not expand the neutral input vocabulary unless an observed winit/native
fact cannot be represented without semantic loss. Such a gap is a protocol defect
requiring explicit review, not permission to stash native event objects in the
runtime.

### AccessKit is a semantic adapter, not semantic authority

The reference application uses `accesskit` plus `accesskit_winit` over ordinary
`SemanticPublication`.

The adapter owns a per-surface mapping between RunenUI `SemanticNodeId` values
and AccessKit `NodeId` values. That mapping is adapter state only:

- RunenUI semantic identity remains authoritative;
- AccessKit IDs remain stable for retained RunenUI semantic lifetimes while one
  adapter instance is live;
- retired AccessKit IDs are not reused in that adapter lifetime;
- a complete adapter reset may rebuild from a RunenUI full snapshot without
  changing RunenUI semantic identity.

The current RunenUI semantic vocabulary maps deliberately:

- `SemanticRole::Generic` -> AccessKit generic/presentational container role;
- `SemanticRole::Group` -> the nearest AccessKit grouping role;
- `SemanticRole::Text` -> AccessKit label/text presentation with plain text
  exposed as value/content rather than inventing an editable-text model;
- `SemanticRole::Button` -> AccessKit button;
- RunenUI name, description, value, disabled/inert state, absolute bounds,
  children, relationships, focus, and supported actions are translated where
  AccessKit exposes the corresponding neutral fact;
- any currently published RunenUI fact without a justified AccessKit equivalent
  produces an adapter diagnostic rather than being silently reinterpreted as a
  different UI behavior.

The adapter consumes consecutive `SemanticUpdate`s only when its realized
surface/revision exactly matches the declared previous revision; otherwise it
rebuilds from the complete `SemanticSnapshot`.

AccessKit action callbacks never mutate RunenUI reentrantly. They resolve the
adapter-owned AccessKit ID mapping to the exact RunenUI semantic target, translate
supported native actions (`Click`, `Focus`, context-menu equivalents) to the
corresponding current `SemanticAction`, and enqueue a host event/user message.
The host thread then constructs an ordinary `SemanticActionRequest` and calls
`AppRuntime::submit_semantic_action`. Unsupported AccessKit actions are diagnosed
and do not bypass canonical ingress.

### External hosts own their frame loop

The reusable renderer API must be usable without winit. A dedicated downstream
external-host fixture demonstrates an engine/game-style loop that explicitly
decides when to:

1. submit neutral input/work;
2. pump the RunenUI runtime;
3. consume redraw intent;
4. publish a surface;
5. render the publication to an offscreen or host-supplied target;
6. present/consume the resulting frame;
7. acknowledge redraw.

No helper may hide a framework-owned infinite loop or require winit to use the
renderer. The fixture may use deterministic iteration rather than wall-clock
real-time; ownership of the loop is the required observation.

### Renderer/host instrumentation is observational

M7 adds edge instrumentation sufficient to correlate production behavior without
becoming a second trace authority. Renderer/host observations include at least:

- `SurfaceId`;
- current `PaintRevision` and optional base revision;
- whether the renderer classified the publication as already-current,
  exact-base/incremental-eligible, or full-resync;
- logical/physical target size and raster scale;
- declared damage;
- resource lookup/realization/cache-hit/cache-rebuild events by opaque ref/kind
  without exposing pointer-derived identity;
- render/readback/present success or failure.

Instrumentation is immutable output/diagnostics. It cannot mutate runtime state,
forge revisions, or replace the accepted runtime trace.

### Screenshot/golden proof uses the actual renderer

The canonical screenshot proof renders controlled fixtures through the same
`runenui_render_wgpu` implementation into an sRGB offscreen target and reads the
result back.

A canonical Linux Vulkan software-adapter job may be used to make CI rendering
repeatable without requiring physical GPU hardware. This still exercises the
actual wgpu renderer/backend and produces real pixel output; the noop backend is
not valid M7 pixel proof.

Goldens use deterministic bundled resources, controlled raster scale, and an
explicit comparator policy. The first renderer implementation must document its
chosen tolerance before golden acceptance. The comparator may allow bounded
backend edge/rounding variation, but it must not permit geometry/order/clip/color
or missing-resource divergences. Cross-backend pixel identity is not an M7 goal;
logical scene semantics remain owned by M6 conformance.

### Recovery uses complete public snapshots

Renderer target/surface recreation, dropped GPU realization caches, or lost
renderer-local revision state never require hidden prior RunenUI scenes. The
renderer reprocesses the complete current `PaintPublication` and re-resolves
resources by stable `ResourceRef` values.

M7 does not claim complete production device-loss/recovery policy across all
platforms; M13 owns that breadth. It does prove that the public M6 snapshot model
is sufficient to reconstruct the reference renderer after renderer-local state
loss.

## M7 implementation sequence

The architecture yields four serial slices after M7A0 acceptance and
accepted-main verification:

1. **M7A — reusable wgpu renderer and edge resource realization**
   - create `runenui_render_wgpu`;
   - implement complete current M6 paint semantics;
   - define the renderer-edge resource provider products;
   - implement image/shaped-run realization, revision/resync logic,
     instrumentation, offscreen rendering, readback, and canonical golden proof.
2. **M7B — standalone winit host and native input/scale/redraw path**
   - add the reference winit application;
   - integrate wake/pump/redraw, native scale/resize, displayed-surface point
     ingress, keyboard/text/IME/focus translation, wgpu window presentation, and
     real-window smoke proof.
3. **M7C — AccessKit semantic adapter path**
   - integrate snapshot/delta mapping, current semantic vocabulary, focus/state,
     relationships/actions, adapter ID lifetime, and non-reentrant semantic
     action round-trip through the reference host.
4. **M7D — external-host proof and milestone closure**
   - add the winit-free downstream external-host consumer;
   - rerun integrated M4/M5/M6/M7 proof;
   - reconcile accepted status/architecture only after implementation acceptance;
   - close M7 only after accepted-main validation.

Child issues are created serially from then-current accepted `main`; M7A is not
created until M7A0 itself is accepted, merged, and accepted-main validated.

## Alternatives rejected

### Build a custom native event loop/window layer now

Rejected. M7 needs to validate RunenUI's neutral contracts against a real mature
host, not spend the milestone rebuilding platform window/event infrastructure.
M13 can revisit platform ownership if production evidence demonstrates a missing
abstraction.

### Use softbuffer/pixels as the reference renderer

Rejected as the primary renderer. They are useful presentation/pixel-buffer
layers but do not exercise the conventional GPU pipeline, resource realization,
transforms/clips, and renderer ownership the M7 roadmap intends to prove. wgpu
provides the stronger reference consumer while still remaining backend-neutral
from RunenUI's perspective.

### Adopt a production text/shaping renderer in M7

Rejected. `cosmic-text`/glyphon or equivalent would collapse M8's deliberate
text-stack decision into a milestone whose requirement is only baseline font
resource realization. The M7 reference font provider is intentionally bounded
and replaceable.

### Put resource lookup in core/runtime

Rejected. M6 deliberately made resource identity external and opaque. The wgpu
renderer is the first real consumer that needs payloads, so the provider seam
belongs at that edge. Moving it into runtime would create provider/cache
lifecycle authority unrelated to UI behavior and would couple external hosts to
one storage model.

### Create generic render/resource/host/accessibility crates before use

Rejected. Only `runenui_render_wgpu` currently has a demonstrated independent
consumer boundary: both the winit reference path and the external-host proof need
the renderer without sharing host ownership. Other extraction waits for real
reuse pressure.

### Let the framework own the native/game frame loop

Rejected. The roadmap explicitly requires external host-controlled execution,
and existing explicit pumping/wake/redraw APIs already support it. A hidden loop
would weaken embedding and create a second scheduling owner.

## Consequences

Positive consequences:

- M7 tests the accepted neutral kernel against real host/render/accessibility
  ecosystems without contaminating core/runtime with native dependencies;
- the renderer becomes independently reusable by standalone and engine-style
  hosts;
- resource realization gains one concrete, minimal owner without inventing a
  framework resource registry;
- baseline font pixels are proven without preempting M8 shaping/layout;
- native resize/scale input cannot silently violate displayed-generation safety;
- AccessKit action callbacks converge through ordinary semantic ingress;
- offscreen rendering makes real pixel proof possible in CI and downstream
  external hosts.

Costs and risks:

- wgpu materially increases compile time and dependency weight in the dedicated
  renderer crate;
- native host/accessibility behavior has platform-specific operational details
  even though the framework contract remains neutral;
- a canonical software-renderer golden environment requires dedicated CI setup;
- the M7 reference resource payload interface may later need extraction or
  revision when M8 production text or a second renderer provides real pressure;
- withholding point input during a native resize/scale transition is a deliberate
  safety policy that later production hosts may refine only if they can preserve
  exact displayed mapping.

Those costs are accepted because they are isolated at real edge boundaries rather
than imposed on the neutral kernel.

## Non-goals

M7A0 and this ADR do not provide or promise:

- complete Windows/macOS/Linux qualification;
- mobile/web profiles;
- multi-window/multi-surface lifecycle completion;
- production text shaping, fallback, bidi, line breaking, or editing;
- production layout/style/control breadth;
- visual effects/animation breadth;
- complete renderer device-loss/recovery policy;
- one mandatory renderer for future RunenUI consumers;
- public stable package/API compatibility before the release milestones;
- a RunenUI-owned application/game main loop.

Those outcomes remain with their accepted roadmap owners.