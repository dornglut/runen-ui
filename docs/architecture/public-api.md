# Current Public API Contract

> **Category: Current contract**

This document records the reviewed public surface for application work, the
deterministic scheduler, routed semantic commands, canonical trace, semantic
contribution/identity, renderer-independent semantic publication/update/
diagnostics, and accepted M5C semantic action ingress/accessibility resolution.
Source-level Rust documentation is authoritative for signatures.
[ADR 0003](../adr/0003-extensible-view-widget-component-protocol.md) defines the
open authoring/widget foundation; [ADR 0004](../adr/0004-mounted-runtime-reconciliation.md)
defines mounted ownership and reconciliation; accepted
[ADR 0006](../adr/0006-effects-scheduling-and-trace-v2.md) defines application
work and scheduling; M4B is implemented, owner-accepted, and squash-merged.
M4C1 and M4C2 are complete, owner-accepted, and squash-merged in
[archive PR #77](../history/public-repository-migration.md#accepted-imported-milestone-history)
and
[archive PR #99](../history/public-repository-migration.md#accepted-imported-milestone-history).
M4C2 used the documented infrastructure-only CI waiver after exact-head local
validation and final review passed. M4C3 was owner-accepted at feature head
`01b7ae018abeaff8d316764afba5bc8cde074381` after exact-head CI run
`29996101708` succeeded, then squash-merged in PR #15 as
`2fc165b9386f55c061d61232400375b13ad175bf`. M4C4 was owner-accepted at
feature head `f3201a83583af0c1d148bec87cd9140ff42795b7` after exact-head CI run
`30006170403` succeeded, then squash-merged in
[PR #22](https://github.com/dornglut/runen-ui/pull/22) as
`f95571634a9c6528e5834e9589b048ad5197bd15`. M4C5 was owner-accepted at
feature head `d0d2ef1d53a8ab1d940beb4155f5f991229f042e` after exact-head CI run
`30843238697` succeeded and independent rereview found no blocking defect. It
was squash-merged in [PR #27](https://github.com/dornglut/runen-ui/pull/27) as
`284ecdcfe107e0a7afc88e4bf4fc82eecc52a226`. M4D1 was owner-accepted at
feature head `990c49edb5b68c37dd3b7d37dd3f1196a9557c7a` after canonical exact-head
CI run `31269401262` / #657 and the frozen complete-diff review passed. It was
squash-merged in [PR #39](https://github.com/dornglut/runen-ui/pull/39) as
`2fe269366386d7aee9de2a2573498b64ad486293`. M4D2 was owner-accepted at
feature head `1bd7dcfdbb46dec52da62faabb739c835e971c80` after canonical exact-head
CI run `31321448821` / #712 and the frozen complete-diff review passed. It was
guarded-squash-merged in [PR #41](https://github.com/dornglut/runen-ui/pull/41)
as `8c67655ffce438c2e35e6478e7299bd704033b8b`, and all 23 changed-file blob
identities match between reviewed feature head and accepted squash. The M4D2
post-merge authority reconciliation is also accepted and merged. M4D3 was
owner-accepted at feature head `b5f72ccaa89a9fb54d81ec3f35701cbdfbc9ba5d` after canonical exact-head CI run
`31398930987` / #765 and the final critical cold review passed. It was
guarded-squash-merged in
[PR #43](https://github.com/dornglut/runen-ui/pull/43) as
`596f0d823b9833d71a038cc4aebe834c7b94e4a6`, and all 16 changed-file blob
identities match between reviewed feature head and accepted squash. The final M4
authority reconciliation records all eight M4D3-owned rows as owner-accepted,
closes M4, and activates M5 semantics and deterministic public testing. The accepted
[M4C delivery and routed-transaction charter](m4c-delivery-and-routed-transaction-charter.md)
continues to record M4 target ownership and transaction decisions, and the
[M4 conformance matrix](m4-conformance-matrix.md) remains M4 observable-
acceptance authority.

M5A semantic contribution and independent identity is owner-accepted. The
reviewed M5A feature head `8377ced53c08d7b5be3020368ceddd3ee81294a5`
passed exact-head CI run `31497457992` / #889 and was guarded-squash-merged in
[PR #53](https://github.com/dornglut/runen-ui/pull/53) as
`e3c304600ec1777cd17a1973946a43c765df1c31`; its required reconciliation is
accepted through PR #54 as recorded by work tracking.

M5B semantic publication and incremental updates is owner-accepted and fully
reconciled. Exact reviewed head `3b9db8b37098786cc0d53d38ae5d597c3460c38b`
passed exact-head CI #1082 and was guarded-squash-merged in
[PR #58](https://github.com/dornglut/runen-ui/pull/58) as
`43d23aefb81757a516ae569b3e86b9e0f2c71e23`; reviewed and squash trees are
identical at `1708d2536c6f1d202ac58dd7cb5f3cc97a438517`. Because the connector-origin
merge did not emit the normal push workflow event, the exact squash was
independently revalidated through unchanged read-only pull-request CI #1084
attempt 2 in temporary PR #60, which was closed unmerged. The mandatory M5B
reconciliation was then owner-accepted and guarded-squash-merged in PR #61 as
`afb7f8f363a8df3eb51be1a9bc5f0f180f84190b`; accepted-main CI #1090 passed.

M5C semantic action ingress and accessibility resolution is now owner-accepted.
Exact reviewed feature head `504899b79059eb94ad4474d67bba1e27eb30b374`
passed exact-head CI #1170 / `31889342640`, was explicitly accepted by the
repository owner, and was guarded-squash-merged in
[PR #62](https://github.com/dornglut/runen-ui/pull/62) as
`846c4e6adfdcd9236586f1b9978f63e71ff4fb86`. Reviewed head and squash share
exact tree `dfa7cb71166a3f333b560508a7e82fbeb45df000`, and accepted-main push CI
#1171 / `31903354382` passed at that exact squash. The mandatory M5C post-merge
current-contract reconciliation is the current gate; M5D #50 remains blocked
until that reconciliation is accepted, merged, and accepted-main verified. The
accepted [M5 semantics and testing charter](m5-semantics-and-testing-charter.md)
owns the remaining sequential M5D–M5E boundaries; the
[M5 conformance matrix](m5-conformance-matrix.md) owns observable acceptance;
and [work tracking](../work-tracking.md) owns volatile branch, head, blocker,
and next-action state.

## Ownership and inventory

`runenui_core` owns `UiApp`, host-neutral effects/subscriptions/work protocols,
validated authored values and identity, style intent and resolution, transient
`View`/`Element` authoring, typed built-in views, the open state-aware
`Widget`/`ChildLayoutWidget` contracts, lifecycle contexts,
`WidgetInvalidation`, the shared runtime namespace and opaque
`MountedNodeId`/`SemanticNodeId`/`SurfaceId`/`SurfaceInputContext`/
`MonotonicInstant`/`WorkSequence` protocol values, routed event/command
vocabulary, `EventContext`, typed recursive action mapping, canonical
`LogicalSize`/`LogicalRect`, and the platform-neutral semantic authoring/action
vocabulary: `SemanticKey`, `SemanticRole`, `SemanticValue`, `SemanticText`,
`SemanticState`, `SemanticAction`, `SemanticActionTarget`,
`SemanticRelationshipKind`, `SemanticReference`, `SemanticRelationship`,
`SemanticBounds`, `SemanticItem`, `SemanticNodeContribution`,
`SemanticContribution`, `SemanticContributionValidation`,
`SemanticContributionError`, and `SemanticContributionContext`.

`runenui_runtime` owns `AppRuntime`, the canonical generalized FIFO and pump,
persistent mounted storage, reconciliation, lifecycle execution, focus and
interaction slots, mounted targeting, invalidation scheduling, capability
caches, the separate runtime semantic generational arena and mounted-owner/
semantic-key binding store, measurement/layout execution, bounded trace, live
work registry, clocks, completion ingress, wake/redraw, mounted publication, the
accepted renderer-independent semantic publication state, and exact semantic
action admission/resolution. The public runtime, mounted inspection, integrity,
and M5C vocabulary includes:

- core-owned `MountedNodeId`, `SemanticNodeId`, `SurfaceId`,
  `SurfaceInputContext`, `MonotonicInstant`, `MonotonicTimeError`, and
  `WorkSequence`, deliberately re-exported by runtime;
- `MountedNodeRef` and `MountedTreeIndex` for mounted inspection only;
- `ReconciliationGeneration` and `ReconciliationReport`;
- `RuntimeConfig`, `SubmitActionResult`, `CommandSubmission`,
  `UnacceptedCommand`, `SubmitCommandError`, `UnacceptedSurfaceCommand`,
  `SubmitSurfaceCommandError`, `PumpBudget`, and `PumpReport`;
- M5C `SemanticActionRequest`, `SubmitSemanticActionErrorKind`, and
  `SubmitSemanticActionError`; successful semantic submission reuses the existing
  `CommandSubmission` receipt above;
- host-neutral `KeyboardEvent`, `CommittedTextEvent`, `CompositionEvent`,
  opaque `CompositionGeneration`, checked `CompositionRange`, and explicit
  `WidgetTextInput` capability values, plus input/automation submission
  receipts and owned recovery errors;
- `RuntimeStatus`, `RuntimeTerminalReason`, `ShutdownReport`, `RuntimeError`,
  `PublishSurfaceError`, and `SurfacePublicationCounter`;
- `TraceConfig`, `TracePayloadCapture`, `TraceSequence`, `TraceRecord`,
  `TraceRecordKind`, `TraceSurfaceIngressKind`, `TraceSurfaceSnapshotKind`,
  `TraceSurfaceRejection`, `TraceTarget`, `TraceSinkDeliveryOutcome`, `Trace`,
  `TraceJsonlLine`, `TraceSinkReceiveError`, and `TraceSinkReceiver`;
- accepted M4D3 offline replay types `TraceReplay`,
  `TraceReplayCompleteness`, `TraceReplayError`, `TraceReplayKind`,
  `TraceReplayRecord`, `TraceReplaySequence`, and `TraceReplayWorkSequence`;
- renderer-facing `SurfaceFrame`, `SurfaceStyleReport`, and
  `SurfaceLayoutReport` products;
- accepted M5B `SemanticRevision`, `SemanticNodeState`,
  `SemanticRelationship`, `SemanticNode`, `SemanticSnapshot`,
  `SemanticFocusChange`, `SemanticUpdate`, `SemanticUpdateResult`, and
  `SemanticPublication`;
- accepted M5B `SemanticDiagnostic`, `SemanticDiagnosticReport`, and
  `SemanticOwnerWithdrawalReason`;
- `SurfacePublication` as the complete aligned publication aggregate with
  explicit renderer-only and complete-product observation/extraction.

The ordinary preludes remain narrow. Specialist runtime/mounted/lifecycle
inspection is imported explicitly from crate roots. Generated IDs and
sequences, queue/envelope storage, mounted state, mounted/semantic arena storage,
semantic owner bindings, reconciliation reports, trace records, and publication
products have no public constructors. Replay identities are separately typed
observational values and have no conversion into live runtime-issued
`TraceSequence` or `WorkSequence` authority. M5B exposes semantic IDs only in
read-only semantic snapshots/updates. M5C accepts those exact public IDs only
through `SemanticActionRequest`, resolves them against private mounted owner/key
bindings, and exposes read-only semantic-origin callback metadata through
`SemanticActionTarget`; neither surface exposes a public semantic-to-
`MountedNodeId` routing shortcut.

## Queue, pump, and runtime status

`RuntimeConfig::default()` selects a waiting-envelope capacity of 4096, a
`TraceConfig` capacity of 1024, and displayed hit-test snapshot retention of two
(current plus immediately previous). `with_queue_capacity`, `with_trace_config`,
and `with_surface_snapshot_retention(NonZeroUsize)` return adjusted values; fields
remain private. Queue capacity counts waiting envelopes only, and zero is valid.
Queue and trace capacities are logical limits: internal storage grows with
accepted envelopes or retained records and does not reserve the complete
configured capacity when the runtime mounts. `TraceConfig::new(capacity)`
defaults text/IME capture to `TracePayloadCapture::Redacted` and leaves external
sink delivery disabled. `with_payload_capture(TracePayloadCapture::FullText)`
explicitly opts canonical trace records into exact committed-text/preedit
retention, independently from action labels and sink delivery.
`with_sink_capacity(NonZeroUsize)` enables the subordinate lazily bounded sink
without treating the configured logical capacity as an eager allocation request.
Trace capacity zero is fully dormant: it creates no sink, retains no raw
diagnostic payload, and invokes no action label hook. Default live local-task,
send-task, timer, subscription, and host-request limits are 2048 each; the
default transaction-output limit is 1024.
`RuntimeLimits::with_subscription_diagnostics` independently bounds the public
diagnostic retention slice; zero disables that auxiliary retention without
changing canonical trace behavior.

`AppRuntime::submit_action(action)` appends one application action when running,
capacity is available, and the non-wrapping sequence authority can advance. It
returns a runtime-issued `WorkSequence`, beginning at 1, or a
`SubmitActionError<Action>` classified as full, closed, or terminal. Rejection
returns the exact owned action through `into_action` and does not consume a work
sequence. `Action` requires no `Clone`, `Send`, or `Debug` bound.

`AppRuntime::submit_command(target, command, origin)` is the public exact-mounted
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

`AppRuntime::submit_semantic_action(request: SemanticActionRequest) ->
Result<CommandSubmission, SubmitSemanticActionError>` is the accepted public
exact-semantic ingress. Callers construct the request with
`SemanticActionRequest::new(surface, target, action)`; the request fields remain
private. M5 supports exactly `Activate`, `RequestFocus`, `OpenMenu`, and
`OpenContextMenu`; there is no semantic `LogicalScroll` variant or compatibility
alias. Submission validates runtime status, exact current surface/namespace,
semantic target authority/key, semantic freshness, exact current publication
membership, support, composed hidden/inert/disabled state, action-specific
readiness, and canonical queue/work/trace capacity before acceptance. A dirty
semantic authority rejects rather than refreshing synchronously; layout-only
dirtiness remains admissible. Submission invokes no widget callback. Acceptance
returns the existing `CommandSubmission` with the canonical `WorkSequence` and
appends to the existing command FIFO; rejection returns
`SubmitSemanticActionError`, whose accessors recover the exact owned request.

The queued semantic work privately retains exact surface, semantic ID, semantic
key, mounted-owner lifetime, and action metadata. Queue-front processing
revalidates that exact binding before any callback. An accepted request that has
become stale records canonical `SemanticActionProcessingRejected` under its
accepted `WorkSequence`, invokes no callback, and never retargets. Semantic-origin
routed callbacks receive read-only `SemanticActionTarget` metadata; ordinary and
delegated commands do not inherit it. `RequestFocus` uses the accepted M4
Focusable/Automatic eligibility. PRIMARY `Activate` requires owner actionable and
enabled; named `Activate` requires authored support, owner enabled, and an exact
non-disabled/non-inert node without imposing an unrelated owner-actionable gate.
Menu/context actions require exact support/state and likewise do not impose an
actionable gate.

After routed callbacks and before semantic `Activate` or `RequestFocus` default
mutation, runtime revalidates the exact accepted semantic authority without
synchronous refresh. Explicit `prevent_default()` yields canonical
`SemanticDefaultSuppressed`; callback-caused semantic invalidation yields the
distinct `SemanticDefaultTargetInvalidated` outcome. Both fail closed and retain
the accepted work/causal lineage.

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
records, and appends cleanup before ordered starts/actions. For an application
update the accepted order is cancellation cleanup, mounted subscription
reconciliation, update outputs, application subscription starts, then mounted
lifecycle outputs. Routed commands reuse the work planner while committing
coalesced subscription reconciliation, routed outputs, semantic-default output,
then mounted work.

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
Work-sequence, work-generation, reconciliation-generation, or enabled-trace-
sequence exhaustion in direct command or already-accepted mutable work is
terminal and non-resettable: no later mutable callback runs, queued envelopes are
cancelled, new submissions return their actions, and inspection/state extraction
remain available. Public authored-ID automation resolution is the deliberate
exception: work-sequence or enabled-trace-sequence exhaustion returns the exact
automation request without terminalizing, enqueuing work, invoking callbacks,
changing focus/widget/application state, or requesting wake. `shutdown()` is
explicit and idempotent; its report exposes whether shutdown was already complete
plus cancelled-envelope and unmounted-lifetime counts. `into_state` and `Drop`
invoke the same shutdown authority.

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
must author sibling-local unique keys. Recursive `Element::map_action` preserves
semantic contribution exactly because contribution contains no application
action type.

The private mounted tree is the sole runtime authority. `MountedTreeIndex`
traverses logical mounted preorder; arena slot order is never observable as tree
order. A mounted node owns parent/ordered children, authored metadata, current
widget description, persistent erased state, interaction slots, replacement
status, capability caches, private semantic owner bindings, and internal dirty
phases. `MountedTreeIndex` does not publish semantic-node identities; M5B's
independent public semantic snapshot/index does so without turning mounted
inspection into semantic authority.

## State-aware widget contract

Every `Widget<Action>` declares `State: 'static` and creates it. Stateless
widgets use `State = ()`. The runtime passes persistent state to:

- `mount`, `update`, and `unmount`;
- routed `event` callbacks;
- immutable activation facts, text-input facts, measurement, paint,
  `semantics`, and diagnostics;
- mutable activation;
- `ChildLayoutWidget::child_layout`.

The semantic capability is:

```rust
fn semantics(
    &self,
    state: &Self::State,
    context: SemanticContributionContext,
) -> SemanticContribution
```

`SemanticContributionContext` exposes only the direct mounted-child count; it
contains no mounted/semantic ID, runtime namespace, layout coordinate, focus, or
action authority. A widget contribution is an ordered action-type-independent
owner-local forest. `SemanticKey::PRIMARY` is reserved and distinct from a named
`"primary"` key. Additional nodes use validated named keys. A zero-node owner is
transparent. When a nonempty owner has direct mounted children, the contribution
contains exactly one `SemanticItem::MountedChildren`; when it has none, that
marker is forbidden. Validation rejects duplicate keys, missing/duplicate/
unnecessary markers, and missing local relationship targets without implicit
repair or first/last selection.

`SemanticNodeContribution` carries platform-neutral role, optional name and
description, value, authored `SemanticState` (`disabled`, `hidden`, `inert`),
deduplicated semantic action intent, relationships, `SemanticBounds`, optional
plain text, and recursive local semantic children. `SemanticBounds::Owner` uses
the mounted owner's runtime-derived bounds; `OwnerLocal(LogicalRect)` is
validated owner-local geometry only. Widgets cannot author absolute surface
coordinates or runtime focus through the contribution contract. `SemanticAction`
describes the accepted M5 action vocabulary; executable exact semantic-node
action ingress is provided separately by `AppRuntime::submit_semantic_action`.
AccessKit/native types are absent.

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
pointer, pointer-boundary, pointer-capture, focus, keyboard, committed-text, and
composition families. Ordinary pointer, focus, keyboard, committed-text, and
composition events use checked Capture/Target/Bubble routing where their family
contract requires it; boundary and capture notifications are target-only and
non-cancelable, while focus notifications are routed and non-cancelable.
Programmatic/automation/accessibility/controller, keyboard, and pointer sources
retain internally consistent derivation. `EventContext` exposes the
original/current/optional-related target, origin, accepted `WorkSequence`,
`MonotonicInstant`, optional pointer identity, independent physical hit
target/path, and propagation/default facts. For semantic-origin command
callbacks, `SemanticCommandEvent::semantic_action_target()` exposes the optional
read-only `SemanticActionTarget`; ordinary and delegated commands carry none.
Semantic activation default receives the same exact metadata separately through
`WidgetActivationContext::semantic_action_target()`. The context provisionally
collects owned actions, delegated commands, exact-owner subscription invalidation,
ordinary invalidation, mounted tasks/timers/cancellation, stop propagation, and
prevent default plus ordered capture/release requests for the current pointer.
Recursive mapping preserves every staged capture request. `WidgetEventOutput`
reports only independent persistent-state mutation. Mapping moves non-`Clone`
actions and recursively maps mounted work while preserving commands, controls,
invalidation, semantic contribution, and the state-change fact. Only the checked
erased widget bridge constructs and extracts `EventContext`; runtime supplies its
validated facts and output bound. Public origin constructors are direct-only,
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
removed without an alias. Built-in Text, Button, and Container author canonical
semantic contribution rather than `WidgetSemanticProof`.

`Element::map_action` replaces only action plumbing. It recursively delegates
every state-aware capability and preserves underlying widget/state type IDs and
semantic contribution content. Compatible reconciliation installs the newly
authored description, mapper, and activation factory while retaining state and
mounted identity. No global `Action: Clone`, `Send`, `Sync`, or `'static` bound
exists.

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
For semantics specifically, an erased state/widget payload mismatch withdraws
all owner semantic lifetimes and marks mounted integrity failure; authored
semantic contribution invalidity instead withdraws owner semantics without
misclassifying mounted state as corrupt.

## Identity and targeting

Core-owned `MountedNodeId` privately stores one exact mounted identity under the
shared runtime namespace:

```text
opaque runtime namespace + checked u32 mounted-arena slot + u64 generation
```

It is `Clone`, `Debug`, `Eq`, and `Hash`, but not `Copy`. Equality and hashing
include namespace identity, slot, and generation. There is no global counter,
random ID, serialization, or preorder identity.

`SemanticNodeId` is a distinct opaque type using the same runtime namespace but
a separate runtime-owned semantic arena slot/generation. It is not coupled to or
reconstructed from the mounted arena address. A mounted owner may own zero, one,
or many live semantic IDs. Runtime binds each live semantic ID to the exact
mounted owner lifetime plus one stable owner-local `SemanticKey`.

Compatible mounted-owner retention plus the same semantic key preserves the
exact semantic ID even when the contribution order changes. Removing a semantic
key revokes only that semantic lifetime; removing/replacing the mounted owner
revokes all of its semantic lifetimes. Reusing a vacated semantic arena slot
advances generation, so old IDs remain stale and never retarget replacements.
The shared generational arena preflights capacity and generation retirement; the
semantic store validates existing bindings before mutation and rejects duplicate,
foreign, missing, or owner/key-mismatched index state instead of choosing first
or last. Capacity failure is fail-closed for the complete owner semantic set and
does not partially mutate arena/bindings.

Both mounted and semantic IDs are process-local and runtime-instance-local.
Runtime namespace validation occurs before slot interpretation. Foreign semantic
IDs, stale generations, missing addresses, checked public-slot overflow, and
generation exhaustion do not truncate or wrap. Ordinary downstream code cannot
construct live IDs or extract/reconstruct their private parts. M5B publishes
live semantic IDs only inside the independent semantic snapshot/update product;
no conversion exposes the private mounted owner behind a semantic ID.

Authored `ElementId` remains a validated lookup/diagnostic handle. It does not
affect mounted reconciliation compatibility and may change while mounted
identity survives. M5B relationship composition resolves exact owner-local
`SemanticKey` references and cross-owner targets of unique authored `ElementId`
plus optional semantic key; missing, hidden, stale, or ambiguous targets
produce deterministic diagnostics without first/last fallback. M4C1 command
submission accepts only an exact mounted target. M4C5 automation resolves exactly
one authored ID before that command ingress. M5C resolves exact public semantic
action requests through the private semantic owner/key mapping and never exposes
that mapping as public mounted routing authority.

## Reconciliation

The root is compatible only when its key, widget type ID, and state type ID
match and it has no recorded integrity failure. `None` root keys compare equal.

For children, keys are sibling-local. A unique keyed child matches a unique old
sibling with the same key and compatible widget/state types regardless of
position. Keyed reorder preserves mounted IDs, state, interaction slots, focus,
and clean caches. A key change, type incompatibility, or cross-parent placement
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

Semantic identity reconciliation is independent from mounted sibling matching.
After a valid widget contribution is produced, the runtime reconciles its
validated ordered semantic keys against the mounted owner's existing semantic
bindings. Surviving owner/key pairs retain IDs regardless of contribution order;
removed keys are revoked; additions receive semantic-arena identities. A direct
mounted-child structural change invalidates semantic structure so marker
validity is recomputed. Unrelated compatible layout/paint/diagnostic changes do
not requery unchanged semantic contribution. M5B then composes finalized owner
facts through publication-local indexes into the independent semantic product.

## Lifecycle and shutdown

Initial mount and compatible update run parent-before-child in new authored
preorder. Each preserved node receives exactly one checked update from the new
description before that description is committed. Removal revokes every
exact-owner producer token, queued completion payload, source, mapper, timer,
registry record, and semantic lifetime children-before-parent before the
corresponding unmount hook begins. Each node remains mounted-arena-live through
its hook; arena removal, stale mounted identity, and state drop follow the hook.
Replacement fully unmounts and drops the old subtree before mounting the new
subtree.

State drops after its unmount hook. Interaction slots, caches, and semantic
owner bindings disappear with the mounted lifetime. Explicit
`AppRuntime::shutdown`, `into_state`, and `Drop` share one idempotent postorder
`RuntimeShutdown` authority; every remaining node receives exactly one shutdown
unmount.

## Routed commands, focus, and interaction slots

`AppRuntime::submit_command` appends one non-reentrant exact-mounted semantic-
command envelope. Accepted `submit_semantic_action` work joins this same command
FIFO after exact semantic admission. At the queue front runtime revalidates the
appropriate exact target authority, snapshots one owned root-to-target route,
validates every route node and erased event bridge, and admits the complete
configured transaction boundary before the first callback. Capture visits root-
to-parent, target runs once, and bubble visits parent-to-root. Stopping
propagation affects only later callbacks; preventing default affects only the
cancelable semantic default.

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

For ordinary exact-mounted unprevented `Activate`, runtime re-queries the
original mounted target after callback invalidation. Only a still-live enabled/
actionable target invokes the existing widget activation capability exactly once
as semantic default. M5C semantic-origin `Activate` retains the accepted semantic
target metadata and performs its separate exact semantic post-callback
revalidation before default; PRIMARY and named readiness rules remain distinct as
described above. Semantic `RequestFocus` likewise revalidates before committing
focus. Callback-invalidated semantic defaults are trace-distinct from explicit
prevention and never retarget.

Prevented activation never invokes its factory. `CancelOrBack`, `OpenMenu`, and
`OpenContextMenu` route once and have no default action, runtime mutation, or
second ancestor pass. Programmatic, automation, accessibility-stub, and
normalized-controller origins use the exact-mounted path. M4C5 automation
resolves exactly one authored ID before command ingress. M5C accessibility-origin
semantic requests resolve an exact current semantic ID privately and then use the
same command/routed/default/update/reconciliation authority without a second
queue or dispatcher.

Direct programmatic activation, direct focus mutation/traversal helpers, the
transitional `FocusTargetResult`, and the old pointer/keyboard
activation/resolution helpers are removed. Focus changes enter through canonical
command defaults; keyboard modality follows accepted raw keyboard ingress, not a
public `CommandOrigin::keyboard()` constructor. M4C2 owns surface context, M4C3
implements pointer lifecycle/release-inside activation, M4C4 implements focus
scopes/modality, M4C5 implements keyboard/text/composition and automation
resolution, M4D1 implements normalized in-memory trace reconstruction, M4D2
implements deterministic export and bounded subordinate sink delivery, M4D3
implements accepted offline replay plus final migration/closure proofs, M5A
implements semantic contribution/identity, M5B implements semantic publication,
and M5C implements semantic accessibility action resolution.

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
M5C semantic `RequestFocus` first resolves and admits the exact current semantic
PRIMARY, then converges on this same focus default under the accepted
`WorkSequence`.

Committed transitions update focus and focus-within atomically, then route
non-cancelable `FocusOut` before `FocusIn`, each Capture/Target/Bubble, before
initiating routed/default outputs. `FocusEvent` exposes kind, reason, and exact
target; `EventContext::related_target` exposes the opposite live endpoint.
Removal/replacement suppresses post-unmount delivery, clears incompatible
memory, and records the exact cleanup reason. Shutdown clears focus and memory
with `FocusReason::Shutdown` while retaining the last accepted modality. M5B
captures focus at routed transaction start and, after successful commit, dirties
only the semantic product when final focus changed; the next publication then
projects the exact visible PRIMARY without re-entering semantic contribution.

Pointer interaction state is runtime-owned per checked `PointerId`: device and
surface ownership, physical path, buttons, pressed owner/inside state, and one
exact live capture owner remain distinct. `submit_pointer` is the sole public
pointer ingress and never accepts an unchecked mounted target. Down/move/up/
cancel/wheel use the canonical queue and routed transaction engine. Primary
activation requires an eligible down and physical release inside the same exact
live owner; wheel derives one route-only logical-scroll command.

## Keyboard, committed text, composition, and automation

The owner-accepted M4C5 implementation exposes `AppRuntime::submit_keyboard`,
`submit_text`, `start_composition`, `submit_composition_update`,
`submit_composition_end`, `cancel_composition`, and
`submit_automation_command`. Each returns a runtime-issued receipt on admission
or a structured error that retains the exact unaccepted input or automation
request. A rejected `start_composition` instead returns
`SubmitCompositionStartError` with a `CompositionStartRequest`: it has no
generation because generations exist only in successful
`CompositionStartSubmission` receipts. The public APIs are host-neutral and
require callers to pump; they do not synchronously dispatch callbacks or
actions.

`KeyboardEvent` separates physical and logical key identity and carries phase,
modifiers, repeat, location, composition state, and optional device identity.
It binds to the exact focused lifetime and routes Capture/Target/Bubble. An
eligible non-repeated Enter down atomically reserves and appends one canonical
`Activate`; a repeated down never duplicates it. Space down records exact
focused ownership and a matching live eligible Space up atomically reserves and
appends `Activate`. These possible semantic defaults are admitted before routed
callbacks, and their derived command acceptance retains causal trace lineage.
An unmatched Space up never clears another device or lifetime's ownership;
explicit focus/lifetime/eligibility cleanup remains the other revocation path.

`CommittedTextEvent` is nonempty Unicode input, not a `LogicalKey::Character`
shortcut. It routes only to a focused `WidgetTextInput` that opts into committed
text and has no editable-text default. That capability separately opts into
composition. `start_composition` allocates an opaque runtime-local generation
only after focused-capability and mandatory-admission checks. Update, end, and
explicit cancellation require that exact generation; the pending-to-active
lifetime rejects foreign, missing, stale, invalid-range, duplicate-close, and
replacement attempts without retargeting.

Composition cancellation routes while the exact owner is live and precedes
`FocusOut` on focus transfer. Removal/replacement, disablement, text-capability
loss, shutdown, and drop also cancel before unmount. If required cleanup cannot
be routed or committed because integrity/admission is no longer available, the
runtime records causal suppression, retires the exact composition lifetime
without falsely claiming callback delivery, terminalizes, and preserves the
terminal-to-shutdown causal chain. No composition operation performs editable-
text mutation or an implicit committed-text default. Default trace policy stores
only metrics/ranges and no raw committed text or preedit; exact payload capture
requires explicit independent `TracePayloadCapture::FullText` configuration.

Automation resolves a unique authored ID in logical preorder and submits the
ordinary semantic command with automation origin. Missing IDs return a
structured outcome; ambiguous IDs return stable logical-preorder positions and
opaque mounted identities without widget state or user input. Neither submits a
command, and a target made stale between resolution and processing is rejected
without fallback. Public automation work-sequence or enabled-trace-sequence
exhaustion returns the exact authored request without terminalizing or mutating
the runtime. The old first-match lookup, direct activation path, and
compatibility aliases are gone.

## C9 public authority delta

| Old contract | New contract | Reason | Accepted ADR rule | Downstream migration | Proof owner |
|---|---|---|---|---|---|
| Send-subscription startup could accept provisionally | `Starting` submissions return `SendSubscriptionSinkError::NotStarted(exact_item)`; only `Running` accepts | Success must mean durable ownership | ADR 0006 producer admission | Match `NotStarted` and recover with `into_item` | `subscription_scheduler::send_subscription_start_outcomes_are_once_only_reclaimed_and_explicitly_retryable` |
| Cancelled send-task completion could enter ingress | `SendTaskCompletionError::Stale(exact_completion)` | Producer validity is exact-generation, not global | ADR 0006 cancellation | Match `Stale` separately from `Closed` | `scheduler_work::cancelled_send_completion_never_invokes_ui_mapper` |
| Direct mounted activation was public runtime authority | `submit_command(exact_target, Activate, origin)` is the canonical exact-mounted semantic ingress; M5C exact-semantic requests converge into the same command path after private semantic admission | Every source must use routing, admission, default, FIFO, and trace | ADR 0005 canonical commands | Submit and pump; semantic clients use `submit_semantic_action` and recover exact requests on rejection | `routed_commands` plus M5C semantic conformance |
| `Widget::activate` returned `Option<Action>` | It returns `WidgetActivationOutput<Action>` and is invoked only by routed `Activate` default | State mutation is independent from action output | ADR 0003 widget protocol; ADR 0004 mounted state; ADR 0005 default | Return `none`, `action`, `changed`, or `changed_with_action` | `mounted_work_output::routed_activation_separates_scheduler_wake_from_redraw` |
| Rejected composition start could imply a generation | `SubmitCompositionStartError` recovers a generation-free `CompositionStartRequest`; only accepted `CompositionStartSubmission` carries a runtime-issued generation | Rejected ingress must not fabricate accepted lifetime authority | ADR 0005 exact ingress ownership | Match the start error and recover its request; retain generations only from successful receipts | `input_m4c5` composition-start rejection proofs |
| Public authored-ID automation reused ordinary terminal sequence-exhaustion policy | Automation work/trace-sequence exhaustion returns the exact authored request without terminalizing; direct commands and accepted work retain ordinary terminal policy | Resolution is provisional until the canonical command is admitted | ADR 0005 canonical command convergence and rejection non-mutation | Recover `AutomationRequest` and retry only under new capacity | `automation_rejection` sequence-exhaustion proofs |

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
| `INTERACTION` | activation and text-input capability | focus/input validation, interaction, semantics, paint |
| `LAYOUT` | measurement, child layout | layout, hit testing, semantic bounds, paint placement |
| `PAINT` | paint | paint output |
| `SEMANTICS` | semantic contribution | semantic output |
| `DIAGNOSTICS` | widget diagnostics | diagnostic output |
| `ALL` | all capabilities | all dependent phases |

Ordinary capability caches retain unresolved, ready, or payload-mismatch state.
The semantic contribution cache additionally distinguishes authored
`Invalid(SemanticContributionError)`, semantic `IdentityExhausted`, semantic
index-integrity failure, and state-payload mismatch so failure meaning is not
collapsed. Valid contribution is structurally validated before runtime semantic
identity reconciliation. Invalid contribution withdraws the complete owner
semantic set; identity capacity failure is preflighted/fail-closed; index
corruption withdraws the owner and marks mounted integrity failure. Semantic
owner/key reconciliation cannot partially consume bindings on failed preflight.

`INTERACTION` alone does not clear paint or semantic widget facts. Structural
and common authored changes add runtime-detected phase work; they do not
automatically requery unrelated clean capabilities. Direct mounted-child
structural change invalidates semantic structure conservatively; child-count
changes also change `SemanticContributionContext`. Unrelated compatible updates
do not. M5B layout-only bounds changes dirty the semantic publication without
requerying unchanged widget contribution. Runtime focus changes likewise dirty
the semantic focus/product only; they do not invalidate or re-enter owner
semantic contribution. M5C semantic admission rejects while semantic authority
is dirty rather than synchronously refreshing it; layout-only dirtiness remains
admissible because current semantic binding/support/state authority is unchanged.

Publication-context changes compare root constraints, exact style-token content,
and measurement-provider identity/revision. Providers must change identity or
revision whenever behavior changes. Runtime-owned proof publication products and
an inspectable `SurfacePhaseReport` allow clean tree/style/layout/hit-test/paint/
semantics/diagnostics branches to be genuinely skipped while preserving clean
widget caches. Topology snapshots retain structural/alignment metadata only.
Style and layout execution use current mounted authored values, and
reconciliation—not the context key—detects authored token-reference or gap
changes. The report lists actual executed phase functions; private test-only
counters are incremented at the phase entry points and are independent from
report recording. M3 does not claim a production retained layout cache.

## Reports and publication

Initial mount completes reconciliation generation 1. Each successfully processed
application action increments once using `checked_add`; the action processor and
routed transaction admission preflight relevant exhaustion before application,
widget, mounted, factory, focus, cache, or reconciliation-report mutation.
Processing-rejection, routed-integrity, or terminal trace facts remain permitted
when trace sequencing is available; submission-time rejection never allocates
one.

`ReconciliationReport` defines counts by mounted lifetime: live nodes after
completion, new lifetimes mounted, preserved nodes updated once, lifetimes
ended, same-parent preserved moves, retained-focus truth, and structured
deterministic diagnostics containing complete duplicate-key old/new occurrences.
Semantic identity is not counted as mounted lifetime. The separate semantic arena
and exact owner/key bindings feed M5B publication without changing mounted
reconciliation counts.

`AppRuntime::publish_surface(&mut self, context)` is the only public surface
publication authority and is fallible. A successful call returns one
`SurfacePublication` with a fresh opaque `SurfaceInputContext` naming the one
logical surface, a fresh coordinate revision, the exact displayed hit-test
generation, renderer products, the semantic publication, and its semantic
diagnostic report. Recoverable publication backpressure returns
`PublishSurfaceError::Full` before publication commit; closed/terminal runtimes
return their exact status, and exhausted publication counters preserve their
exact terminal subreason. Candidate-dependent semantic revision/update and
mandatory sibling products are resolved before the final mounted/publication
commit boundary. The runtime retains a configurable nonzero bounded `VecDeque`
of immutable hit-test snapshots; oldest retirement is deterministic, and
retained contexts never re-hit-test current geometry.

`MountedTreeIndex`, `SurfaceFrame`, `SurfaceStyleReport`, and
`SurfaceLayoutReport` expose aligned mounted ID, parent, authored-ID, and current
preorder sequences for renderer/layout inspection. Renderer products no longer
carry semantic contribution or runtime semantic authority: `SurfaceNode` has no
semantic accessor, renderer debug output omits semantics, and renderer/cache
topology is not an alternate semantic tree. The independent `SemanticSnapshot`
has its own roots/order/index, stable opaque `SemanticNodeId`s, absolute semantic
bounds, runtime focus, composed state/support/relationships, revision, and
optional exact previous-revision delta. `SemanticDiagnosticReport` is a separate
mandatory sibling and does not advance semantic revision by itself.

`SurfacePublication` equality compares renderer products plus semantic
publication and semantic diagnostics; displayed `input_context()` identity is
compared explicitly when required. `renderer_products_eq` and
`into_renderer_products` are explicit renderer-only observations.
`into_complete_products` retains the input context, renderer products, semantic
publication, and semantic diagnostic report so complete-product consumption
cannot silently omit semantics. A tree change rebuilds aligned renderer products
from one current topology snapshot; semantic composition independently consumes
the staged topology/layout/finalized semantic-owner facts and runtime focus,
never `SurfaceFrame`. The free transient publication function is removed.

## Bounded canonical trace

`TraceConfig::new(capacity)` configures retained records, defaults
`TracePayloadCapture` to `Redacted`, and leaves the subordinate external sink
disabled. Capacity zero disables retention without allocating trace sequences,
constructing a sink, capturing raw text/preedit, invoking an application label
hook, or changing runtime behavior. The configured trace and sink capacities are
logical limits rather than eager allocation requests.

`AppRuntime::trace()` borrows the one canonical `Trace`, whose `records()` and
`kinds()` iterators run oldest-to-newest without a duplicate store. `TraceRecord`
accessors expose a non-forgeable `TraceSequence`, structured kind, optional
`WorkSequence`, optional causal-parent `TraceSequence`, optional reconciliation
generations before/after, optional `TraceTarget`, optional `TraceWorkIdentity`,
and optional `TraceSinkDeliveryOutcome`. Work identity exposes only read-only
owner, family, exact private generation value, and optional authored `WorkKey`;
it is not a runtime capability. Action payloads are never stored. Optional static
application labels come from `UiApp::trace_action_label(&Action)` and are retained
on `TraceActionIdentity` without adding an `Action: Debug` bound; the hook is not
invoked when tracing is disabled.

Scheduler records link the application transaction, work request, generation
commit, start attempt/outcome, completion/firing/cancellation, and final action
using causal parents and the actual accepted envelope `WorkSequence` where one
exists. M4C1 event records also expose logical instant, immutable original target,
callback current target, and command origin. M4C2 surface ingress adds structured
context acceptance, current-versus-retained snapshot selection, displayed
generation/revision, exact target binding, and rejection facts. For accepted
surface commands the chain is `SurfaceContextAccepted -> SurfaceTargetBound ->
CommandSubmissionAccepted -> RoutedEventStarted`; exact mandatory admission
reserves the three-record surface prefix plus the future routed outcome.
Acceptance causally parents route start and snapshot; phase, control, state,
invalidation, output collection, default, and commit records form the routed
chain. Collected actions and delegated commands parent their later accepted
envelopes and transactions. Submission and processing rejection are distinct by
observation: submission rejection has no canonical record and consumes no trace
identity, while processing rejection after acceptance is recorded. Routed
integrity failures classify broken topology, event-bridge mismatch, callback-
bridge failure, output-allowance overflow, semantic-default failure, or commit-
invariant failure without losing accepted causal facts. M4D1 normalizes this
canonical in-memory graph across scheduler, routed, surface, pointer, focus/
modality, keyboard/text/composition/automation, application-action, terminal/
cancellation/shutdown, logical-time, and publication facts.

M5C extends this same canonical graph rather than adding a semantic side channel.
Accepted semantic work records `SemanticActionBound` with exact semantic target
metadata before ordinary command acceptance/routing. Accepted work that fails
queue-front exact semantic revalidation records `SemanticActionProcessingRejected`
under the accepted `WorkSequence`. Explicit default prevention records
`SemanticDefaultSuppressed`; post-callback semantic authority invalidation records
`SemanticDefaultTargetInvalidated`. The export schema remains version 1 and
replay remains observational/inert.

M4D2 adds deterministic versioned JSONL projection and the subordinate bounded
sink without introducing a second trace/history/order authority. `Trace::export_jsonl()`
projects the retained snapshot with stable schema/version fields, fixed field
ordering, explicit symbolic tokens, exact JSON escaping, deterministic trace-only
runtime-local identity tokens, and the exclusive dropped-prefix watermark.
Default-redacted committed text and composition preedit retain metrics and checked
ranges only. `TracePayloadCapture::FullText` is the independent explicit opt-in to
exact payload retention; it is unrelated to debug formatting, action labels, and
sink enablement.

`TraceConfig::with_sink_capacity(NonZeroUsize)` enables one framework-owned
subordinate sink. `AppRuntime::take_trace_sink_receiver()` transfers its receiver
at most once. Runtime-side delivery first reserves lazy atomic logical capacity,
retains the canonical immutable record, then transports an `Arc<TraceRecord>` on
an asynchronous channel without waiting for receiver capacity. JSON encoding
occurs only in `TraceSinkReceiver::try_recv()`, outside mutable runtime
transactions. `TraceJsonlLine` exposes the encoded object, while
`TraceSinkReceiveError::{Empty, Closed}` describes nonblocking receive outcomes.
`Delivered`, `Full`, and first `Closed` attach to the same canonical record and
consume no second trace sequence. `Full` loses only the external copy; first
`Closed` retires sink authority, so no later record is sent through that closed
path. An open sink receives the canonical `RuntimeShutdown` attempt before sender
closure. The public sink surface exposes no arbitrary callback or runtime-work
submission capability.

## Offline trace replay

M4D3 adds `TraceReplay::parse_jsonl(&str)` as an accepted inert offline consumer
of the serialized JSONL v1 projection. The manual M4D2 encoder remains protocol
authority; replay does not consume live `TraceRecord` values and does not receive
runtime, queue, host, callback, scheduler, or mutation capabilities.

A replay document must begin with the `runenui.trace` version-1 header and then
contain `runenui.trace.record` version-1 objects. The parser validates required
field shapes, non-zero trace/work sequence identities, exact retained-record
count, one contiguous retained canonical sequence segment, and strictly earlier
causal parents. An export with no dropped watermark is `Complete`; an export
with `dropped_before_sequence` is explicitly
`TraceReplayCompleteness::DroppedPrefix` and never claims full causal history.
Because retained records are contiguous from the declared watermark, every
causal parent at or above that watermark is retained and every earlier missing
parent is explained by the dropped prefix.

`TraceReplaySequence` and `TraceReplayWorkSequence` deliberately duplicate only
the serialized numeric observation in separate replay-only types. They expose
`get()` for diagnostics and reconstruction, but no conversion or constructor can
produce live `TraceSequence` or `WorkSequence` authority. `TraceReplayRecord`
exposes stable kind name, replay-only sequence/work sequence, causal parent,
reconciliation before/after generations, and optional logical instant. A Counter
proof destroys the live runtime, parses only its owned JSONL string, and then
reconstructs automation/focus, raw-keyboard derivation, routed default,
application action/update/reconciliation, redraw, and publication ancestry. A
structurally valid divergent projection fails that reconstruction instead of
silently claiming equivalent behavior.

This replay surface is accepted M4 headless causal-proof infrastructure. M5C
proves its semantic records survive deterministic export and replay as inert
observation. Replay still does not provide the M5D public test harness, semantic
query/action convenience model, or an application-specific expectation engine.

M4C3 adds pointer submission, ordered validation and stream resolution,
physical-path and boundary-bundle planning, default applied/suppressed,
interaction commit, capture/boundary notification, activation/logical-scroll
collection, stationary-publication re-hit, and terminal diagnosis-to-cleanup
facts. The accepted pointer `WorkSequence` and causal parents reconstruct the
slice-local lineage; M4D1 preserves and normalizes that lineage but does not own
missing earlier pointer parentage.

M4C4 adds focus command and scope-policy evaluation, directional candidate and
restoration outcomes, exact old/new focus targets and reasons, focus-within
changes, routed notification queue/suppression, retained modality, reconciliation
cleanup, and shutdown ordering. The accepted command `WorkSequence` and causal
parents reconstruct the slice-local focus/modality lineage; M4D1 preserves and
normalizes that lineage but does not own missing M4C4 parentage.

M4C5 adds accepted/processed keyboard, committed-text, composition, Space
cleanup, and automation-resolution facts to the same trace. Keyboard default
derivation, canonical derived-command acceptance, composition activation and
retirement, cleanup cancellation/retirement, and suppressed-delivery terminal
cleanup retain their work sequence and causal parent. M4D1 replaces the remaining
nullable/staged trace construction with role-typed input, automation, and action
contexts; retains redacted UTF-8 byte/Unicode scalar metrics and checked
composition byte/scalar ranges by default; records explicit cleanup delivery/
suppression and exact lifetime/device identity; classifies accepted application
actions by type/category without payload retention or a global `Action: Debug`
bound; and proves the full Counter/public terminal and publication
reconstruction. M4D2's explicit FullText policy may additionally retain exact
committed-text/preedit payload under that independent opt-in. The mandatory
admission proofs preserve capacity-zero behavior, oldest-first eviction,
exclusive watermark, and exact sequence exhaustion.

Transaction semantic request/invalidation records preserve callback collector
order independently from cleanup-before-start queue grouping. Final action
acceptance is recorded before queue append, and the accepted action trace record
is the causal parent of the application transaction that later processes it.

Oldest records are evicted at capacity. `dropped_before_sequence()` is an
exclusive watermark: `Some(S)` means every trace sequence less than `S` is no
longer retained. Ordinary eviction cannot affect application behavior. When
enabled mandatory trace sequencing cannot advance for direct commands or
already-accepted mutable work, the runtime becomes terminal before the pending
mutable callback and cancels queued work. The accepted M4D2 export/sink surface
remains the sole live projection/transport over the canonical in-memory
authority; accepted M4D3 replay consumes only its serialized output offline.
M5A–M5C introduce no second trace or behavior engine.

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
  combined input intent, pointer/keyboard activation helpers, old key phase/key
  vocabulary, optional-target keyboard events, and public
  `CommandOrigin::keyboard()`;
- direct focus mutation/traversal helpers, `FocusTargetResult`,
  `KeyboardFocusResult`, `handle_keyboard_focus`, and the transitional runtime
  policy module;
- first-match `MountedTreeIndex::node_by_authored_id` lookup authority;
- M2 `WidgetSemanticProof` and singular mounted/renderer-facing semantic-ID
  projection authority;
- renderer semantic contribution carriage/`SurfaceNode::semantics()` and
  ambiguous publication consuming aliases that could silently omit semantics;
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
- reconciliation generation/report vocabulary and one private reconciliation
  plan/apply authority;
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
  rejection recovery;
- host-neutral keyboard, committed-text, and composition protocol values;
  `WidgetTextInput`; canonical input ingress/owned recovery; opaque runtime-local
  composition generations; deterministic authored-ID automation resolution; and
  redacted input trace facts;
- M4D1 role-typed input/automation/action trace contexts, normalized causal
  reconstruction, and public non-`Debug` action identity facts;
- M4D2 `TracePayloadCapture`, explicit FullText opt-in, deterministic
  `Trace::export_jsonl()`, optional static action labels, `TraceSinkDeliveryOutcome`,
  and the one-time `TraceSinkReceiver`/`TraceJsonlLine` nonblocking subordinate
  sink surface;
- accepted M4D3 `TraceReplay`, `TraceReplayCompleteness`, `TraceReplayError`,
  `TraceReplayKind`, `TraceReplayRecord`, `TraceReplaySequence`, and
  `TraceReplayWorkSequence` for inert offline JSONL validation and causal
  reconstruction;
- M5A core-owned canonical `LogicalSize`/`LogicalRect` geometry and
  platform-neutral semantic authoring vocabulary (`SemanticKey`, role/value/text/
  state/action/relationship/bounds types, `SemanticItem`,
  `SemanticNodeContribution`, `SemanticContribution`, validation/error/context),
  plus runtime-owned independent semantic arena/binding reconciliation;
- M5B `PublishSurfaceError`/`SurfacePublicationCounter`, `SemanticRevision`,
  semantic node/state/relationship, snapshot/focus/update/update-result/
  publication types, typed semantic diagnostic report/reasons, staged atomic
  fallible publication, and mandatory semantic siblings in `SurfacePublication`,
  with explicit renderer-only versus complete-product APIs;
- M5C `SemanticActionTarget`, `SemanticActionRequest`, reuse of the existing
  `CommandSubmission` receipt, typed semantic submission errors with exact request
  recovery, `AppRuntime::submit_semantic_action`, private exact semantic-to-mounted
  binding resolution, queue-front/post-callback revalidation, and canonical
  semantic binding/processing/default trace outcomes.

M1 validated values, textual identity, typed configuration, arity-free
composition, protected generated products, and finite saturating geometry remain
in force. The accepted contract includes effects, subscriptions, tasks, timers,
host requests, all four readiness budgets, wake/redraw, M4C1 exact-target routed
semantic commands, M4C2 displayed-generation surface context, the M4C3
host-neutral pointer lifecycle, the owner-accepted M4C4 focus-scope/modality
protocol, the owner-accepted M4C5 keyboard/text/composition and authored-ID
automation implementation, the owner-accepted M4D1 normalized in-memory trace
schema, the owner-accepted M4D2 deterministic export/redaction/bounded-sink
surface, the owner-accepted M4D3 offline replay/M4 closure proof surface, M5A
semantic contribution/independent identity, M5B independent semantic
snapshot/update/diagnostic publication, runtime-derived semantic focus/absolute
bounds, relationship resolution, support composition, renderer-independent
publication cutover, and M5C exact semantic action ingress/accessibility
resolution through the canonical command/routed/default/trace architecture. M4
is complete and M5 is active. M5D public testing harness, AccessKit/native
accessibility, native host translation, production scrolling, editable text, and
platform IME objects remain later work.
