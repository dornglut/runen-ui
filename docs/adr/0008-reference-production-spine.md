# ADR 0008: Reference production spine

> **Category:** ADR
>
> **Status:** Accepted target architecture on exact-head owner acceptance
>
> **Decision date:** 2026-08-23
>
> **Milestone:** M7
>
> **Reviewed baseline:** `42df29bc68cfec97c13f80f0f59c209db512152c`
>
> **Acceptance:** this ADR becomes accepted target architecture only when the exact
> M7A0 package containing it is explicitly accepted by the repository owner,
> guarded-merged, and accepted-main validated. Acceptance freezes the ownership,
> dependency, and observable integration decisions below; it does not claim that
> any M7 renderer, host, resource realization, accessibility adapter, screenshot
> path, or external-host integration is implemented.

## Context

M6 completed the renderer-neutral paint/hit protocol at proof maturity. Accepted
`main` exposes immutable `PaintPublication`, `PaintScene`, `HitTestScene`, exact
`SurfaceInputContext`, symbolic `ResourceRef` identity, independently typed
semantic publication/action ingress, wake/redraw handshakes, and explicit bounded
runtime pumping. Two independent scene consumers prove that the public scene
protocol does not require widget-kind, mounted/layout, semantic, resource-provider,
or private-runtime authority.

M7 must now prove those neutral contracts through one real production spine:
real pixels, a real native window/input path, baseline image/text-resource
realization, native accessibility, screenshot evidence, instrumentation, and a
host-owned loop suitable for engine/game embedding.

The current source audit establishes that most host-neutral coordination already
exists:

- `AppRuntime::pump` is the explicit application/runtime work driver;
- `WakeTransport` is a narrow thread-safe wake notification seam;
- `take_redraw_request` / `acknowledge_redraw` expose revision-aware dirty
  publication coordination;
- `publish_surface` produces one aligned immutable `SurfacePublication`;
- pointer, keyboard, committed-text, composition, surface-command, and semantic
  action ingress already enter canonical runtime queues;
- `SurfacePublication` exposes paint, hit/input, semantics, layout/debug, and
  diagnostics as independently typed siblings;
- `ResourceRef` is an externally owned immutable process-local logical-resource
  identity and deliberately exposes no provider key, payload, lookup handle,
  decoder, shaper, or backend cache;
- `SemanticPublication` already supports complete snapshots, exact consecutive
  deltas, and full resynchronization, while `SemanticActionRequest` provides
  ordinary surface-scoped action ingress.

M7 therefore adds edge-owned integrations. It does not add a second runtime loop,
scene model, semantic tree, input authority, or core/runtime resource registry.

## Inherited authority this ADR does not supersede

This ADR composes rather than redefines accepted contracts:

- ADR 0004 owns mounted lifetime, reconciliation, lifecycle, and mounted identity;
- ADR 0005 owns canonical event routing, focus/input semantics, `SurfaceId`,
  exact displayed `SurfaceInputContext`, retained generations, stale/foreign/
  retired rejection, and no retargeting;
- ADR 0006 owns queue/pump, host requests, clock, wake/redraw, trace, shutdown,
  and transaction causality;
- accepted M5 owns semantic identity, publication, action, and testing contracts;
- ADR 0007 and M6 conformance own `PaintPublication`/`PaintScene`/`HitTestScene`,
  `PaintRevision`/base/damage, logical scene geometry/order/color/hit semantics,
  and `ResourceRef` identity;
- M8 owns production style/layout/international text;
- M10 owns complete editable-text interaction;
- later platform work owns the broad platform/multi-window/recovery matrix.

M7 conformance references inherited rows instead of duplicating them.

## Decision

### Adopt a thin reference stack at the edge

A0 evaluated the current compatible upstream releases on 2026-08-23 and adopts
these families for the M7 reference path:

- **winit 0.30.x** for the standalone native window/event-loop host;
- **wgpu 30.x** for the conventional cross-platform GPU renderer;
- **AccessKit 0.24.x** plus **accesskit_winit 0.33.x** for native accessibility;
- **image 0.25.x**, default formats disabled and PNG enabled, only in the
  reference resource owner/fixture;
