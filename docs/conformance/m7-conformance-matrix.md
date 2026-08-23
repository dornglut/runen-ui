# M7 Conformance Matrix

> **Category: Target architecture**
>
> **Status:** Accepted target contract on owner acceptance
>
> **Milestone:** M7
>
> **Reviewed baseline:** `42df29bc68cfec97c13f80f0f59c209db512152c`
>
> **Acceptance condition:** This matrix becomes normative only when the exact
> M7A0 architecture/conformance package containing it is explicitly accepted by
> the repository owner and merged. That acceptance freezes the M7 target contract
> and implementation sequence; it does not promote any behavior row. M7
> implementation remains blocked until the accepted M7A0 tree is itself validated
> on `main`.

This matrix is the single M7-specific observable behavior and proof inventory.
[ADR 0008](../adr/0008-reference-production-spine.md) owns the reference host,
renderer, resource, accessibility, screenshot, instrumentation, and external-host
architecture. Accepted M4 remains authoritative for routed/native-neutral input,
focus, displayed `SurfaceInputContext`, stale/foreign/retired generations, and
wake/redraw/runtime work semantics. Accepted M5 remains authoritative for
semantic identity/publication/action ingress and publication atomicity. Accepted
M6 remains authoritative for `PaintPublication`/`PaintScene`/`HitTestScene`,
logical geometry/order/color/hit semantics, `PaintRevision`/base/damage, and
`ResourceRef` identity.

M7 adds real edge consumers of those contracts. It does not duplicate inherited
observations under new IDs.

```text
20 total unique rows
0 owner-accepted
0 implementation-complete
0 proof-complete
20 blocked
0 duplicate IDs
0 invalid statuses
0 invalid schemas
```

## Row contract and completion rule

Every ID is permanent. New observations append the next zero-padded number in
that family; IDs are never recycled because implementation moves. Allowed status
meanings remain:

- `blocked`: the owning implementation slice has not been accepted;
- `implementation-complete`: public behavior exists but the complete proof
  package has not passed;
- `proof-complete`: exact-head positive/negative/diagnostic or trace proof and
  validation pass, but owner acceptance and merge remain pending;
- `owner-accepted`: public behavior, complete proof, validation, critical review,
  explicit owner acceptance, guarded merge, content identity, and required
  accepted-main validation have passed.

`Required` means the row must be `owner-accepted` before M7 closes. M7A0 has no
behavior rows. Delivery slices frozen by ADR 0008 are:

- M7A — reusable wgpu renderer and edge resource realization;
- M7B — standalone winit host and native input/scale/redraw path;
- M7C — AccessKit semantic adapter path;
- M7D — external-host proof and milestone closure.

Proof through a private runtime bridge, widget-kind renderer path, semantic facts
inside paint input, forged `SurfaceInputContext`, core/runtime resource registry,
second expected renderer, hidden framework-owned game loop, or native type leaked
into the neutral kernel is invalid M7 conformance.

