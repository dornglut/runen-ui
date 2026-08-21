# `runenui_runtime`

> **Category: Current contract**

`runenui_runtime` owns RunenUI's live deterministic mounted execution, scheduling, routing, tracing, and surface publication for the current headless framework foundation. It consumes public `runenui_core` protocol values and remains independent from `runenui_testing`, native platform implementations, and concrete renderer backends.

## Current ownership

Runtime owns:

- one persistent generational mounted tree with keyed reconciliation, widget state/lifecycle, interaction slots, focus, invalidation, and checked mounted targeting;
- one separate generational semantic arena plus exact mounted-owner/semantic-key bindings;
- one generalized sequenced FIFO and explicit bounded pump for application actions, routed commands/input, tasks, timers, subscriptions, host responses, and derived work;
- deterministic monotonic time, producer generations, cancellation/replacement, configured backpressure, wake/redraw authority, terminal/shutdown behavior, and one bounded canonical trace with deterministic export and inert replay;
- routed pointer, focus, keyboard, committed-text, composition, automation, and exact displayed-surface input behavior;
- proof-level measurement/layout and one fallible staged surface-publication transaction;
- an independent semantic publication sibling with stable semantic identities, deterministic tree order, bounds, relationships, composed state/support, runtime-derived focus, revisions/updates, and typed diagnostics;
- public exact surface-scoped semantic action admission/resolution with private semantic-to-mounted owner/key resolution and convergence onto the existing command FIFO/routed/default/update/trace architecture.

## Identity and authority boundaries

Mounted and semantic IDs are distinct opaque runtime-issued identities under one runtime namespace. Mounted IDs address mounted widget lifetimes. Semantic IDs use a separate semantic arena and are retained by exact mounted-owner lifetime plus stable owner-local semantic key. Removing a semantic key revokes that semantic lifetime; replacing/removing its mounted owner revokes all owned semantic lifetimes. Later slot reuse advances generation and never retargets stale IDs.

Public semantic snapshots expose semantic identities only. They do not expose or reconstruct the private mounted owner. Semantic action target metadata is read-only origin context, not a mounted routing capability. There is no public semantic-to-mounted shortcut, bare semantic-ID surface guessing, second semantic queue/default engine, or semantic scrolling compatibility path.

## Publication

`AppRuntime::publish_surface` is the sole live surface-publication authority. A successful publication contains aligned renderer-facing proof products plus the independent semantic publication and semantic diagnostics. Renderer-facing proof products do not carry production semantic authority.

Publication follows the accepted staged boundary:

```text
admit -> read-only/staged plan -> candidate-dependent final preflight -> commit
```

Recoverable refusal exposes no partial new publication and preserves the previous coherent products/identities/revisions. Checked counter, work/trace-sequence, and integrity exhaustion never wrap or saturate into false success.

The retained renderer-side publication cache is still proof-level. Its production replacement is owned by the renderer-neutral scene milestone and must preserve this staged atomicity and semantic-product separation.

## Scheduling, routing, and tracing

The runtime has one canonical queue and work/trace lineage. Accepted semantic actions, pointer/keyboard/automation/programmatic commands, application actions, and scheduler outputs converge through existing sequencing rather than source-specific behavior engines. Submission rejection is fail-closed and returns owned input where the public API promises recovery. Accepted work is revalidated at processing boundaries and never retargets stale identities.

`AppRuntime::pump` is caller-bounded. Runtime never hides an unbounded settle loop or wall-clock sleep. Public deterministic testing composes these same APIs but owns no live runtime state or private mutation seam.

## Must not own

`runenui_runtime` must not own application-domain policy/state, testing convenience authority, native window/event-loop or accessibility adapters, concrete renderer backends, production controls/text, ECS assumptions, or compatibility aliases preserving retired prototype APIs.

See the [public API contract](../../docs/architecture/public-api.md), [workspace structure](../../docs/architecture/workspace-structure.md), [M5 semantic/testing charter](../../docs/conformance/m5-semantics-and-testing-charter.md), [current status](../../docs/status.md), [testing guide](../../TESTING.md), and [roadmap](../../docs/roadmap.md).