- **ab_glyph 0.2.x** only for loading the deterministic fixture font and
  rasterizing already-selected glyphs.

The reviewed releases (`winit 0.30.13`, `wgpu 30.0.0`, `accesskit 0.24.1`,
`accesskit_winit 0.33.2`, `image 0.25.10`, `ab_glyph 0.2.32`) fit the repository's
Rust 1.93 compatibility floor. Exact patch versions, features, license inventory,
and transitive dependency state are revalidated by the implementation PR that
adds them; the version families are not RunenUI protocol.

M7 deliberately does not adopt a production shaping/layout stack merely to draw
text. Font discovery, fallback, shaping, bidi, line breaking, wrapping, paragraph
layout, and production text measurement remain M8 work.

### Keep native/backend dependencies outside neutral authority

No winit, wgpu, AccessKit, image-decoder, font-rasterizer, native-window, or
physical-geometry type becomes `runenui_core` or `runenui_runtime` behavior
authority.

The required dependency shape is:

```text
runenui_core <- runenui_runtime
                     ^
                     |
            runenui_render_wgpu
               ^             ^
               |             |
      reference_winit   external_host fixture
            |
        winit + AccessKit
```

M7 justifies one new reusable production edge crate:

- **`runenui_render_wgpu`** consumes ordinary public paint publications and an
  edge resource provider. It owns wgpu instance/adapter/device/queue, target
  surfaces/textures, pipelines, uploads, realization caches, render-update
  classification, offscreen readback, and renderer instrumentation. It has no
  winit or AccessKit dependency.

The standalone winit host and AccessKit glue remain a reference application until
a second real consumer proves a reusable host/adapter crate boundary. A generic
`runenui_platform` crate, a crate per concept, and a generic `runenui_resources`
crate are rejected without independent ownership pressure.

A separate downstream external-host fixture consumes `runenui_render_wgpu`
without winit or AccessKit. That independent consumer is the Cargo-enforced reason
the renderer cannot own the native event loop.

### Renderer consumes ordinary paint publication only

`runenui_render_wgpu` consumes `PaintPublication` and its complete `PaintScene`.
It may use public scene requirements/capability checks and caller-owned resource
payloads. It must not inspect or import concrete widgets/controls, `WidgetTypeId`,
semantic roles/tree state, mounted/layout storage, or private runtime mutation
seams.

The renderer tracks only renderer-owned realized state, including the exact
`(SurfaceId, PaintRevision)` it has successfully realized:

- same revision => already current;
- exact realized base => incrementally eligible;
- first observation, skipped revision, another surface, renderer reset, target
  rebuild, or lost realization state => full reprocessing of the complete current
  publication;
- damage is never consumed against a mismatched base.

Dropping renderer-local revision/GPU/resource caches must be recoverable from the
complete current publication plus the caller's resource provider. Runtime owns no
renderer acknowledgement protocol and no GPU handle.

M7 does not require partial present. A renderer may conservatively redraw the
complete target even when damage is incrementally eligible, while still exposing
the correct classification and damage facts.

### One render implementation supports window and offscreen targets

The same primitive/clip/resource implementation renders both:

1. a wgpu surface supplied by a native host target; and
2. a renderer-owned offscreen texture that can be copied to CPU-visible readback.

No software renderer becomes expected authority. Screenshot evidence must come
from the actual wgpu renderer. A wgpu noop/mock backend is insufficient pixel
proof.

Reference fixtures use controlled target format, scale, sampling, resources, and
geometry. Exact comparison is used for deliberately exact interior probes;
boundary/driver-sensitive pixels use a documented tight tolerance. The tolerance
must be small enough that geometry, order, clipping, opacity, color, and missing
resource defects cannot pass.

### Resource lookup and realization are edge-owned

M7 adds no core/runtime `ResourceRef -> payload` registry.

`runenui_render_wgpu` defines the minimum renderer-edge provider interface and
calls a provider owned by its caller. Lookup is keyed by the complete opaque
`ResourceRef`; consumers do not split, serialize, infer, or replace its identity.

The provider exposes two conceptual payloads:

