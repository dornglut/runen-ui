# RunenUI Status

RunenUI is pre-1.0. This document summarizes **accepted current capability maturity**; it does not track branches, pull requests, CI runs, blockers, or next actions.

## Maturity vocabulary

| State | Meaning |
|---|---|
| `absent` | No accepted implementation exists. |
| `planned` | Accepted target; implementation is not current behavior. |
| `proof` | Narrow deterministic behavior exists and is tested. |
| `partial` | Real implementation exists but major production behavior is missing. |
| `usable` | Suitable for current internal/headless uses within documented limits. |
| `stable` | Public compatibility and production support are intentionally guaranteed. |
| `deferred` | Deliberately outside the current production foundation. |
| `archived` | Historical material only. |

No RunenUI subsystem is currently `stable`.

## Current capability map

| Area | Maturity | Accepted capability | Decisive limitation / next owner |
|---|---|---|---|
| Authoring and composition | `usable` | typed transient views/elements, builders, component action mapping, arbitrary child counts, downstream widgets | production control breadth remains later |
| Application model | `proof` | application-owned state/actions, explicit update, queued effects and subscriptions | one mounted application root; no native host |
| Mounted identity and reconciliation | `usable` | generational mounted identity, keyed reconciliation, lifecycle, state retention, stale/foreign rejection | runtime-local/process-local; one logical surface |
| Events and interaction | `proof` | canonical queued semantic commands, pointer, focus, keyboard, committed text, composition, automation, capture/target/bubble | host translation and production scrolling/editing absent |
| Focus and navigation | `proof` | scopes, traversal, directional navigation, restoration, modality | cross-surface focus is later host work |
| Effects and scheduling | `proof` | one sequenced FIFO, bounded pump, tasks/timers/subscriptions/host requests, cancellation, wake/redraw | runtime supplies seams rather than a default platform executor |
| Trace and replay | `partial` | bounded canonical trace, deterministic JSONL projection, redaction, subordinate sink, inert replay | diagnostic/headless foundation, not a production observability service |
| Styling | `partial` | typed values/tokens, computed style, provenance, diagnostics | themes/recipes/state styling and production breadth belong to M7 |
| Layout and measurement | `proof` | explicit constraints, measurement-provider seam, deterministic row/column proof, aligned products | no production flex/grid/scroll/incremental layout; M7 |
| Semantic contribution and identity | `partial` | owner-local semantic forests, independent runtime semantic IDs, strict validation | native platform accessibility adapter absent |
| Semantic publication and actions | `proof` | independent surface-scoped semantic snapshot/update/diagnostics plus exact semantic action ingress | one logical surface; native adapter later |
| Public deterministic testing | `partial` | downstream `runenui_testing`, deterministic time, bounded settle, public interaction and semantic queries, read-only observation | no production scene/platform test matrix yet |
| Surface publication | `proof` | staged atomic publication with renderer-facing proof products and independent semantics | retained narrow publications still use proof-level cache representation; M6 owns production scene substrate |
| Hit testing | `proof` | deterministic rectangle proof targeting with displayed-generation safety | no production `HitTestScene`, clips/transforms/stacking; M6 |
| Renderer-neutral paint/hit scene | `absent` | accepted target contracts exist | no current production scene API; M6 |
| Production renderer backend | `absent` | none | requires accepted neutral scene first; M10 |
| Deterministic text measurement | `proof` | provider seam and deterministic headless metrics | not shaping, bidi, wrapping, font fallback, or editing |
| Production text subsystem | `absent` | none | M8 |
| Built-in button | `proof` | canonical activation convergence and semantic participation | production styling/control breadth/native adapters later |
| Standard control library | `absent` | proof controls only | M9 |
| Host neutrality | `usable` | core/runtime/testing contain no native window, GPU, ECS, or accessibility-adapter dependency | neutrality alone is not a host implementation |
| Native host/platform integration | `absent` | host-neutral seams only | event loop, DPI, clipboard, cursor, IME, accessibility, multi-window: M10 |
| Raw controller/gamepad translation | `absent` | normalized controller-origin commands only | device lifecycle/axes/dead-zone/platform mapping: M10 |
| Source formats and advanced devtools | `deferred` | no UI source language or visual tooling | M12 |

## Accepted foundation

The repository has completed the foundational sequence through renderer-independent semantics and deterministic public testing (M0–M5). The accepted M6 architecture/conformance contract defines the next renderer-neutral scene work, but target contracts do not imply implemented scene behavior. Conformance details and accepted default-branch row state are authoritative under [conformance](conformance/README.md).

See the [roadmap](roadmap.md) for durable sequencing and [architecture](architecture/README.md) for current ownership. Live execution state belongs in GitHub.