## M7A — reusable wgpu renderer and edge resource realization

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| RENDER-01 | A reusable `runenui_render_wgpu` package consumes ordinary public `PaintPublication`/`PaintScene` plus renderer-edge resource payloads and can be used without winit or AccessKit. Its renderer path imports no concrete widget/control kinds, semantic roles/tree facts, mounted/layout storage, or private runtime mutation seam; wgpu device/queue/surface/pipeline/command state remains renderer-owned. | Package dependency audit plus ordinary-publication render fixture and winit-free downstream consumer compile proof | `WidgetTypeId`/built-in match, semantic renderer input, mounted/layout lookup, private runtime dependency, winit/AccessKit renderer dependency, or wgpu handle in core/runtime audit | Renderer construction/dependency diagnostics plus repository boundary audit | M7A | blocked | Required |
| RENDER-02 | The wgpu renderer produces real pixels for every current M6 paint primitive while preserving accepted logical item order, transforms, conjunctive clips including rounded clips, opacity, literal fill/stroke/shaped-run foreground and source-over semantics, image destination mapping, shaped-run origin, logical coordinate space, and `RasterScale`; backend raster edge sampling may vary only where M6 explicitly leaves it backend-specific. | Offscreen/window render corpus covering fill, stroke, image, shaped run, order/layers, transforms, singular transforms, clips/rounded clips, opacity and scale | Backend resorting, semantic/widget reinterpretation, hidden layout placement, clip union/last-wins, scale-mutated scene geometry, alternate blend/color contract, zero-width hairline, or unsupported primitive silent fallback proof | Per-frame renderer diagnostics plus selected pixel/probe evidence correlated to public scene items | M7A | blocked | Required |
| RENDER-03 | Renderer update classification is exact for the realized `(SurfaceId, PaintRevision)`: same revision is already-current; exact base is incrementally eligible; first/skipped/foreign/reset state is full-resync. Damage is never consumed against a mismatched base. Dropping renderer-local revision/GPU caches permits reconstruction from the complete current publication with no hidden prior RunenUI scene or runtime acknowledgement state. | Contiguous, unchanged, skipped, cross-surface, renderer-reset and cache-reset render corpus | Mismatched-base damage application, global revision conflation, hidden previous scene/cache requirement, runtime-held renderer acknowledgement, or stale GPU resource authority proof | Renderer update-mode/revision/base/damage instrumentation | M7A | blocked | Required |
| RESOURCE-01 | Renderer resource resolution is caller-owned and keyed by the complete opaque `ResourceRef`; the provider preserves one immutable logical-content binding for each live ref, validates kind, and supplies no provider key to RunenUI. Renderer realization/cache state may be dropped and rebuilt without rebinding the ref. Missing, wrong-kind or malformed resources fail deterministically rather than selecting another provider or widget fallback. | Multi-ref provider fixture, cache-drop/rebuild and missing/wrong-kind failure corpus | Core/runtime provider registry, split-key/provider guessing, same-ref logical rebinding, backend handle stored as logical identity, silent placeholder/control fallback, or provider lookup in widget/runtime proof | Resource lookup/kind/realization/cache diagnostic stream keyed only by opaque ref/kind | M7A | blocked | Required |
| RESOURCE-02 | Image-kind resources resolve to an explicit decoded unpremultiplied RGBA8 sRGB payload with finite non-zero pixel extent. The renderer samples the complete image domain into the exact M6 destination with no implicit crop/contain/cover/repeat policy. Reference PNG decoding is provider-owned and its decoder defaults cannot redefine protocol color/fit semantics. | Deterministic PNG/provider/image render corpus including same payload at multiple logical destinations/scales | Decoder/backend fit policy, ambiguous decoder color-space semantics treated as protocol, raw compressed bytes interpreted in runtime, image provider in core/runtime, or hidden crop/repeat proof | Image decode/normalize/upload/cache diagnostics plus golden/probe output | M7A | blocked | Required |
| RESOURCE-03 | Shaped-text-run resources resolve through the provider to immutable resource-local logical bounds plus alpha coverage realized for the requested `RasterScale`; scene foreground remains separate literal paint state. The same logical ref may be re-realized at another raster scale without changing resource identity. The M7 provider uses a deterministic bundled font/glyph rasterizer only for already-selected glyphs and performs no production shaping, fallback, bidi, line breaking, wrapping or paragraph layout. | Bundled-font shaped-run corpus across foreground changes, raster scales, renderer cache loss and identical logical ref | Foreground encoded into resource identity, new ref required only for scale, renderer/widget text reshaping, font discovery/fallback, production layout authority, or hidden text semantics proof | Shaped-run provider/raster/cache diagnostics plus golden/probe output | M7A | blocked | Required |
| GOLDEN-01 | The actual wgpu renderer renders controlled fixtures to its offscreen target, copies the same output to CPU-visible readback and compares it under a documented bounded pixel policy. The golden path uses the same render/resource implementation as the window path and does not use a second software renderer as expected authority. A real rendering backend, not wgpu noop, supplies pixel evidence. | Canonical offscreen readback/golden corpus with deterministic bundled resources and controlled scale | Separate expected renderer, mocked/noop pixel source, manually authored expected scene bypass, tolerance that admits geometry/order/missing-resource defects, or fixture-only renderer code path proof | Golden comparator diagnostics including backend/adapter/format/scale and bounded mismatch details | M7A | blocked | Required |
| OBS-01 | Renderer observation exposes immutable frame facts sufficient to correlate public `SurfaceId`, paint revision/base, update mode, declared damage, logical/physical target extent, raster scale, resource realization/cache events and render/readback/present result without mutating runtime state or replacing the canonical runtime trace. | Instrumentation snapshot/observer tests correlated to exact publications and renderer outcomes | Renderer instrumentation that allocates RunenUI identities, changes queue/publication state, stores semantic/widget facts, or acts as a second runtime trace/mutation API | Renderer observation records and repository authority audit | M7A | blocked | Required |

