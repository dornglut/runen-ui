# Current Public API Contract

> **Category: Current contract**

This document records the reviewed public surface for application work, the
deterministic scheduler, activation, and canonical trace. Source-level Rust
documentation is authoritative for signatures. [ADR 0003](../adr/0003-extensible-view-widget-component-protocol.md)
defines the open authoring/widget foundation; [ADR 0004](../adr/0004-mounted-runtime-reconciliation.md)
defines mounted ownership and reconciliation; accepted
[ADR 0006](../adr/0006-effects-scheduling-and-trace-v2.md) defines application
work and scheduling; M4B is implemented, owner-accepted, and squash-merged.
Routed events and trace export/replay remain later unimplemented M4 slices. The
accepted
[M4C delivery and routed-transaction charter](m4c-delivery-and-routed-transaction-charter.md)
records target ownership and transaction decisions but does not describe
implemented public API until each slice is accepted. The
[M4 conformance matrix](m4-conformance-matrix.md) owns observable acceptance.

## Ownership and inventory

`runenui_core` owns `UiApp`, host-neutral effects/subscriptions/work protocols,
validated authored values and identity, style intent and
resolution, transient `View`/`Element` authoring, typed built-in views, the open
state-aware `Widget`/`ChildLayoutWidget` contracts, proof capability values,
lifecycle contexts, `WidgetInvalidation`, and typed recursive action mapping.

`runenui_runtime` owns `AppRuntime`, the canonical generalized FIFO and pump,
persistent mounted storage, reconciliation,
lifecycle execution, focus and interaction slots, mounted targeting,
invalidation scheduling, capability caches, measurement/layout execution,
bounded trace, live work registry, clocks, completion ingress, wake/redraw, and
mounted publication. The public runtime, mounted inspection, and integrity
vocabulary includes:

- `MountedNodeId`, `SemanticNodeId`, `MountedNodeRef`, and `MountedTreeIndex`;
- `ReconciliationGeneration` and `ReconciliationReport`;
- `RuntimeConfig`, `WorkSequence`, `SubmitActionResult`, `PumpBudget`, and
  `PumpReport`;
- `RuntimeStatus`, `RuntimeTerminalReason`, `ShutdownReport`,
  `FocusTargetResult`, `ActivationCommit`, `ActivationResult`, and `RuntimeError`;
- `TraceConfig`, `TraceSequence`, `TraceRecord`, `TraceRecordKind`,
  `TraceTarget`, and `Trace`;
- read-only frame, style-report, layout-report, and publication products.

The ordinary preludes remain narrow. Specialist runtime/mounted/lifecycle
inspection is imported explicitly from crate roots. Generated IDs and
sequences, queue/envelope storage, mounted state, arena storage, reconciliation
reports, trace records, and publication products have no public constructors.

## Queue, pump, and runtime status

`RuntimeConfig::default()` selects a waiting-envelope capacity of 4096 and a
`TraceConfig` capacity of 1024. `with_queue_capacity` and `with_trace_config`
return adjusted values; fields remain private. Queue capacity counts waiting
envelopes only, and zero is valid. Queue and trace capacities are logical limits:
internal storage grows with accepted envelopes or retained records and does not
reserve the complete configured capacity when the runtime mounts.
Default live local-task, send-task, timer, subscription, and host-request limits
are 2048 each; the default transaction-output limit is 1024.
`RuntimeLimits::with_subscription_diagnostics` independently bounds the public
diagnostic retention slice; zero disables that auxiliary retention without
changing canonical trace behavior.

`AppRuntime::submit_action(action)` appends one application action when running,
capacity is available, and the non-wrapping sequence authority can advance. It
returns a runtime-issued `WorkSequence`, beginning at 1, or a
`SubmitActionError<Action>` classified as full, closed, or terminal. Rejection
returns the exact owned action through `into_action` and does not consume a work
sequence. `Action` requires no `Clone`, `Send`, or `Debug` bound.

Application and mounted output batches use one provisional transaction planner.
It resolves owner/family/key bindings for the complete batch, preflights every
queue sequence and work generation without consuming rejected capacity, then
atomically invalidates exact cancellation/replacement targets, installs accepted
records, and appends cleanup before ordered starts/actions.
For an application update the accepted order is cancellation cleanup, mounted
subscription reconciliation, update outputs, application subscription starts,
then mounted lifecycle outputs. Activation uses cleanup, mounted subscription
reconciliation, primary action, then auxiliary outputs.

