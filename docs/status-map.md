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
| Authoring and composition | `partial` | Immutable `Element<Action>` trees; builders; `element!`; text, button, row, and column descriptors | Closed `ElementKind`; no component action mapping or external widget protocol; macro and tuple composition do not scale | M1–M2 |
| Application model | `usable` | Application-owned state/actions; `UiApp`; explicit synchronous `update`; deterministic dispatch | No action queue, effects, tasks, subscriptions, cancellation, or reentrancy contract | M4 |
| Runtime identity and lifecycle | `absent` | Per-build preorder `RuntimeNodeId` and borrowed `RuntimeTreeIndex` proof | No persistent mounted tree, reconciliation, generational IDs, lifecycle, local state, or invalidation; keys are unused | M3 |
| Events and interaction | `proof` | Typed pointer/keyboard vocabulary; hit targeting; traversal focus; press activation; overlapping input-intent helpers | No routing phases, pointer identity/capture, release-based activation, focus scopes, text/IME separation, or stale-target protection | M4 |
| Effects and scheduling | `absent` | Synchronous dispatch only | No effects, queue, task executor, timers, subscriptions, cancellation, wakeups, or shutdown model | M4 |
| Styling | `partial` | Colors, padding, radius, typed tokens, computed style, provenance, missing-token diagnostics | No themes, recipes, variants, interaction states, typography, borders, fallback, or preferences | M7–M8 |
| Layout and measurement | `proof` | Explicit normalized constraints; borrowed measurement provider; computed padding; one measurement per node; row/column arrangement; overflow diagnostics | Narrow algorithm; no sizing vocabulary, alignment, flex/grid, wrapping, overlays, clipping, scrolling, baselines, or incremental layout | M7–M8 |
| Semantic/accessibility data | `absent` | Focusability is inferred from built-in element kind | No semantic tree, stable semantic IDs, roles/states/actions, AccessKit adapter, or accessibility tests | M5, M10 |
| Surface publication | `proof` | Aligned `SurfaceFrame`, `SurfaceStyleReport`, and `SurfaceLayoutReport` products | Products use transient IDs and public constructors; no surface generation or multi-surface lifecycle | M1, M3, M6, M10 |
| Hit testing | `proof` | Reverse-order rectangle hit testing over frame bounds | No explicit hit scene, stacking contract, clips, transforms, visibility/inertness, pointer policy, or stable generation | M6 |
| Rendering | `absent` | Deterministic debug text consumer of `SurfaceFrame` | `SurfaceFrame` carries semantic control kinds; no paint primitives, resources, conventional backend, or SDF consumer | M6, M10, M12 |
| Text | `proof` | Provider seam and deterministic Unicode-scalar-count measurement for headless proofs | No production provider, font database, shaping, fallback, bidi, wrapping, baselines, editing, selection, clipboard, or IME | M8 |
| Controls | `proof` | Text and button primitive proofs; enabled state and typed activation | No mounted behavior, complete semantics, keyboard/accessibility contract, style states, or standard control library | M9 |
| Host/platform integration | `absent` | Host-neutral input and measurement vocabulary only | No host contract, native event loop/window, DPI, clipboard, cursor, IME, drag/drop, accessibility bridge, or multi-window support | M10 |
| Testing and diagnostics | `partial` | Substantial proof-level integration tests; deterministic output; style/layout reports; strict lints | No public harness, semantics queries, deterministic clock/tasks, replay, snapshots, fuzzing, property tests, benchmarks, or platform tests | M5, M11 |
| Trace and observability | `proof` | Coarse mount/action/update/rebuild records and debug rendering | Duplicated unbounded storage; no sequence/generation/effect/layout/paint events, sink, export, redaction, or replay | M4–M5 |
| Source formats and devtools | `deferred` | Context-export tooling only; no UI source system | No parser, source mapping, inspector, hot reload, live preview, or visual authoring | M12 |
| Advanced editor/game systems | `deferred` | Product direction only | No virtualization, advanced data controls, animation, overlays, docking, workspaces, or advanced multi-surface systems | M12 |
| Legacy archive | `archived` | Recoverable repository history and a planned archival tag | Historical tree currently pollutes active checkout and full-audit context until M0B removal | M0B |

## Current milestone

M0 is active. M0 establishes truthful authority, removes obsolete and legacy material from the active branch, and creates the package, licensing, governance, toolchain, and validation baseline. No M1 implementation belongs in M0.
