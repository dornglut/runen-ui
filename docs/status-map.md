# RunenUI Status Map

> **Category: Current contract**

This map reports the maturity of the active implementation at the current revision. It does not treat types, target documents, or historical code as implemented behavior.

## Maturity states

| State | Meaning |
|---|---|
| `absent` | No accepted implementation exists. |
| `planned` | Accepted roadmap target; implementation has not started. |
| `proof` | Narrow deterministic behavior exists and is tested. |
| `partial` | Real implementation exists with major production behavior missing. |
| `usable` | Suitable for current internal examples within documented limits. |
| `stable` | Public compatibility and production support are intentionally guaranteed. |
| `deferred` | Deliberately outside the current production foundation or first release. |
| `archived` | Historical material only; not active authority or implementation. |

No framework subsystem is currently `stable`.

## Subsystem status

| Area | Current maturity | What exists | Decisive limitation | Target milestone |
|---|---|---|---|---|
| Authoring and composition | `usable` | Separate built-in views/private widgets; downstream leaves; canonical child-layout `Container`; recursive mapping; arity-free composition | Descriptions are transient reconciliation inputs, not mounted identity/state | M2–M3 complete |
| Application model | `usable` | Application-owned state/actions; `UiApp`; explicit synchronous `update`; deterministic dispatch | No action queue, initial/update effects, state-derived subscriptions, tasks, keyed cancellation, or reentrancy contract | M4 |
| Mounted runtime identity and indexing | `usable` | Runtime-local generational `MountedNodeId`, distinct `SemanticNodeId`, logical-preorder `MountedTreeIndex`, stale/foreign rejection, authored-ID lookup diagnostics | One mounted root and runtime-local IDs remain the M3 boundary | M3 complete |
| Keyed reconciliation | `usable` | Transactional sibling-local matching, unkeyed ordinal policy, structured duplicate-key no-reuse diagnostics, cross-parent remount, exact reports | Cross-parent movement remounts; duplicate keys preserve no ambiguous lifetime | M3 complete |
| Persistent widget state and lifecycle | `usable` | State-aware capabilities, preorder mount/update, postorder removal/replacement/shutdown, state drop after unmount, interaction slots | No M4 task/subscription ownership | M3 complete; M4 queued |
| Events and interaction | `proof` | Typed pointer/keyboard vocabulary; mounted hit targets; traversal focus; explicit mutable non-`Clone` activation; immediate transient-root rebuild and mounted reconciliation after successful dispatch | Mounted targets are stale/foreign safe, but there are no routing phases, surface-input generation, pointer identity/capture, release-based activation, focus scopes, text/IME separation, or full event-level target diagnostics | M4 |
| Normalized UI navigation commands | `absent` | None; keyboard Tab/Enter/Space policies are device-specific proofs | No abstract next/previous/directional/activate/cancel/menu/context commands or modality tracking | M4 |
| Directional/spatial focus navigation | `absent` | None | Current focus movement is linear traversal only | M4 |
| Effects and scheduling | `absent` | Synchronous dispatch only | No effects, queue, initial work, task executor, timers, subscriptions, keyed cancellation, wake handshake, limits, or shutdown model | M4 |
| Styling | `partial` | Colors, padding, radius, typed tokens, computed style, provenance, missing-token diagnostics | No themes, recipes, variants, interaction states, typography, borders, fallback, or preferences | M7–M8 |
| Layout and measurement | `proof` | Separate one-query intrinsic/child-layout snapshots; component-wise minimum combination; padding/gaps; unsupported/unknown fallbacks; aligned index/frame/style/layout products | Linear M2 proof only; no production sizing/alignment/flex/grid/scroll/incremental layout | M7–M8 |
| Focusability facts | `proof` | Any widget contributes enabled/actionable facts through the open protocol; built-in and external controls pass traversal tests | Still proof-level and not the M5 semantic/focus model | M5 |
| Semantic/accessibility tree | `absent` | Widgets publish minimal deterministic role/name/enabled/action-intent proof facts and mounted lifetimes expose `SemanticNodeId` | Proof facts are not a production semantic tree and have no relationships, values, semantic actions, AccessKit adapter, or accessibility claim | M5, M10 |
| Surface publication | `proof` | Topology-only whole-surface cache, current mounted style/layout reads, exact token-content key, independent phase-entry/report proofs, and warmed structural/common-field tests | One current publication domain; proof facts are not M5/M6 products or production retained layout | M6, M10 |
| Hit testing | `proof` | Reverse-order rectangle hit testing over frame bounds returns generation-safe mounted targets | No explicit hit scene, stacking contract, clips, transforms, visibility/inertness, pointer policy, or M4 surface-input generation contract | M4, M6 |
| Debug/semantic frame consumption | `proof` | Deterministic text rendering includes open widget paint/semantic/diagnostic proof facts | Debug output is not a paint scene, semantic tree, accessibility product, or backend | M5–M6 |
| Renderer-neutral paint scene | `absent` | M2 publishes deterministic open widget paint/debug facts | Proof facts are not primitives/resources and have no clips, transforms, layers, or damage | M6 |
| Production renderer backend | `absent` | None | No conventional or SDF backend; the neutral scene must be accepted first | M10, M12 |
| Deterministic text measurement | `proof` | Provider seam and Unicode-scalar-count measurements support headless tests | Fixed metrics are not font, shaping, grapheme, bidi, wrapping, or baseline layout | M8 |
| Production text subsystem | `absent` | None | No font discovery, shaping, fallback, bidi, wrapping, editing, selection, clipboard, or IME | M8 |
| Button behavior | `proof` | Label, enabled state, typed action, press activation, and focused Enter/Space behavior are tested | No mounted pressed state, pointer capture, release-inside activation, `on_activate`, semantics, recipes, or accessibility contract | M4, M9 |
| Standard control library | `absent` | None beyond text/button proofs | No complete lifecycle/event/semantic/style/layout/keyboard/accessibility contracts | M9 |
| Host neutrality | `usable` | Active core/runtime have no native window, GPU, ECS, platform-controller, or legacy dependencies | Neutrality alone is not a host integration contract | M10 |
| Host/platform integration | `absent` | None | No host contract, native event loop/window, DPI, clipboard, cursor, IME, drag/drop, accessibility bridge, wake acknowledgment, or multi-window support | M4, M10 |
| Raw controller/gamepad platform input | `absent` | None | No device connection/identity, raw button/axis translation, normalization/dead zones, or embedded-host mapping | M10 |
| Testing and diagnostics | `partial` | Substantial proof-level integration tests; genuine downstream widget conformance; deterministic open widget/style/layout inspection; strict lints | No unified M5 public harness, M4 public conformance matrix implementation, stable semantic queries, deterministic clock/tasks, replay, snapshots, fuzzing, property tests, benchmarks, or platform tests | M4–M5, M11 |
| Trace and observability | `proof` | Coarse mount/action/update/rebuild records and debug rendering | Duplicated unbounded storage; no sequence/generation/effect/surface/saturation/wake-ack facts, sink, export, redaction, or replay | M4–M5 |
| Source formats and devtools | `deferred` | Context-export tooling only; no UI source system | No parser, source mapping, inspector, hot reload, live preview, or visual authoring | M12 |
| Advanced editor/game systems | `deferred` | Product direction only | No virtualization, advanced data controls, animation, overlays, docking, workspaces, or advanced multi-surface systems | M12 |
| Legacy archive | `archived` | Git history and annotated tag `legacy-runenwerk-ui-archive-2026-07-11` preserve the audited tree | Removed from active content; salvage remains opt-in and must follow current architecture | M0B complete |

## Current milestone

M0–M3 are complete. ADR 0005 and ADR 0006 were accepted by the repository owner
on 2026-07-14 as the M4 implementation charter. M4 remains queued until its first
implementation slice begins. The Gate 0 package closes the implementation-
defining questions, including dedicated mounted subscription declarations,
exact route-only command defaults, terminal pointer cleanup, executor refusal,
and bounded trace-sink behavior. It supplies the normative proof matrix plus the
directional-focus corpus. No M4 implementation has started.