Initial application work uses the same single-plan authority. After successful
initial reconciliation, it atomically admits mounted subscription reconciliation
in mounted preorder, initial effects in collector order, application subscription
starts in declaration order, then mounted mount output in mounted preorder and
collector order. All aggregate limits are preflighted before commit; a rejected
plan consumes no partial queue sequence or work generation and starts no work.

`AppRuntime::pump(PumpBudget::new(envelopes, imports, polls, promotions))`
shares four explicit limits across every checkpoint in that call. `PumpReport`
exposes exact counters, independent exhaustion flags, readiness/deadline facts,
publication dirtiness, and a `PumpOutcome` of quiescent, budget exhausted,
closed, or terminal. One popped envelope is one processed-envelope unit. Each
accepted application action completes update, root rebuilding,
reconciliation, focus validation, reports, and mandatory trace records before
the next begins. `max_local_polls`, `polled_local_work`, and the `local_polls`
exhaustion flag cover local tasks and local subscription sources together.
Checkpoints import bounded completions, promote due timers, poll wake-eligible
local work in private generation order, and append accepted outputs at the FIFO
tail.

`SubscriptionSet::local` accepts a `LocalSubscriptionSource` whose `poll_next`
receives a safe `Context`; sleeping sources are not polled again until their
waker fires. `SubscriptionSet::send` accepts a start-once
`SendSubscriptionSource` and supplies a cloneable `SendSubscriptionSink` whose
nonblocking `start` returns `Started`, `Unavailable`, `Full`, `Closed`, or
`Rejected`. While the source remains `Starting`, `try_send` returns
`SendSubscriptionSinkError::NotStarted(exact_item)`; only `Running` accepts.
Running ingress returns the exact item on full, closed, or stale rejection.
Neither refusal path retries implicitly. The send-side item may be `Send` while
the UI mapper and resulting application action remain non-`Send`.

`AppRuntime::status()` reports `Running`, `Terminal(reason)`, or `Closed`.
Work-sequence, work-generation, reconciliation-generation, or enabled-trace-sequence exhaustion
is terminal and non-resettable: no later mutable callback runs, queued envelopes
are cancelled, new submissions return their actions, and inspection/state
extraction remain available. `shutdown()` is explicit and idempotent; its report
exposes whether shutdown was already complete plus cancelled-envelope and
unmounted-lifetime counts. `into_state` and `Drop` invoke the same shutdown
authority.

`WakeTransport::request_wake` executes only after the runtime has claimed one
eligible request epoch and released every RunenUI synchronization guard. Request
state and `callback_in_flight` are separate facts: callbacks remain serialized,
while an acknowledged new request can remain pending behind a callback already
in flight. Installing a transport claims a pending undelivered request once;
replacing a transport never redelivers an already claimed epoch. Closing wake
authority prevents new claims and returns without waiting. A callback claimed
before close may finish afterward, but its completion cannot reopen or re-arm
the closed state. Panic recovery from host adapters is not promised.

`HostProtocol::Response` is retained runtime data and therefore `'static`; it
needs `Send` only when creating a detached `HostResponseCompletion`. Creation
does not reserve the request. `submit` returns `Full`, `Closed`, or `Stale` with
the exact completion. One lock-protected state machine owns each live response
generation: registration inserts `Open`; detached acceptance changes
`Open -> DetachedQueued`; direct acceptance changes `Open -> DirectClaimed`;
full detached submission leaves `Open` for exact retry. Cancellation,
replacement, owner revocation, completion, terminal closure, and shutdown remove
the retained response slot and any queued detached payload. No `Cancelled`
tombstone is retained. A missing slot is stale authority, and every competing or
late transition is stale.

## Transient authoring and mounted authority

`View<Action>` consumes a typed authored value into one erased
`Element<Action>`. An element contains common authored ID/key, layout/style
intent, authoring diagnostics, transient children, and one safely erased widget
description. Reconciliation consumes it. `AppRuntime` does not retain an element
root, publish from an element, target an element, or treat authored/preorder
position as persistent identity.

Components are ordinary Rust composition and typed action mapping. They do not
automatically create mounted identity or state. Stable reorderable collections
must author sibling-local unique keys.