- **image source:** immutable non-zero pixel extent plus explicitly normalized
  unpremultiplied RGBA8 sRGB pixels for an image-kind ref;
- **shaped-run source:** immutable resource-local logical placement/coverage facts
  sufficient to produce alpha coverage at a requested `RasterScale` for a
  shaped-text-run ref. Scene foreground remains ordinary paint state outside
  resource identity.

Exact Rust signatures remain implementation details, but ownership is fixed:

- application/reference provider owns logical payload storage and preserves the
  immutable binding for each live `ResourceRef`;
- decoder/font fixture work occurs in the provider, not core/runtime;
- renderer owns backend upload/texture/atlas/cache realization and may drop it at
  any time without changing the logical ref;
- scale-sensitive shaped-run coverage may be regenerated for the same logical ref;
- missing, wrong-kind, malformed, or unavailable payloads fail deterministically
  with structured diagnostics rather than silently selecting another provider,
  widget behavior, or placeholder control;
- provider lookup never becomes live runtime authority.

The reference provider uses `image` only for controlled PNG decoding with default
format features disabled, then normalizes output into the explicit RGBA8 sRGB
payload. Decoder color-space ambiguity does not become protocol semantics.

The shaped-run fixture uses one bundled redistributable font and explicit fixture
glyph selection/placement. `ab_glyph` loads/rasterizes those glyphs; it does not
perform production shaping, fallback, line breaking, or paragraph layout.

### winit owns native mechanics, not UI work

The standalone reference application owns the winit event loop and window. It:

- installs `WakeTransport` using an event-loop proxy/user event;
- calls `AppRuntime::pump` only from host-owned event-loop execution;
- translates runtime redraw intent to native redraw intent;
- supplies validated logical size and `RasterScale` through the existing surface
  build context;
- translates native pointer/keyboard/text/composition/focus events into existing
  host-neutral RunenUI values;
- drives the reusable renderer;
- keeps native IDs, native event objects, physical geometry, event-loop objects,
  and window handles out of core/runtime.

The host never invokes widget callbacks directly and owns no second UI queue.

### Redraw acknowledgement ends at successful publication, not presentation

M7 preserves ADR 0006 exactly. A `RedrawRequest` names dirty **publication** work,
not renderer/presenter success.

For a taken redraw request, the host follows this boundary:

1. reach a host frame opportunity and ensure the renderer target can be addressed;
2. call `publish_surface` for the dirty RunenUI revision;
3. if publication fails, do **not** acknowledge the request;
4. if publication succeeds, acknowledge that consumed `RedrawRequest` promptly;
5. retain the resulting complete immutable `SurfacePublication` and render/present
   it through the renderer.

A render/present failure after successful publication does not undo or defer the
runtime redraw acknowledgement. It is renderer/host recovery state. The host
retains the complete publication and schedules a native/render retry without
forcing RunenUI to republish unchanged state. If newer runtime invalidation later
arms another redraw, the host may publish the newer state; skipped renderer
revisions are handled by the M6 full-resync rule.

This distinction prevents GPU/window recovery from becoming a second runtime
publication authority.

### Displayed input tracks successful presentation, not publication

The host keeps one edge-owned displayed-surface record containing:

- the exact `SurfaceInputContext` from the publication that is actually displayed;
- the native physical extent and native scale used to present that publication.

The record changes only after successful presentation. A successfully published
but not yet presented frame does not become point-input authority.

A resize or scale-factor change can invalidate native physical-to-logical mapping
before a replacement frame is displayed. Point-based native ingress is therefore
admitted only while the displayed record matches the current native coordinate
mapping. While it does not match, point ingress is withheld and diagnosed until
a matching publication is successfully presented. Native points are never paired
with a stale context/new scale, substituted with the current runtime context, or
retargeted through current geometry.

If a renderer failure leaves an older frame displayed, that older displayed
context remains the host's target only while the native mapping still matches;
ordinary inherited stale/retired admission remains authoritative if runtime
history has moved beyond it.

Keyboard, committed-text, and composition ingress continue through their
focus-bound neutral APIs and do not gain fabricated surface context.

