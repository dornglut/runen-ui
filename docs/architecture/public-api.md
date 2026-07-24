# Current Public API Contract

> **Category: Current contract**

This document records the reviewed public surface for application work, the
deterministic scheduler, routed semantic commands, and canonical trace. Source-level Rust
documentation is authoritative for signatures. [ADR 0003](../adr/0003-extensible-view-widget-component-protocol.md)
defines the open authoring/widget foundation; [ADR 0004](../adr/0004-mounted-runtime-reconciliation.md)
defines mounted ownership and reconciliation; accepted
[ADR 0006](../adr/0006-effects-scheduling-and-trace-v2.md) defines application
work and scheduling; M4B is implemented, owner-accepted, and squash-merged.
M4C1 and M4C2 are complete, owner-accepted, and squash-merged in
[archive PR #77](../history/public-repository-migration.md#accepted-imported-milestone-history)
and
[archive PR #99](../history/public-repository-migration.md#accepted-imported-milestone-history).
M4C2 used the documented infrastructure-only CI waiver after exact-head
local validation and final review passed. M4C3 was owner-accepted at feature
head `01b7ae018abeaff8d316764afba5bc8cde074381` after exact-head CI run
`29996101708` succeeded, then squash-merged in PR #15 as
`2fc165b9386f55c061d61232400375b13ad175bf`. M4C4 was owner-accepted at
feature head `f3201a83583af0c1d148bec87cd9140ff42795b7` after exact-head CI run
`30006170403` succeeded, then squash-merged in
[PR #22](https://github.com/dornglut/runen-ui/pull/22) as
`f95571634a9c6528e5834e9589b048ad5197bd15`. M4C5 becomes the next
implementation slice only after this post-merge authority update merges and its
resulting accepted `main` is recorded; M4D1–M4D3 remain blocked in sequence. M4
is active and incomplete. The accepted
[M4C delivery and routed-transaction charter](m4c-delivery-and-routed-transaction-charter.md)
records target ownership and transaction decisions but does not describe
implemented public API until each slice is accepted. The
[M4 conformance matrix](m4-conformance-matrix.md) owns observable acceptance, and
[work tracking](../work-tracking.md) owns volatile branch, head, blocker, and
next-action state.

## Ownership and inventory

`runenui_core` owns `UiApp`, host-neutral effects/subscriptions/work protocols,
validated authored values and identity, style intent and
resolution, transient `View`/`Element` authoring, typed built-in views, the open
state-aware `Widget`/`ChildLayoutWidget` contracts, proof capability values,
lifecycle contexts, `WidgetInvalidation`, the shared runtime namespace and
opaque `MountedNodeId`/`SemanticNodeId`/`SurfaceId`/`SurfaceInputContext`/
`MonotonicInstant`/`WorkSequence` protocol values, routed event/command vocabulary,
`EventContext`, and typed
recursive action mapping.

`runenui_runtime` owns `AppRuntime`, the canonical generalized FIFO and pump,
persistent mounted storage, reconciliation,
lifecycle execution, focus and interaction slots, mounted targeting,
invalidation scheduling, capability caches, measurement/layout execution,
bounded trace, live work registry, clocks, completion ingress, wake/redraw, and
mounted publication. The public runtime, mounted inspection, and integrity
vocabulary includes:

- core-owned `MountedNodeId`, `SemanticNodeId`, `SurfaceId`,
  `SurfaceInputContext`, `MonotonicInstant`, `MonotonicTimeError`, and
  `WorkSequence`, deliberately re-exported by runtime;
- `MountedNodeRef` and `MountedTreeIndex`;
- `ReconciliationGeneration` and `ReconciliationReport`;
- `RuntimeConfig`, `SubmitActionResult`, `CommandSubmission`,
  `UnacceptedCommand`, `SubmitCommandError`, `UnacceptedSurfaceCommand`,
  `SubmitSurfaceCommandError`, `PumpBudget`, and `PumpReport`;
- `RuntimeStatus`, `RuntimeTerminalReason`, `ShutdownReport`, and `RuntimeError`;
- `TraceConfig`, `TraceSequence`, `TraceRecord`, `TraceRecordKind`,
  `TraceSurfaceIngressKind`, `TraceSurfaceSnapshotKind`,
  `TraceSurfaceRejection`, `TraceTarget`, and `Trace`;
- read-only frame, style-report, layout-report, and publication products.

The ordinary preludes remain narrow. Specialist runtime/mounted/lifecycle
inspection is imported explicitly from crate roots. Generated IDs and
sequences, queue/envelope storage, mounted state, arena storage, reconciliation
reports, trace records, and publication products have no public constructors.

## Queue, pump, and runtime status

`RuntimeConfig::default()` selects a waiting-envelope capacity of 4096, a
`TraceConfig` capacity of 1024, and displayed hit-test snapshot retention of two
(current plus immediately previous). `with_queue_capacity`, `with_trace_config`,
and `with_surface_snapshot_retention(NonZeroUsize)` return adjusted values; fields
remain private. Queue capacity counts waiting
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

`AppRuntime::submit_command(target, command, origin)` is the sole public
semantic-command ingress. It validates the exact core-owned `MountedNodeId`,
appends a `SemanticCommand` envelope to the same FIFO, requests wake only after
acceptance, and returns `CommandSubmission` with the assigned `WorkSequence`.
`SubmitCommandError` distinguishes full, closed, exact terminal reason,
foreign, stale, missing, work-sequence exhaustion, and enabled-trace-sequence
exhaustion. Every rejection returns the exact owned target, command, and origin
through `UnacceptedCommand`; it invokes no widget callback and consumes no work
or trace sequence, allocates no trace record, and emits no wake. Accepted-then-
stale processing rejection is instead a canonical trace outcome causally owned
by the already accepted command.

`AppRuntime::submit_surface_command(context, logical_point, command, origin)` and
`submit_resolved_surface_command(context, target, command, origin)` are the checked
displayed-surface ingress paths. They validate runtime namespace, logical surface,
retained generation, coordinate revision, exact snapshot targeting or membership,
and current mounted target status before using the same command preflight,
canonical FIFO, routed transaction, semantic default, update, reconciliation, and
wake authority as direct exact-target submission. Rejection returns the exact owned
context, logical point or resolved target, command, and origin through
`UnacceptedSurfaceCommand`; structured kinds distinguish surface/context lifetime,
target lifetime, queue/status, and sequence failures.

Application and mounted output batches use one provisional transaction planner.
It resolves owner/family/key bindings for the complete batch, preflights every
queue sequence and work generation without consuming rejected capacity, then
atomically invalidates exact cancellation/replacement targets, installs accepted
records, and appends cleanup before ordered starts/actions.
For an application update the accepted order is cancellation cleanup, mounted
subscription reconciliation, update outputs, application subscription starts,
then mounted lifecycle outputs. Routed commands reuse the work planner while
committing coalesced subscription reconciliation, routed outputs, semantic
default output, then mounted work.

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
- routed `event` callbacks;
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

The open `Widget::event` capability receives immutable `UiEvent` data and one
borrowed `EventContext<'_, Action>`. The event protocol exposes semantic-command,
pointer, pointer-boundary, pointer-capture, and focus families. Ordinary pointer
and focus events use `Capture`/`Target`/`Bubble`; boundary and capture
notifications are target-only and non-cancelable, while focus notifications are
routed and non-cancelable. Programmatic/automation/accessibility/controller,
keyboard, and pointer sources retain internally consistent derivation. The
context exposes the original/current/optional-related target, origin, accepted
`WorkSequence`, `MonotonicInstant`, optional pointer identity, independent
physical hit target/path, and propagation/default facts. It provisionally
collects owned actions, delegated commands, exact-owner subscription
invalidation, ordinary invalidation, mounted tasks/timers/cancellation, stop
propagation, and prevent default plus ordered capture/release requests for the
current pointer. Recursive mapping preserves every staged capture request.
`WidgetEventOutput` reports only independent persistent-state mutation. Mapping
moves non-`Clone` actions and recursively maps mounted work while preserving
commands, controls, invalidation, and the state-change fact. Only the checked
erased widget bridge constructs and extracts `EventContext`; runtime supplies
its validated facts and output bound. Public origin constructors are direct-only,
while `emit_command` is the sole authority that turns callback output into a
delegated origin targeting the current node. `UiEvent::as_semantic_command`
returns `Option<&SemanticCommandEvent>` so later event variants do not require a
command-shaped accessor.

The default update invalidates `ALL` for correctness. Built-in text, button, and
linear-container widgets implement narrower comparison-based invalidation.
Button callback replacement requires no `Clone`, `Copy`, `Debug`, `Eq`, or
`PartialEq` on `Action` and does not itself invalidate visual capabilities.
`Button::on_activate(callback: impl FnMut() -> Action + 'static)` installs an
owned action factory invoked for every accepted routed semantic default. `on_press` is
removed without an alias.

`Element::map_action` replaces only action plumbing. It recursively delegates
every state-aware capability and preserves underlying widget/state type IDs.
Compatible reconciliation installs the newly authored description, mapper, and
activation factory while retaining state and mounted identity. No global
`Action: Clone`, `Send`, `Sync`, or `'static` bound exists.

## Safe core/runtime bridge

Core cannot depend on runtime, so doc-hidden `runenui_core::__runtime` plumbing
consumes an element into common fields, an erased mounted widget, and transient
children. Erased operations use checked `Any` downcasts. Every node on an
immutable route passes event-bridge validation before the first callback. The
bridge is absent from the prelude, has opaque fields, exposes no concrete widget
downcasts, and provides no payload or arena construction path. Its cross-crate
entry methods are technically public where runtime integration requires them;
that does not grant live runtime authority. Unrelated fabricated values cannot
extract or reuse a live namespace, obtain an accepted queue/trace identity, or
inject a fabricated context/sequence into an accepted transaction. The bridge
is doc-hidden, unstable, unsupported for application use, semver-exempt before
1.0, and safe. Both crates forbid unsafe code.

A payload mismatch never invokes a typed callback with the wrong state and
never panics by design. It emits
`runenui.runtime.state-payload-mismatch`. Integrity-aware caches preserve the
difference between mismatch and ordinary capability defaults while publication
uses deterministic disabled/zero/vertical/default fallbacks. Activation exposes
`RuntimeError::WidgetStatePayloadMismatch`. A mismatch during compatible update
replaces immediately without partially committing the new description; a
mismatch discovered by another capability replaces on the next reconciliation.

## Identity and targeting

Core-owned `MountedNodeId` privately stores:

```text
shared opaque runtime namespace + checked u32 arena slot + u64 generation
```

It is `Clone`, `Debug`, `Eq`, and `Hash`, but not `Copy`. Equality and hashing
include namespace identity, slot, and generation.
There is no global counter, random ID, serialization, or preorder identity.

`SemanticNodeId` is a distinct type using the same runtime namespace and mounted
lifetime triplet. It survives compatible update and keyed reorder, and changes on
replacement. It is not yet a semantic-tree node or accessibility identity
contract.

Both IDs are process-local and runtime-instance-local. Validation checks the
runtime namespace first. A different namespace is foreign; an issued slot whose
generation/lifetime no longer matches is stale; a same-runtime slot beyond the
addressable arena is missing. Old IDs never
target slot replacements, even when the deterministic arena reuses the same
index.

`MountedNodeId`, `MonotonicInstant`, `MonotonicTimeError`, and `WorkSequence`
have one core-owned type authority. Runtime alone creates the live namespace,
checks `usize -> u32` slot conversion, allocates non-wrapping generations and
work sequences, advances clocks, and schedules timers. Hidden bridges carry no
arena, clock, queue, or validation-bypass authority.

Authored `ElementId` remains a validated lookup/diagnostic handle. It does not
affect reconciliation compatibility and may change while mounted identity
survives. M4C1 command submission accepts only an exact mounted target; authored
automation lookup is deferred to M4C5.

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

## Routed commands, focus, and interaction slots

`AppRuntime::submit_command` appends one non-reentrant semantic-command
envelope. At the queue front runtime revalidates the exact target, snapshots one
owned root-to-target route, validates every route node and erased event bridge,
and admits the complete configured transaction boundary before the first
callback. Capture visits root-to-parent, target runs once, and bubble visits
parent-to-root. Stopping propagation affects only later callbacks; preventing
default affects only the cancelable semantic default.

One bounded transaction ledger counts routed actions, delegated commands,
semantic-default output, mounted effects/cancellation, and unique exact-owner
subscription invalidation. Conservative maximum-safe admission reserves the
configured aggregate output allowance for every callback-accessible family,
even when a particular callback ultimately emits nothing. It also covers queue
slots and work sequences, reconciliation and work generations, each mounted-
accessible work family, and mandatory trace sequencing. A zero output allowance
rejects before callback; trace retention capacity zero disables trace allocation without
changing behavior. Unexpected failure after mutable callback entry poisons the
runtime rather than dropping provisional output.

Commit order is widget mutation/invalidation, coalesced subscription
reconciliation, routed actions and delegated commands in emission order,
semantic-default output, then mounted work. Delegated commands target the
current routed node, preserve source, change derivation to `Delegated`, receive a
later sequence, and never run recursively.

For unprevented `Activate`, the runtime re-queries the original target after
callback invalidation. Only a still-live enabled/actionable target invokes the
existing widget activation capability, exactly once, as semantic default.
Prevented activation never invokes its factory. `CancelOrBack`, `OpenMenu`, and
`OpenContextMenu` route once and have no default action, runtime mutation, or
second ancestor pass. Programmatic, automation, accessibility-stub, and
normalized-controller origins use this same exact-target path; authored-ID
automation and semantic accessibility resolution are not implemented.

Direct programmatic activation, direct focus mutation/traversal helpers, the
transitional `FocusTargetResult`, and the old pointer activation/resolution
helpers are removed. Focus changes enter through `submit_command`; normalized
keyboard modality uses `CommandOrigin::keyboard()` without adding raw keyboard
routing. M4C2 owns surface context, M4C3 implements pointer lifecycle/release-
inside activation, M4C4 implements focus scopes/modality, M4C5 owns
keyboard/text/IME and automation resolution, M4D trace
normalization/export/replay, and M5 semantic accessibility mapping.

One runtime-owned `FocusState` retains the exact focused mounted lifetime, its
committed focus-within route, exact-generation scope memories, last
`FocusReason`, and last accepted `InputModality`. `Element::focusable`,
`focus_hidden`, and `focus_scope` author participation without exposing arena or
generation construction. `FocusScope` carries separate linear/directional
`FocusBoundaryPolicy`: the root wraps linearly and stops directionally; nested
defaults delegate; explicit nested policy may trap, stop, wrap, delegate, or
derive `LogicalFocusScroll` through the canonical command queue.

`FocusNext`, `FocusPrevious`, four directional commands, `RequestFocus`, and
`RestoreFocus` use the canonical exact-target command transaction. Linear
selection follows current mounted logical order. Directional selection reads
the current retained publication rectangles and uses mounted order only as its
final tie-break; its private score is not API. Remembered restoration accepts
only the exact live, eligible generation and otherwise uses normal fallback.

Committed transitions update focus and focus-within atomically, then route
non-cancelable `FocusOut` before `FocusIn`, each Capture/Target/Bubble, before
initiating routed/default outputs. `FocusEvent` exposes kind, reason, and exact
target; `EventContext::related_target` exposes the opposite live endpoint.
Removal/replacement suppresses post-unmount delivery, clears incompatible
memory, and records the exact cleanup reason. Shutdown clears focus and memory
with `FocusReason::Shutdown` while retaining the last accepted modality.

Pointer interaction state is runtime-owned per checked
`PointerId`: device and surface ownership, physical path, buttons, pressed
owner/inside state, and one exact live capture owner remain distinct.
`submit_pointer` is the sole public pointer ingress and never accepts an
unchecked mounted target. Down/move/up/cancel/wheel use the canonical queue and
routed transaction engine. Primary activation requires an eligible down and
physical release inside the same exact live owner; wheel derives one route-only
logical-scroll command. Raw keyboard/text/IME routing remains M4C5.

## C9 public authority delta

| Old contract | New contract | Reason | Accepted ADR rule | Downstream migration | Proof owner |
|---|---|---|---|---|---|
| Send-subscription startup could accept provisionally | `Starting` submissions return `SendSubscriptionSinkError::NotStarted(exact_item)`; only `Running` accepts | Success must mean durable ownership | ADR 0006 producer admission | Match `NotStarted` and recover with `into_item` | `subscription_scheduler::send_subscription_start_outcomes_are_once_only_reclaimed_and_explicitly_retryable` |
| Cancelled send-task completion could enter ingress | `SendTaskCompletionError::Stale(exact_completion)` | Producer validity is exact-generation, not global | ADR 0006 cancellation | Match `Stale` separately from `Closed` | `scheduler_work::cancelled_send_completion_never_invokes_ui_mapper` |
| Direct mounted activation was public runtime authority | `submit_command(exact_target, Activate, origin)` is the only semantic ingress | Every source must use routing, admission, default, FIFO, and trace | ADR 0005 canonical commands | Submit and pump; recover exact `UnacceptedCommand` on rejection | `routed_commands`, Counter, and downstream routed-event conformance |
| `Widget::activate` returned `Option<Action>` | It returns `WidgetActivationOutput<Action>` and is invoked only by routed `Activate` default | State mutation is independent from action output | ADR 0003 widget protocol; ADR 0004 mounted state; ADR 0005 default | Return `none`, `action`, `changed`, or `changed_with_action` | `mounted_work_output::routed_activation_separates_scheduler_wake_from_redraw` |

The runtime owns one bounded pointer registry rather than per-node aggregate
interaction booleans. Removal, replacement, disablement, loss of actionability,
cancel, up, shutdown, and drop clear incompatible exact-generation ownership.
No production scroll-offset mutation is implied by logical-scroll intent.

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
routed transaction admission preflight relevant exhaustion before application, widget,
mounted, factory, focus, cache, or reconciliation-report mutation. Processing-
rejection, routed-integrity, or terminal trace facts remain permitted when trace
sequencing is available; submission-time rejection never allocates one.

`ReconciliationReport` defines counts by mounted lifetime: live nodes after
completion, new lifetimes mounted, preserved nodes updated once, lifetimes
ended, same-parent preserved moves, retained-focus truth, and structured
deterministic diagnostics containing complete duplicate-key old/new occurrences.

`AppRuntime::publish_surface(&mut self, context)` is the only public surface
publication authority. Every call returns `SurfacePublication` with a fresh opaque
`SurfaceInputContext` naming the one logical surface, a fresh coordinate revision,
and the exact displayed hit-test generation. The runtime retains a configurable
nonzero bounded `VecDeque` of immutable hit-test snapshots; oldest retirement is
deterministic, and retained contexts never re-hit-test current geometry.
`MountedTreeIndex`, `SurfaceFrame`, `SurfaceStyleReport`, and
`SurfaceLayoutReport` expose equal mounted/semantic ID, parent, authored-ID, and
current preorder sequences with no ghost actionable/focusable node. A tree change
rebuilds every aligned renderer product from one current topology snapshot.
Compatible common-field changes retain that topology but refresh style/layout facts
from current mounted nodes. Publication equality deliberately compares only the
renderer-facing frame/style/layout products; callers compare `input_context()`
explicitly when exact displayed-snapshot identity matters. The free transient
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
actual accepted envelope `WorkSequence` where one exists. M4C1 event records
also expose logical instant, immutable original target, callback current target,
and command origin. M4C2 surface ingress adds structured context acceptance,
current-versus-retained snapshot selection, displayed generation/revision, exact
target binding, and rejection facts. For accepted surface commands the chain is
`SurfaceContextAccepted -> SurfaceTargetBound -> CommandSubmissionAccepted ->
RoutedEventStarted`; exact mandatory admission reserves the three-record surface
prefix plus the future routed outcome. Acceptance causally parents route start and
snapshot; phase,
control, state, invalidation, output collection, default, and commit records form
the routed chain. Collected actions and delegated commands parent their later
accepted envelopes and transactions. Submission and processing rejection are
distinct by observation: submission rejection has no canonical record and
consumes no trace identity, while processing rejection after acceptance is
recorded. Routed integrity failures classify broken topology, event-bridge
mismatch, callback-bridge failure, output-allowance overflow, semantic-default
failure, or commit-invariant failure without losing accepted causal facts. This
remains an in-memory causal graph, not the deferred M4D
normalization/export/replay contract.

M4C3 adds pointer submission, ordered validation and stream resolution,
physical-path and boundary-bundle planning, default applied/suppressed,
interaction commit, capture/boundary notification, activation/logical-scroll
collection, stationary-publication re-hit, and terminal diagnosis-to-cleanup
facts. The accepted pointer `WorkSequence` and causal parents reconstruct the
slice-local lineage; M4D may normalize this schema but does not own missing
pointer parentage.

M4C4 adds focus command and scope-policy evaluation, directional candidate and
restoration outcomes, exact old/new focus targets and reasons, focus-within
changes, routed notification queue/suppression, retained modality, reconciliation
cleanup, and shutdown ordering. The accepted command `WorkSequence` and causal
parents reconstruct the slice-local focus/modality lineage; M4D may normalize
this schema but does not own missing M4C4 parentage.

Transaction semantic request/invalidation records preserve callback collector
order independently from cleanup-before-start queue grouping. Final action
acceptance is recorded before queue append, and the accepted action trace record
is the causal parent of the application transaction that later processes it.

Oldest records are evicted at capacity. `dropped_before_sequence()` is an
exclusive watermark: `Some(S)` means every trace sequence less than `S` is no
longer retained. Ordinary eviction cannot affect application behavior. When
enabled mandatory trace sequencing cannot advance, the runtime becomes terminal
before the pending mutable callback and cancels queued work. The current contract
has no external sink, JSONL/export/redaction contract, replay, or M4D-normalized
schema.

## Breaking migrations

Removed without aliases:

- `RuntimeNodeId`, `RuntimeNodeRef`, and `RuntimeTreeIndex`;
- `WidgetState`, `WidgetStateMismatch`, `WidgetLifecycle`,
  `WidgetLifecycleRequest`, and the old lifecycle context;
- direct element lifecycle/capability execution and preorder action extraction;
- free `publish_surface(Element)`;
- `AppRuntime::dispatch` and private direct-dispatch authorities;
- `Button::on_press` and one-shot button actions;
- direct runtime activation, activation result/capacity compatibility types,
  combined input intent, and pointer/keyboard activation helpers;
- direct focus mutation/traversal helpers, `FocusTargetResult`,
  `KeyboardFocusResult`, `handle_keyboard_focus`, and the transitional runtime
  policy module;
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
- core-owned mounted/semantic identity, monotonic time, work sequence values,
  and runtime-owned inspection;
- reconciliation generation/report vocabulary;
- state-aware widget lifecycle/activation/event contexts and unmount reasons;
- selective widget invalidation;
- one read-only `FocusState`, host-neutral focus scope/focusability/modality/reason
  values, semantic focus commands, and routed focus notifications;
- public runtime integrity errors;
- canonical action submission, work sequencing, bounded pumping, runtime status,
  terminal reasons, and explicit shutdown reports;
- repeatable `Button::on_activate` factories invoked only by routed semantic default;
- bounded canonical trace configuration, sequences, records, targets, opaque
  scheduler work identities/outcomes, routed command/surface/pointer/focus causal
  facts, and retention watermark;
- routed event/command vocabulary, `EventContext`, `WidgetEventOutput`, checked
  mapped event capability, and exact-target command submission with owned
  rejection recovery.

M1 validated values, textual identity, typed configuration, arity-free
composition, protected generated products, and finite saturating geometry remain
in force. The current contract includes effects, subscriptions, tasks, timers,
host requests, all four readiness budgets, wake/redraw, and M4C1 exact-target
routed semantic commands, M4C2 displayed-generation surface context, the M4C3
host-neutral pointer lifecycle, and the owner-accepted M4C4 focus-scope/modality
protocol. It does not imply native host translation, production scrolling, M4C5
keyboard/text/IME or authored-ID automation, M4D trace export/replay, M5 semantic
accessibility mapping, or M4 completion.
