# Events, Effects, and Scheduling Architecture

> **Category: Current architecture**

This document summarizes the current durable ownership of routed interaction, application work, scheduling, focus/input state, and trace. Exact behavioral requirements remain in [ADR 0005](../adr/0005-canonical-event-routing-and-commands.md), [ADR 0006](../adr/0006-effects-scheduling-and-trace-v2.md), and the [M4 conformance matrix](../conformance/m4-conformance-matrix.md).

## One runtime processing authority

`runenui_runtime` owns one generalized sequenced FIFO and an explicit bounded pump. Accepted application actions, routed semantic commands, pointer/keyboard/text/composition input, automation, semantic actions after admission, task/timer/subscription/host completions, and derived runtime work converge through that processing architecture instead of source-specific dispatch loops.

Callbacks do not synchronously recurse into application update or reconciliation. Routed transactions collect provisional mutation and output, preflight bounded authority before mutable execution where required, and commit accepted output in deterministic order. Later queued work revalidates exact runtime identity rather than retargeting stale owners.

## Runtime-issued identity and targeting

Mounted, semantic, surface, pointer/composition, work, and trace identities are runtime-issued and non-forgeable for ordinary downstream code. Mounted and semantic identities are distinct lifetimes. Surface input carries exact displayed-generation context so retained historical input is interpreted against the publication that produced it rather than current geometry.

Foreign, stale, missing, retired, or mismatched authority rejects explicitly. Terminal pointer cleanup is allowed to repair locally owned pointer integrity where the accepted event contract requires it, but foreign runtime/surface input cannot mutate local pointer state.

## Routed interaction

The routed event path snapshots one live mounted route and uses Capture, Target, and Bubble according to the event family. Propagation control and default prevention are independent. Widget callbacks may emit typed actions and commands, request invalidation, request focus/capture changes, and request bounded owner-local work through the accepted context contracts; they do not gain direct runtime storage or application-state authority.

Device-specific input is normalized at the host boundary. Current host-neutral families include pointer, keyboard, committed text, IME composition, and semantic commands. Committed text is not inferred from keyboard characters, and composition has an exact owner lifetime that is cancelled rather than retargeted across focus/lifecycle changes.

Primary pointer activation is release-inside behavior. Keyboard, automation, programmatic, normalized-controller, and semantic accessibility origins converge on the canonical command/default path where their accepted behavior maps exactly. Route-only commands do not invent application behavior when no widget consumes them.

## Focus and interaction state

Runtime owns the exact mounted focus lifetime, nested focus scopes, focus-within projection, restoration memories, retained modality, pointer pressed/capture state, composition ownership, and stationary-pointer physical-path state.

Focus transitions are committed atomically and derive deterministic routed notifications. Directional navigation uses current retained layout geometry under the accepted public-outcome corpus; the scoring implementation is private policy, not public API.

Interaction cleanup is lifecycle-aware. Removal, replacement, disablement, cancellation, terminal transition, and shutdown revoke incompatible exact-generation state before later work can address it.

## Application work and effects

`runenui_core` owns host-neutral application/effect/subscription description protocols. `runenui_runtime` owns their live execution state, generation authority, queueing, cancellation, and completion processing.

The current scheduler supports ordered initial/update effects, application and mounted subscriptions, local and send-capable tasks, monotonic timers, typed host requests/responses, explicit logical/manual time, wake/redraw signaling, configured bounded capacities, and lifecycle-owned cancellation.

Work admission and cancellation are generation-safe. Rejected work returns owned input where the public contract promises recovery. Accepted work is never silently dropped merely because later ownership becomes stale; it is processed or rejected through the canonical runtime/trace semantics.

`AppRuntime::pump` is explicitly caller-bounded. Readiness, completion imports, local polling, and due-timer promotion have separate budgets. The runtime does not hide an unbounded settle loop or wall-clock sleep.

## Wake and redraw

Wake and redraw are separate authorities. Wake requests are synchronized so accepted work cannot be stranded by a lost notification, and host callbacks run outside RunenUI synchronization guards. Redraw uses revisioned request/acknowledgement semantics so acknowledgement cannot erase newer dirtiness.

Terminal and closed runtime states prevent new mutable work from being accepted. Shutdown is explicit and idempotent and shares cleanup authority with state extraction/drop.

## Trace and replay

Runtime owns one bounded canonical trace. Trace records retain deterministic runtime facts and causal relationships without requiring application actions to implement `Debug`. Text and composition payload capture is redacted by default and requires explicit opt-in for exact payload retention.

The deterministic JSONL projection and optional bounded external sink are subordinate views of the canonical trace, not second ordering/history authorities. Offline replay consumes serialized trace observation only; it cannot create live runtime identities, invoke callbacks, or mutate an application.

## Semantic action convergence

Semantic contribution and publication are independent from renderer products. Exact surface-scoped semantic action admission validates current semantic authority, then accepted work converges on the same command FIFO/routed/default/update/trace architecture. The semantic-to-mounted owner/key binding remains private runtime authority.

Semantic action processing revalidates the exact accepted semantic lifetime at queue-front and before applicable defaults. It never guesses another surface, owner, or replacement target.

## Boundaries

This architecture does not make RunenUI a native event loop, platform input translator, default executor service, production scrolling implementation, editable-text system, or multi-window host. Those capabilities require their owning later contracts.

Current maturity is summarized in [status](../status.md). Durable future sequencing belongs in the [roadmap](../roadmap.md). Permanent M4 observable/proof obligations remain under [conformance](../conformance/README.md).