## M7B — standalone winit host and native input/scale/redraw path

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| HOST-01 | A real winit 0.30 standalone reference application creates and owns the native window/event loop while `runenui_core`/`runenui_runtime` remain free of winit/native window/event/physical geometry types. Native events are translated into existing neutral RunenUI values rather than stored or routed as native objects. | Real-window smoke plus dependency/public API audit and translation unit corpus | winit/raw window types in core/runtime behavior, direct widget callbacks from native events, private runtime host bridge, or native event object stored in scene/runtime state proof | Host translation diagnostics plus repository dependency/source audit | M7B | blocked | Required |
| HOST-02 | Runtime wake/redraw integrates through existing public contracts: winit event-loop proxy/user-event implements `WakeTransport`; the host thread explicitly pumps; `RedrawRequest` becomes native redraw intent; successful redraw publishes, renders/presents and only then acknowledges the consumed redraw under the accepted runtime contract. The host owns orchestration but no second UI work queue/scheduler. | Wake-from-runtime, coalescing, action/task/timer wake, redraw, failed-render/no-ack and successful-ack real-host corpus | Busy hidden framework loop, direct runtime pumping from wake callback thread, duplicate UI queue, premature redraw acknowledgement, reentrant widget dispatch, or lost wake proof | Inherited wake/redraw trace plus host event/pump/present observation | M7B | blocked | Required |
| HOST-03 | Native physical extent and scale-factor changes are converted into validated logical surface size and public `RasterScale`; renderer targets are recreated/resized as needed while RunenUI scene geometry remains logical. Scale/resize causes the required new paint publication/full-surface damage under inherited M6 semantics and never grants the host direct paint-revision authority. | Real resize/scale-factor corpus plus publication metadata/render-target comparison | Native DPI type leak, widget/renderer scale setter in neutral kernel, physical-coordinate scene mutation, stale target size, host-forged paint revision/damage or scale ignored by publication proof | Host size/scale transition records plus paint publication/renderer diagnostics | M7B | blocked | Required |
| HOST-04 | Point-based native ingress uses one edge-owned displayed-surface record containing the successfully presented publication's exact `SurfaceInputContext`, native physical extent and scale. Native points are admitted only while that mapping matches the current displayed/native coordinate state. Resize/scale transitions withhold point ingress until a new matching publication is presented; no event is attached to a stale context or retargeted through current runtime geometry. | Pointer move/down/up/boundary corpus before, during and after resize/scale transition against exact displayed context | Current-context substitution, old-context/new-scale mix, current-tree retarget, forged input context, queued stale physical click replay, or hidden layout lookup proof | Host displayed-mapping diagnostics plus inherited `SURFACE-*` rejection/route trace | M7B | blocked | Required |
| HOST-05 | Native keyboard, modifiers, physical codes, logical meaning, repeat/location, committed text, composition/preedit/end/cancel and host focus transitions map loss-preservingly into existing RunenUI ingress. Unknown key facts use owned neutral code/name forms where available. IME text is not double-delivered as key meaning plus committed text, and focus/composition cleanup preserves inherited M4 behavior. | Real/translation keyboard-text-IME corpus including unknown keys, repeat, focus loss and composition lifecycle | Native key object leakage, character-key-as-committed-text duplication, dropped unknown code where neutral owned form exists, composition generation forgery, direct focused-widget call, or platform-specific bypass proof | Host translation diagnostics plus inherited keyboard/text/composition/focus trace | M7B | blocked | Required |
| HOST-06 | The reference application presents actual `runenui_render_wgpu` output through a wgpu surface attached to the winit window and demonstrates create/show/redraw/resize/close lifecycle sufficient for M7. Recoverable surface outdated/lost/resized cases rebuild renderer target state from current public publications without moving wgpu surface/device authority into runtime. Complete cross-platform/device-loss policy remains M13. | Real native-window render/present smoke with resize and bounded recoverable surface recreation | Debug/software frame substituted for GPU window output, wgpu handle in runtime, hidden prior RunenUI scene needed after surface recreation, host-specific scene path, or claim of complete M13 recovery/platform breadth proof | Window-surface configuration/present/recreate diagnostics plus renderer update observations | M7B | blocked | Required |