### Native keyboard/text/IME translation is loss-preserving

Named winit physical/logical keys map to corresponding RunenUI variants. Unknown
physical codes use `PhysicalKey::Code`; other logical names use
`LogicalKey::Named` rather than dropping information. Character key meaning is
not committed text.

Committed Unicode text uses `submit_text`. Native preedit/composition lifecycle
uses the existing start/update/end/cancel API. The host must not double-deliver
IME text as key meaning and committed text, and focus loss preserves inherited
composition/focus cleanup.

If a real native fact cannot be represented without semantic loss, implementation
stops for an explicit neutral-protocol review rather than storing native objects
inside runtime state.

### AccessKit consumes semantic publication only

The reference application uses AccessKit over ordinary `SemanticPublication`.
The adapter owns a per-surface mapping from live RunenUI `SemanticNodeId` to
AccessKit `NodeId`. That mapping is rebuildable adapter state:

- RunenUI semantic identity remains authoritative;
- retained RunenUI semantic lifetimes keep stable AccessKit IDs while one adapter
  instance is live;
- retired AccessKit IDs are not reused during that adapter lifetime;
- exact consecutive RunenUI semantic updates may be applied only from the
  adapter's realized surface/revision; otherwise the adapter rebuilds from the
  complete semantic snapshot.

Current role mapping is exact and deliberately small:

| RunenUI role | AccessKit role |
|---|---|
| `SemanticRole::Generic` | `Role::GenericContainer` |
| `SemanticRole::Group` | `Role::Group` |
| `SemanticRole::Text` | `Role::Label` |
| `SemanticRole::Button` | `Role::Button` |

For `Role::Label`, plain RunenUI text is exposed through AccessKit value/content
rather than inventing editable-text semantics. Name, description, value,
disabled/inert state, bounds, children, focus, and supported actions map only to
corresponding AccessKit facts. RunenUI relationships map directly where AccessKit
has the same relation: `LabelledBy -> labelled_by`, `DescribedBy -> described_by`,
and `Controls -> controls`. A published fact without a justified AccessKit
equivalent produces an explicit adapter diagnostic rather than behavioral
reinterpretation.

Current semantic actions map as follows:

| RunenUI action | AccessKit action |
|---|---|
| `Activate` | `Action::Click` |
| `RequestFocus` | `Action::Focus` |
| `OpenContextMenu` | `Action::ShowContextMenu` |
| `OpenMenu` | one adapter-owned `Action::CustomAction` entry with a deterministic ID and description `Open menu` |

`OpenMenu` round-trips only when the request is `Action::CustomAction` with the
published custom-action ID. It is never conflated with Click, Expand, or
ShowContextMenu. Other native actions are rejected/diagnosed unless a later
accepted RunenUI semantic action defines matching behavior.

### AccessKit callbacks cannot mutate runtime off-thread or reentrantly

The winit adapter is created before the native window is first shown, as required
by `accesskit_winit`.

The reference path uses `Adapter::with_mixed_handlers`:

- a direct activation handler may read only an adapter-owned thread-safe immutable
  latest full AccessKit tree derived from the latest RunenUI semantic publication;
  it never reads or mutates `AppRuntime` and may return `None` if no derived tree
  has been published yet;
- action requests and deactivation are delivered through the winit event-loop
  proxy and handled on the host thread;
- `Adapter::process_event` is called before the application handles each native
  window event, matching the upstream adapter contract;
- host-thread action handling resolves only adapter-owned AccessKit IDs, builds an
  ordinary `SemanticActionRequest`, submits it through `AppRuntime`, and lets the
  canonical queue/pump/default/trace path execute it.

The thread-safe AccessKit tree snapshot is a derived adapter cache, not semantic
authority: it is completely rebuildable from `SemanticPublication` plus the
adapter's ID mapping and cannot route directly to mounted state.

### Screenshot and instrumentation observe edge work

The renderer exposes immutable observation records sufficient to correlate:

- surface ID;
- paint revision/base and selected update mode;
- declared damage;
- logical and physical target extent plus raster scale;
- resource lookup/realization/cache outcomes;
- render target/backend/format;
- render, readback, and present outcomes.