The private mounted tree is the sole runtime authority. `MountedTreeIndex`
traverses logical mounted preorder; arena slot order is never observable as tree
order. A mounted node owns parent/ordered children, authored metadata, current
widget description, persistent erased state, interaction slots, replacement
status, capability caches, and internal dirty phases.

## State-aware widget contract

Every `Widget<Action>` declares `State: 'static` and creates it. Stateless
widgets use `State = ()`. The runtime passes persistent state to:

- `mount`, `update`, and `unmount`;
- immutable activation facts, measurement, paint, semantics, and diagnostics;
- mutable activation;
- `ChildLayoutWidget::child_layout`.

`WidgetMountContext<Action>`, `WidgetUpdateContext<Action>`, and
`WidgetActivationContext<Action>` can request `WidgetInvalidation`, invalidate
owner-local subscription declarations, and collect exact-mounted-owner actions,
local/send tasks, timers, and keyed cancellation intent. They expose neither
host requests nor subscription declarations. `WidgetUnmountContext` exposes a
`WidgetUnmountReason` of `Removed`, `Replaced`, or `RuntimeShutdown`. Contexts do
not expose mounted IDs or mutable runtime internals.

Mutable activation returns `WidgetActivationOutput<Action>`. Its optional action
and explicit `state_changed` fact are independent, so a widget can truthfully
report state-only mutation, action-only output, both, or neither. Action mapping
preserves the state-change fact.

The current public widget contract has no routed `event` capability,
`EventContext`, `WidgetEventOutput`, `SemanticCommand`, `CommandOrigin`, or
command submission. Those remain accepted M4C1 target architecture and are not
implemented API.

The default update invalidates `ALL` for correctness. Built-in text, button, and
linear-container widgets implement narrower comparison-based invalidation.
Button callback replacement requires no `Clone`, `Copy`, `Debug`, `Eq`, or
`PartialEq` on `Action` and does not itself invalidate visual capabilities.
`Button::on_activate(callback: impl FnMut() -> Action + 'static)` installs an
owned action factory invoked for every accepted proof activation. `on_press` is
removed without an alias.

`Element::map_action` replaces only action plumbing. It recursively delegates
every state-aware capability and preserves underlying widget/state type IDs.
Compatible reconciliation installs the newly authored description, mapper, and
activation factory while retaining state and mounted identity. No global
`Action: Clone`, `Send`, `Sync`, or `'static` bound exists.

## Safe core/runtime bridge

Core cannot depend on runtime, so doc-hidden `runenui_core::__runtime` plumbing
consumes an element into common fields, an erased mounted widget, and transient
children. Erased operations use checked `Any` downcasts. The bridge is absent
from the prelude, exposes no concrete widget downcasts, and provides no payload
or arena construction path. It is technically public only because core and
runtime are separate Rust crates; it is doc-hidden, unstable, unsupported for
application use, semver-exempt before 1.0, and safe. Both crates forbid unsafe
code.

A payload mismatch never invokes a typed callback with the wrong state and
never panics by design. It emits
`runenui.runtime.state-payload-mismatch`. Integrity-aware caches preserve the
difference between mismatch and ordinary capability defaults while publication
uses deterministic disabled/zero/vertical/default fallbacks. Activation exposes
`RuntimeError::WidgetStatePayloadMismatch`. A mismatch during compatible update
replaces immediately without partially committing the new description; a
mismatch discovered by another capability replaces on the next reconciliation.

## Identity and targeting

`MountedNodeId` currently privately stores:

```text
Arc<RuntimeInstanceMarker> + arena slot + u64 generation
```

It is `Clone`, `Debug`, `Eq`, and `Hash`, but not `Copy`. Equality compares
tokens with `Arc::ptr_eq`; hashing includes `Arc::as_ptr`, slot, and generation.
There is no global counter, random ID, serialization, or preorder identity.

`SemanticNodeId` is a distinct type and namespace with the same mounted lifetime
triplet. It survives compatible update and keyed reorder, and changes on
replacement. It is not yet a semantic-tree node or accessibility identity
contract.

Both IDs are process-local and runtime-instance-local. Validation checks the
runtime token first. A different token is `ForeignRuntime`; a same-runtime
invalid/vacant slot or generation mismatch is `StaleTarget`. Old IDs never
target slot replacements, even when the deterministic arena reuses the same
index.