## M7C — AccessKit semantic adapter path

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| A11Y-01 | The reference AccessKit adapter consumes ordinary `SemanticPublication`, keeps an adapter-owned stable mapping from live RunenUI `SemanticNodeId` to AccessKit `NodeId`, applies an exact consecutive semantic delta only when its realized surface/revision matches, and otherwise rebuilds from the complete semantic snapshot. Adapter IDs never replace or leak as RunenUI semantic identity and retired adapter IDs are not reused in one adapter lifetime. | First snapshot, consecutive update, skipped revision/full-resync, node add/remove/recreate and adapter-reset corpus | Mounted-ID aliasing, pointer/address-derived AccessKit ID, recycled live-lifetime mapping, hidden mounted lookup, fabricated semantic revision, or AccessKit tree becoming runtime semantic authority proof | Adapter surface/revision/update-mode/ID-lifetime diagnostics | M7C | blocked | Required |
| A11Y-02 | All currently accepted RunenUI semantic roles (`Generic`, `Group`, `Text`, `Button`) and currently published name/description/value/state/actions/relationships/bounds/tree/focus/plain-text facts map truthfully to AccessKit equivalents or produce an explicit unsupported-fact diagnostic; hidden nodes remain structurally absent and adapter mapping never consults paint/widget type. | Semantic fixture corpus comparing RunenUI snapshot facts to AccessKit tree/update facts, including relationships and focus | Role-by-widget matching, paint-derived accessibility, silent fact loss/reinterpretation, mounted routing identity in AccessKit nodes, disabled/inert conflation that changes RunenUI behavior, or invented editable-text semantics proof | Semantic-to-AccessKit mapping diagnostics plus tree/update snapshots | M7C | blocked | Required |
| A11Y-03 | Current RunenUI semantic actions round-trip through truthful AccessKit actions: `Activate` uses Click, `RequestFocus` uses Focus, `OpenContextMenu` uses ShowContextMenu, and `OpenMenu` uses an adapter-owned custom action unless a later accepted neutral semantic fact justifies a more specific native action. Unsupported native actions are diagnosed/rejected rather than mapped to unrelated widget behavior. | AccessKit action request corpus for each current RunenUI semantic action and unsupported action | Direct widget callback, mounted target lookup exposed to AccessKit, `OpenMenu` silently conflated with Activate/Expand, unsupported action routed anyway, or private semantic ingress proof | Adapter action translation record plus canonical RunenUI semantic-action trace | M7C | blocked | Required |
| A11Y-04 | AccessKit activation/action callbacks never mutate `AppRuntime` reentrantly or from an arbitrary adapter thread. They resolve only adapter-owned IDs, enqueue a host event/message, and the host thread submits an ordinary `SemanticActionRequest`; resulting work follows canonical queue/pump/default/trace semantics. Accessibility activation/deactivation does not create a second semantic publication lifecycle. | Callback-thread/non-reentrant action round-trip plus activation/update/pump ordering corpus | Runtime lock/mutation inside AccessKit callback, callback-owned UI queue, direct mounted command, parallel semantic snapshot, or semantic update committed independently from RunenUI publication proof | Host callback/event diagnostics plus canonical semantic ingress/work trace | M7C | blocked | Required |

## M7D — external-host proof and milestone closure

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| EMBED-01 | A genuine downstream host fixture with no winit or AccessKit dependency owns the frame loop explicitly and decides when to submit neutral work/input, pump, consume redraw intent, publish, invoke the reusable wgpu renderer on an offscreen/host target, consume/present the result and acknowledge redraw. No helper hides an infinite framework-owned loop or takes host scheduling authority. | Winit-free external-host package dependency audit plus deterministic multi-frame state/update/render loop proof | Transitive winit requirement from renderer, hidden `run()` loop, renderer-driven `AppRuntime::pump`, callback-owned application scheduler, private runtime access or direct widget dispatch proof | External host frame-step records correlated with runtime wake/redraw/publication and renderer observations | M7D | blocked | Required |
| EMBED-02 | The standalone winit path and external-host path consume the same accepted runtime/publication contracts, the same `runenui_render_wgpu` implementation and the same renderer-edge resource identity rules. Integrated M4/M5/M6/M7 validation remains green; current status/architecture describe only the real M7 reference spine and do not claim M8/M13 breadth, mandatory wgpu, or framework ownership of external host loops. | Cross-consumer renderer/publication comparison, full configured conformance audit and current-truth documentation review | Second renderer scene/protocol for embedded hosts, host-specific widget semantics, duplicate resource identity, native types in neutral kernel, premature cross-platform/text/control claims, or stale M6-only status proof | Repository validation authority plus final M7 closure record and cross-host observation comparison | M7D | blocked | Required |

## M7 closure rule

M7 closes only when all 20 rows are `owner-accepted`, inherited M4/M5/M6
validation remains green, real wgpu pixel/resource proof and a real winit host are
accepted, the AccessKit adapter round-trips ordinary semantic publication/actions,
the winit-free external host owns its loop explicitly, and final current-contract
reconciliation is accepted and validated on `main`.