These records observe public publications and renderer-owned work. They do not
allocate RunenUI identities, mutate runtime state, or replace the canonical
runtime trace.

The offscreen screenshot/golden path uses the same renderer/resource code as the
window path and copies actual wgpu output to CPU-visible readback. Golden files
are test evidence only.

### External host owns the frame loop explicitly

A separate downstream fixture with no winit or AccessKit dependency proves
real-time/game embedding. Its loop remains visibly host-owned:

1. accept host/application input/work;
2. pump `AppRuntime` under an explicit host-selected budget;
3. consume redraw intent when the host chooses a frame opportunity;
4. publish the surface;
5. acknowledge the consumed redraw immediately after successful publication;
6. render the retained publication using `runenui_render_wgpu` on an offscreen or
   host-supplied target;
7. consume/present the renderer result under host policy.

Renderer failure retries remain host/renderer work over retained publication;
no helper may hide an infinite framework loop, call `pump` from the renderer, or
require winit to use the renderer.

## Frozen implementation sequence

A0 freezes four serial slices. Later child issues are created only from accepted
`main` after their predecessor is accepted and accepted-main validated.

1. **M7A — reusable wgpu renderer and resource realization**
   - create `runenui_render_wgpu`;
   - implement renderer update classification, current M6 primitives, edge
     resource provider, deterministic image/shaped-run fixture realization,
     offscreen readback/goldens, and renderer instrumentation;
   - remain winit/AccessKit-free.
2. **M7B — standalone winit host and native input/scale/redraw**
   - build the real window path over M7A;
   - implement wake/redraw orchestration using the corrected publication
     acknowledgement boundary;
   - implement resize/scale/displayed-context and native input translation;
   - prove actual window-surface presentation and bounded recoverable surface
     recreation without claiming broad platform recovery.
3. **M7C — AccessKit semantic adapter**
   - implement exact current role/property/action mapping;
   - stable adapter-owned ID mapping plus delta/full-resync behavior;
   - mixed-handler activation snapshot and host-thread action round-trip;
   - no paint/widget/private-runtime dependency.
4. **M7D — external-host proof and M7 closure**
   - prove the winit-free host-owned frame loop using the same renderer/runtime
     contracts;
   - run integrated M4/M5/M6/M7 proof;
   - reconcile accepted M7 conformance/status only after implementation is
     accepted and accepted-main validated.

## Explicit non-goals

M7 does not own:

- production international text shaping/layout or font discovery/fallback;
- production style/layout breadth;
- editable-text completion or standard controls;
- broad visual effects/animation;
- complete Windows/macOS/Linux support/recovery matrix;
- multi-window completion;
- backend-specific widget semantics;
- a resource/provider/cache authority in core/runtime;
- a framework-owned engine/game main loop;
- one generic platform facade solely to hide selected upstream libraries.

## Consequences

The reference spine is intentionally asymmetrical: RunenUI defines neutral
behavior and edge packages adapt it to real systems. winit, wgpu, AccessKit,
decoding, and glyph rasterization remain replaceable because no selected upstream
identity enters core/runtime protocol.

The design also keeps two distinct notions explicit:

- **publication consumption** is acknowledged to RunenUI after successful
  `publish_surface`;
- **displayed presentation** becomes native point-input authority only after a
  renderer/presenter succeeds.

Conflating those would either make GPU recovery control runtime dirtiness or make
point input target a frame the user has not seen.

## A0 acceptance gate

M7 implementation remains blocked until this ADR and the M7 conformance matrix:

- match accepted source ownership and inherited ADRs;
- pass the configured matrix schema/ID/status/count audit;
- pass exact dependency/MSRV/license/version-policy review for the selected
  upstream families;
- pass `cargo +stable fmt --all --check` for the repository-audit Rust change;
- pass `cargo validate` and `git diff --check`;
- pass canonical exact-head CI;
- receive complete-diff critical review with no unresolved requested change;
- receive explicit repository-owner acceptance of that exact reviewed head;
- are guarded-merged and accepted-main validated.

M7 implementation issues must not be created before that sequence completes.