The accepted M4C1 target moves `MountedNodeId`, `MonotonicInstant`,
`MonotonicTimeError`, and `WorkSequence` protocol values to core ownership while
runtime retains live namespaces, clocks, queue allocation, and timers. The
current implementation remains runtime-owned; no future signature is current
API.

Authored `ElementId` remains a validated lookup/diagnostic handle. It does not
affect reconciliation compatibility and may change while mounted identity
survives. Ambiguous authored-ID activation is rejected.

## Reconciliation

The root is compatible only when its key, widget type ID, and state type ID
match and it has no recorded integrity failure. `None` root keys compare equal.

For children, keys are sibling-local. A unique keyed child matches a unique old
sibling with the same key and compatible widget/state types regardless of
position. Keyed reorder preserves IDs, state, interaction slots, focus, and
clean caches. A key change, type incompatibility, or cross-parent placement
remounts.

Unkeyed children match by ordinal among unkeyed siblings, not absolute child
index. Keyed insertion/removal does not shift unkeyed matching; unkeyed
insertion/removal can shift later semantic meaning onto existing ordinal
lifetimes.

Keys duplicated in either old or new sibling lists are ineligible for reuse on
both sides. Every new occurrence mounts, every unmatched old occurrence
unmounts, and deterministic diagnostics record the ambiguous key and paths.
There is no first/last-match state preservation.

Final mounted child order exactly follows new authored order. `moved_count`
counts only a preserved node whose sibling position changed under the same
parent. Cross-parent changes are remounts.

## Lifecycle and shutdown

Initial mount and compatible update run parent-before-child in new authored
preorder. Each preserved node receives exactly one checked update from the new
description before that description is committed. Removal revokes every
exact-owner producer token, queued completion payload, source, mapper, timer,
and registry record children-before-parent before the corresponding unmount
hook begins. Each node remains arena-live through its hook;
arena removal, stale identity, and state drop follow the hook. Replacement
fully unmounts and drops the old subtree before mounting the new subtree.

State drops after its unmount hook. Interaction slots and caches disappear with
the mounted lifetime. Explicit `AppRuntime::shutdown`, `into_state`, and `Drop`
share one idempotent postorder `RuntimeShutdown` authority; every remaining node
receives exactly one shutdown unmount.

## Activation, focus, and interaction slots

Mounted proof activation validates runtime and target, reads checked activation
facts, rejects disabled/non-actionable targets, then conservatively reserves the
complete configured callback allowance before invoking the mutable widget/state
pair: one reconciliation generation, `2 * transaction_outputs + 1` queue slots,
`transaction_outputs` work generations plus that much free capacity in every
mounted-accessible work family, and `4 * transaction_outputs + 1` mandatory trace
records. A queued activation returns `Queued(ActivationCommit)` without pumping;
the commit exposes `first_sequence`, optional `primary_action_sequence`, and
`queued_envelopes`.

Auxiliary-only work is queued, state-only mutation and coalesced subscription
invalidation are `Activated`, and only an explicit absence of state mutation,
invalidation, subscription invalidation, action, or auxiliary work is
`NoEffect`. Application state does not change until a caller pumps.
`Saturated(ActivationCapacity)`, `Closed`, `Terminal`, disabled/non-activatable,
stale, foreign, and runtime-error outcomes are distinct. Saturation, closed,
terminal, or known sequence exhaustion rejects before widget state, factory,
invalidation, focus, reconciliation report, publication/cache, or application
mutation. Rejection and terminal trace records may still be appended when trace
sequencing remains available. State-only interaction changes validate focus
immediately. Accepted queue work requests the shared coalesced wake edge;
publication-affecting invalidation requests redraw independently. Pointer and
keyboard activation helpers use this same queue-backed proof authority, but
remain transitional, press-based proofs. Direct programmatic, pointer-press,
and keyboard activation are removed by M4C1; any explicitly retained focus-only
helper cannot emit an action or invoke activation and has an exact M4C3/M4C4/M4C5
removal owner. Routed semantic commands remain M4C1, release-inside pointer
behavior M4C3, focus scopes M4C4, keyboard/text/IME M4C5, and trace export/replay
M4D.

`ActivationCapacity` identifies `WaitingEnvelopes`, `LocalTasks`, `SendTasks`, or
`Timers`; activation never collapses those authorities into a generic queue
result.

## C9 public authority delta

