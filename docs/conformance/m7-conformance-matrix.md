# M7 Conformance Matrix

> **Category:** Target architecture
>
> **Status:** M7A owner-accepted on accepted main; remaining M7 rows remain blocked
>
> **Milestone:** M7
>
> **Reviewed baseline:** `42df29bc68cfec97c13f80f0f59c209db512152c`
>
> M7A is normative for its eight owner-accepted rows after exact-head owner
> acceptance, guarded merge, and accepted-main validation. M7B/C/D remain
> blocked until their own accepted implementation and proof obligations land.

[ADR 0008](../adr/0008-reference-production-spine.md) owns M7 architecture.
M4 owns neutral input and wake/redraw; M5 owns semantic publication/action;
M6 owns paint/hit publication, revision/damage, and `ResourceRef` identity.

```text
20 total unique rows
8 owner-accepted
0 implementation-complete
0 proof-complete
12 blocked
0 duplicate IDs
0 invalid statuses
0 invalid schemas
```

All rows are `Required`. Delivery slices: M7A renderer/resources, M7B winit
host/input, M7C AccessKit, M7D external-host/closure. Private bridges, forged
identity, widget-kind rendering, core/runtime resource registries, second expected
renderers, hidden framework loops, or native/backend types in neutral authority
cannot satisfy M7.

## M7A — renderer and resources

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| RENDER-01 | `runenui_render_wgpu` consumes ordinary paint publication without winit/AccessKit, widget/semantic/mounted/layout/private-runtime knowledge; wgpu state stays renderer-owned. | Dependency audit and winit-free consumer | Forbidden dependency/source audit | Renderer construction diagnostics | M7A | owner-accepted | Required |
| RENDER-02 | Real wgpu pixels preserve all current M6 primitive, order, transform, clip, opacity, color/blend, placement and scale semantics. | Offscreen/window render corpus | Semantic/widget/layout reinterpretation and silent fallback corpus | Frame and pixel/probe records | M7A | owner-accepted | Required |
| RENDER-03 | Realized surface/revision classification is exact; mismatched base never uses damage; complete publication reconstructs state after renderer/cache loss. | Revision/reset corpus | Mismatched-base and hidden-history corpus | Update-mode/revision/base/damage records | M7A | owner-accepted | Required |
| RESOURCE-01 | Caller-owned lookup uses complete `ResourceRef`; logical binding stays immutable; renderer caches are disposable; resource failures are deterministic. | Provider/cache-loss corpus | Registry/split-key/rebinding/fallback audit | Resource realization diagnostics | M7A | owner-accepted | Required |
| RESOURCE-02 | Image payload is explicit non-zero unpremultiplied RGBA8 sRGB; complete image maps exactly to M6 destination; PNG decoding is provider-owned. | PNG/image corpus | Decoder-fit/color-policy and runtime-decode audit | Decode/upload diagnostics and pixels | M7A | owner-accepted | Required |
| RESOURCE-03 | Shaped-run payload supplies resource-local coverage at requested scale; foreground stays scene-owned; same ref may re-realize at another scale; M7 fixture performs no production shaping/layout. | Bundled-font/scale corpus | Shaping/fallback/layout and identity-conflation audit | Raster/cache diagnostics and pixels | M7A | owner-accepted | Required |
| GOLDEN-01 | Actual wgpu offscreen output is read back and compared under a documented tight policy using the same render/resource path as window output; no software/noop expected renderer. | Golden/readback corpus | Alternate expected-renderer and overbroad-tolerance audit | Comparator/backend diagnostics | M7A | owner-accepted | Required |
| OBS-01 | Renderer observation exposes publication/update/damage/extent/scale/resource/render/readback/present facts without runtime mutation or second trace authority. | Observation correlation tests | Mutation/identity/semantic capture audit | Renderer observation records | M7A | owner-accepted | Required |

