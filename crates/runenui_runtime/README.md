# `runenui_runtime`

> **Category: Current contract**

`runenui_runtime` owns RunenUI's live deterministic mounted execution, scheduling, routing, tracing, and surface publication for the current headless proof. It consumes public `runenui_core` protocol values and remains independent from `runenui_testing`, native window/accessibility implementations, and concrete renderer backends.

## Current ownership

Runtime owns:

- one persistent generational mounted tree with sibling-local keyed reconciliation, widget state/lifecycle, interaction slots, focus, invalidation, and checked mounted targeting;
- one separate generational semantic arena plus exact mounted-owner/`SemanticKey` bindings. `SemanticNodeId` allocation is independent from mounted arena allocation; one mounted owner may own zero, one, or many semantic lifetimes;
- one generalized sequenced FIFO and explicit four-budget pump for application actions, routed commands, tasks, timers, subscriptions, host responses, and committed work;
- deterministic manual/host monotonic time, live-only producer generations, cancellation/replacement, configured backpressure, wake/redraw authority, terminal/shutdown behavior, and one bounded canonical trace plus deterministic JSONL export and inert offline replay;
- exact mounted command routing with Capture/Target/Bubble, defaults, pointer lifecycle/capture, focus scopes/modality, raw keyboard, committed text, IME composition, authored-ID automation, and exact displayed-surface input contexts;
- one fallible staged surface-publication transaction and the retained proof-level renderer publication cache;
- an independent `SemanticPublication` sibling with stable semantic identities, deterministic tree order, absolute logical bounds, resolved relationships, composed state/support, runtime-derived visible-PRIMARY focus, revisions/deltas/full-resync behavior, and typed semantic diagnostics;
- public exact surface-scoped semantic action admission/resolution through `SemanticActionRequest`, with private semantic-to-mounted owner/key resolution and convergence onto the existing command FIFO/routed/default/update/trace architecture.

## Identity and authority boundaries

`MountedNodeId` and `SemanticNodeId` are distinct opaque runtime-issued identities under the same runtime namespace. Mounted IDs address mounted widget lifetimes. Semantic IDs use a separate semantic arena slot/generation and are retained by exact mounted-owner lifetime plus stable owner-local `SemanticKey`. Removing a semantic key revokes that semantic lifetime; replacing/removing its mounted owner revokes all owned semantic lifetimes. Later semantic slot reuse advances generation and never retargets stale IDs.

Public semantic snapshots expose semantic IDs only. They do not expose or reconstruct the private mounted owner. `SemanticActionTarget` is read-only exact semantic-origin metadata, not a mounted routing capability. There is no public semantic-to-`MountedNodeId` shortcut, bare semantic-ID surface guessing, direct semantic activation path, second semantic queue/default engine, or semantic `LogicalScroll` compatibility path.

## Publication

`AppRuntime::publish_surface` is the sole live surface-publication authority. A successful `SurfacePublication` contains aligned renderer-facing frame/style/layout/hit/paint proof products plus the independent semantic publication and semantic diagnostic report. Renderer-facing `SurfaceFrame`/`SurfaceNode`/debug products do not carry production semantic authority.

Publication follows the accepted staged boundary:

```text
admit -> read-only/staged plan -> candidate-dependent final preflight -> commit
```

Recoverable refusal exposes no partial new publication and preserves prior coherent semantic IDs/product/revision. Checked publication-counter, work/trace-sequence, or integrity exhaustion never wraps or saturates into false success.

The current retained renderer cache is proof-level and still deep-clones whole `SurfaceCache` state for some planning paths; issue #59 owns removing that cost before or during M6 without weakening accepted M5 atomicity.

## Scheduling, routing, and tracing

The runtime has one queue and one canonical work/trace lineage. Accepted semantic actions, pointer/keyboard/automation/programmatic commands, application actions, and scheduler outputs converge through existing sequencing rather than creating source-specific behavior engines. Submission rejection is fail-closed and returns owned input where the public API promises recovery. Accepted work is revalidated at processing boundaries and never retargets stale identities.

`AppRuntime::pump` is caller-bounded. Runtime never hides an unbounded settle loop or wall-clock sleep. `ManualClock` enables deterministic time; `runenui_testing` composes these public APIs but owns no live runtime state or private mutation seam.

## Must not own

`runenui_runtime` must not own application-domain policy/state, testing convenience authority, native window/event-loop or AccessKit adapters, concrete renderer backends, production controls/text, ECS assumptions, or compatibility aliases preserving retired prototype APIs.

M0–M4 are complete. M5A–M5D are accepted and reconciled; M5E #51 is the active integration/migration/closure slice. M6 implementation remains blocked until accepted M5 closure.

See the [public API contract](../../docs/architecture/public-api.md), [workspace structure](../../docs/architecture/workspace-structure.md), [M5 charter](../../docs/architecture/m5-semantics-and-testing-charter.md), [status map](../../docs/status-map.md), [testing guide](../../TESTING.md), and [roadmap](../../docs/roadmap.md).