| Old contract | New contract | Reason | Accepted ADR rule | Downstream migration | Proof owner |
|---|---|---|---|---|---|
| Send-subscription startup could accept provisionally | `Starting` submissions return `SendSubscriptionSinkError::NotStarted(exact_item)`; only `Running` accepts | Success must mean durable ownership | ADR 0006 producer admission | Match `NotStarted` and recover with `into_item` | `subscription_scheduler::send_subscription_start_outcomes_are_once_only_reclaimed_and_explicitly_retryable` |
| Cancelled send-task completion could enter ingress | `SendTaskCompletionError::Stale(exact_completion)` | Producer validity is exact-generation, not global | ADR 0006 cancellation | Match `Stale` separately from `Closed` | `scheduler_work::cancelled_send_completion_never_invokes_ui_mapper` |
| Activation exposed generic `QueueFull` | `ActivationResult::Saturated(ActivationCapacity)` | Report the bounded refusing authority | ADR 0006 configured saturation | Match the exact capacity | `activation_queue::conservative_activation_admission_rejects_every_bounded_authority_before_callback` |
| `Widget::activate` returned `Option<Action>` | It returns `WidgetActivationOutput<Action>` | State mutation is independent from action output | ADR 0003 widget protocol; ADR 0004 mounted state | Return `none`, `action`, `changed`, or `changed_with_action` | `mounted_work_output::activation_result_counts_auxiliary_batches_and_separates_wake_from_redraw` |
| `NoEffect` meant no newly queued output | It means no state change, invalidation, subscription invalidation, action, or auxiliary work | Coalescing does not erase semantic effect | ADR 0006 transaction semantics | Report mutation explicitly | `mounted_work_output::coalesced_subscription_invalidation_is_an_effect_not_no_effect` |

Focus stores `Option<MountedNodeId>`. It survives compatible update, authored-ID
change, and keyed reorder, and clears on removal, replacement, disablement, or
loss of actionability/focusability. Traversal follows current mounted preorder.

Each node privately owns hovered, pressed, capture-placeholder, and logical
scroll-offset slots. They survive compatible updates and reset on replacement.
The capture placeholder is an ownership proof only; M4 owns pointer IDs, routed
events, true capture, and release-inside activation.

## Invalidation and capability caches

`WidgetInvalidation` is a manual bitset with `NONE`, `INTERACTION`, `LAYOUT`,
`PAINT`, `SEMANTICS`, `DIAGNOSTICS`, and `ALL`, plus containment, union,
emptiness, `BitOr`, and `BitOrAssign`.

| Invalidation | Cleared widget caches | Scheduled dependent output |
|---|---|---|
| `INTERACTION` | activation | focus validation, interaction, semantics, paint |
| `LAYOUT` | measurement, child layout | layout, hit testing, semantic bounds, paint placement |
| `PAINT` | paint | paint output |
| `SEMANTICS` | semantics | semantic output |
| `DIAGNOSTICS` | widget diagnostics | diagnostic output |
| `ALL` | all capabilities | all dependent phases |

Each capability cache retains unresolved, ready, or payload-mismatch state.
`INTERACTION` alone does not clear paint or semantic widget facts. Structural and
common authored changes add runtime-detected phase work; they do not
automatically requery unrelated clean capabilities. Publication-context changes
compare root constraints, exact style-token content, and measurement-provider
identity/revision. Providers must change identity or revision whenever behavior
changes. Runtime-owned proof publication products and an inspectable
`SurfacePhaseReport` allow clean tree/style/layout/hit-test/paint/semantics/
diagnostics branches to be genuinely skipped while preserving clean widget
caches. Topology snapshots retain structural/alignment metadata only. Style and
layout execution use current mounted authored values, and reconciliation—not the
context key—detects authored token-reference or gap changes. The report lists
actual executed phase functions; private test-only counters are incremented at
the phase entry points and are independent from report recording.
M3 does not claim a production retained layout cache.

## Reports and publication

Initial mount completes reconciliation generation 1. Each successfully processed
application action increments once using `checked_add`; the action processor and
mutable activation preflight relevant exhaustion before application, widget,
mounted, factory, focus, cache, or reconciliation-report mutation. Diagnostic
rejection or terminal trace facts remain permitted when trace sequencing is
available.

`ReconciliationReport` defines counts by mounted lifetime: live nodes after
completion, new lifetimes mounted, preserved nodes updated once, lifetimes
ended, same-parent preserved moves, retained-focus truth, and structured
deterministic diagnostics containing complete duplicate-key old/new occurrences.