## M7B — winit host and native input

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| HOST-01 | Real winit host owns window/event loop and translates native events to existing neutral values; native types do not enter core/runtime. | Real-window and translation corpus | Native-type/private-bridge/direct-widget audit | Host translation diagnostics | M7B | blocked | Required |
| HOST-02 | Wake uses `WakeTransport`; host pumps explicitly. A taken redraw is acknowledged only after successful `publish_surface`. Render/present then uses the retained publication; renderer failure retries at the edge and does not defer that acknowledgement or force unchanged republish. | Wake/redraw/publication/render-failure corpus | Premature ack, GPU-gated ack, unchanged-republish recovery, hidden-loop audit | Runtime redraw trace plus host stage records | M7B | blocked | Required |
| HOST-03 | Native size/scale becomes validated logical size and `RasterScale`; renderer target changes while scene geometry remains logical and revision/damage remain runtime-issued. | Resize/scale corpus | DPI leak, physical scene mutation, forged revision/damage audit | Size/scale/publication records | M7B | blocked | Required |
| HOST-04 | Point input uses only the successfully presented publication's exact input context with matching native extent/scale; mismatch withholds input until a matching frame is presented. | Before/during/after transition corpus | Context substitution, stale/new mapping mix, unpublished-frame target audit | Displayed-mapping and inherited surface trace | M7B | blocked | Required |
| HOST-05 | Keyboard, physical/logical key, modifiers, repeat/location, committed text, IME/composition and focus translate loss-preservingly without double delivery or native bypass. | Keyboard/text/IME corpus | Native-object leak, dropped representable key, forged composition, direct-widget audit | Host and inherited input trace | M7B | blocked | Required |
| HOST-06 | Real wgpu output is presented in the winit window with bounded create/show/redraw/resize/close and recoverable target recreation; wgpu authority remains edge-owned. | Native window/present smoke | Debug-frame substitution, runtime wgpu state, hidden scene path audit | Surface/present/recreate records | M7B | blocked | Required |

## M7C — AccessKit

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| A11Y-01 | Adapter consumes ordinary semantic publication, keeps stable adapter-owned IDs, uses exact consecutive delta only from matching surface/revision, otherwise full-resyncs. | Snapshot/delta/reset corpus | Mounted-ID/reuse/private-lookup/fabricated-revision audit | Adapter revision/ID records | M7C | blocked | Required |
| A11Y-02 | Roles map exactly: Generic to GenericContainer, Group to Group, Text to Label, Button to Button; matching properties/relations map truthfully and unsupported facts diagnose rather than reinterpret. A Text/Label node exposes plain semantic text as AccessKit `value`; the same content is not duplicated into AccessKit `label` merely because RunenUI also publishes it as the node name. | Semantic-to-AccessKit comparison corpus including built-in Text duplicate-name/text case | Widget/paint-derived role, duplicate static-text announcement, and silent reinterpretation audit | Mapping diagnostics/tree snapshots | M7C | blocked | Required |
| A11Y-03 | Actions map Activate to Click, RequestFocus to Focus, OpenContextMenu to ShowContextMenu, OpenMenu to the published adapter-owned CustomAction; only its matching custom-action data is accepted. | Current-action and unsupported-action corpus | OpenMenu conflation, wrong custom ID, direct-widget/private-ingress audit | Action translation and canonical semantic trace | M7C | blocked | Required |
| A11Y-04 | Adapter is installed before first window show; mixed activation reads only immutable adapter-derived tree state; action/deactivation use event-loop proxy; `process_event` precedes app handling; runtime mutation happens only on host thread through ordinary semantic ingress. | Activation/callback/ordering corpus | Off-thread/reentrant runtime, parallel semantic authority, wrong install/event order audit | Host/adapter callback and work trace | M7C | blocked | Required |

## M7D — external host and closure

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| EMBED-01 | Winit-free downstream host visibly owns submit/pump/redraw/publish/ack/render/present sequencing; ack follows publication success and renderer retry uses retained publication under host policy. | Winit-free multi-frame host corpus | Hidden loop, renderer-driven pump, premature ack, unchanged-republish recovery, private-runtime audit | External-host frame-step records | M7D | blocked | Required |
| EMBED-02 | Standalone and external hosts share accepted runtime/publication contracts, `runenui_render_wgpu`, and complete-`ResourceRef` provider rules; integrated M4/M5/M6/M7 proof and truthful closure docs remain green. | Cross-host corpus and canonical validation | Host-specific scene/renderer/resource identity or later-capability overclaim audit | Integrated CI/evidence and authority audit | M7D | blocked | Required |

## Closure rule

M7 closes only after all 20 rows are `owner-accepted` on accepted default branch
and final closure reconciliation is accepted-main validated. M8 does not begin
from an unmerged M7 branch.