`AppRuntime::publish_surface(&mut self, context)` is the only public surface
publication authority. `MountedTreeIndex`, `SurfaceFrame`,
`SurfaceStyleReport`, and `SurfaceLayoutReport` expose equal mounted/semantic ID,
parent, authored-ID, and current preorder sequences with no ghost
actionable/focusable node. A tree change rebuilds every aligned product from one
current topology snapshot. Compatible common-field changes retain that topology
but refresh style/layout facts from current mounted nodes. The free transient
publication function is removed.

## Bounded canonical trace

`TraceConfig::new(capacity)` configures retained records; capacity zero disables
retention without allocating trace sequences or changing runtime behavior. The
configured capacity is a logical retention limit, not an eager allocation
request. `AppRuntime::trace()` borrows the one canonical `Trace`, whose
`records()` and `kinds()` iterators run oldest-to-newest without a duplicate
store. `TraceRecord` accessors expose a non-forgeable `TraceSequence`, structured
kind, optional `WorkSequence`, optional causal-parent `TraceSequence`, optional
reconciliation generations before/after, optional `TraceTarget`, and optional
`TraceWorkIdentity`. Work identity exposes only read-only owner, family, exact
private generation value, and optional authored `WorkKey`; it is not a runtime
capability. Action payloads are never stored. Scheduler records link the
application transaction, work request, generation commit, start attempt/outcome,
completion/firing/cancellation, and final action using causal parents and the
actual accepted envelope `WorkSequence` where one exists. This is the M4B
lineage foundation, not the deferred M4D export/replay contract.

Transaction semantic request/invalidation records preserve callback collector
order independently from cleanup-before-start queue grouping. Final action
acceptance is recorded before queue append, and the accepted action trace record
is the causal parent of the application transaction that later processes it.

Oldest records are evicted at capacity. `dropped_before_sequence()` is an
exclusive watermark: `Some(S)` means every trace sequence less than `S` is no
longer retained. Ordinary eviction cannot affect application behavior. When
enabled mandatory trace sequencing cannot advance, the runtime becomes terminal
before the pending mutable callback and cancels queued work. The current contract
has no routed-event causal graph, external sink, JSONL/export/redaction contract,
replay, or records for unimplemented routed-event, external-sink, export, or
replay work.

## Breaking migrations

Removed without aliases:

- `RuntimeNodeId`, `RuntimeNodeRef`, and `RuntimeTreeIndex`;
- `WidgetState`, `WidgetStateMismatch`, `WidgetLifecycle`,
  `WidgetLifecycleRequest`, and the old lifecycle context;
- direct element lifecycle/capability execution and preorder action extraction;
- free `publish_surface(Element)`;
- `AppRuntime::dispatch` and private direct-dispatch authorities;
- `Button::on_press` and one-shot button actions;
- duplicated, unbounded runtime-event/trace storage.

Added:

- core-owned `UiApp`, `HostProtocol`, `NoHostProtocol`, opaque ordered
  `Effects`/`IntoEffects`, `SubscriptionSet`, `LocalSubscriptionSource`,
  `SendSubscriptionSource`, `SendSubscriptionSink`, `WorkKey`, task/timer/host
  work descriptions, and mounted subscription declaration/invalidation
  capability;
- runtime-owned `RuntimeLimits`, live work generations, manual/host monotonic
  clocks, send-executor/completion handles, typed host-request tokens,
  `PumpBudget`/`PumpReport`, wake transport, and redraw request/acknowledgment;
- mounted/semantic identity and inspection;
- reconciliation generation/report vocabulary;
- state-aware widget lifecycle/activation contexts and unmount reasons;
- selective widget invalidation;
- focus/activation stale and foreign results;
- public runtime integrity errors;
- canonical action submission, work sequencing, bounded pumping, runtime status,
  terminal reasons, and explicit shutdown reports;
- repeatable `Button::on_activate` factories and queue-accurate activation
  outcomes;
- bounded canonical trace configuration, sequences, records, targets, opaque
  scheduler work identities/outcomes, and retention watermark.

M1 validated values, textual identity, typed configuration, arity-free
composition, protected generated products, and finite saturating geometry remain
in force. The current contract includes effects, subscriptions, tasks, timers,
host requests, all four readiness budgets, and wake/redraw. It does not imply
routed events, trace export/replay, complete trace-v2 normalization, or M4
completion.
