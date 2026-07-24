# Events, Effects, and Scheduling

> **Category: Target architecture**

This document is the canonical M4 overview. It separates current implementation
facts from the accepted implementation contracts in
[ADR 0005](../adr/0005-canonical-event-routing-and-commands.md) and
[ADR 0006](../adr/0006-effects-scheduling-and-trace-v2.md). The normative proof
inventory is the [M4 conformance matrix](m4-conformance-matrix.md), and the
accepted [M4C delivery charter](m4c-delivery-and-routed-transaction-charter.md)
owns implementation boundaries and sequence. Acceptance authorizes
implementation but does not make any target implemented; support exists only
after its public behavioral proofs pass. Volatile branch, head, blocker, and
next-action state is owned by the [work-tracking system](../work-tracking.md).

## Accepted delivery sequence

```text
M4B   deterministic scheduler and application work (complete and accepted)
M4C0  conformance ownership and decision closure (complete and accepted)
M4C1  routed semantic-command kernel (complete and accepted)
M4C2  displayed-generation surface context (complete and accepted)
M4C3  pointer lifecycle (complete, owner-accepted, and squash-merged)
M4C4  focus scopes and modality (proof package complete; owner acceptance/merge pending)
M4C5  keyboard, text, IME, automation, and M4C closure (blocked by M4C4)
M4D1  complete trace schema (blocked by M4C5)
M4D2  export and sink (blocked by M4D1)
M4D3  replay and milestone closure (blocked by M4D2)
```

M4C1, M4C2, and M4C3 are complete and owner-accepted. M4C3's accepted feature
head `01b7ae018abeaff8d316764afba5bc8cde074381` passed exact-head CI run
`29996101708` and was squash-merged in PR #15 as
`2fc165b9386f55c061d61232400375b13ad175bf`. M4C4 is implemented and its proof
package is complete on the active feature branch, but owner acceptance and merge
remain pending; M4C5–M4D3 remain blocked in sequence. M4B's
implemented live-only producer authority remains unchanged, and M4 is active
and incomplete.

## Current application-work and scheduler implementation

The current implementation provides one bounded runtime-owned generalized FIFO,
core-owned opaque non-wrapping work-sequence values, an explicit four-budget
pump, exact-target routed semantic commands, and one bounded canonical trace
sequence. Each processed
action completes mounted reconciliation and focus validation before the next
begins. Compatible nodes retain focus; stale and foreign mounted targets are
rejected; each node owns proof interaction slots; and routed callbacks/default
activation can mutate persistent widget state only after checked route-wide
admission. Retired direct pointer/focus helpers are absent; semantic focus work
uses the same canonical command queue and routed transaction.

Core-owned initial/update effects, local/send tasks, deterministic timers,
application and mounted complete-set subscriptions, keyed generational
cancellation, typed host requests, configured limits, completion ingress,
race-free wake, revisioned redraw, lifecycle cancellation, and scheduler trace
facts are implemented. Scheduler callbacks preflight queue/work/trace capacity;
send work becomes running only after executor acceptance; repeating-deadline
overflow terminates only that timer after its current valid firing; and
work-specific trace records carry opaque exact owner/family/generation/key
identity. Capture/target/bubble routing, propagation/default control, delegated
commands, routed output mapping, and the command causal trace are implemented.
Displayed-generation surface input context and exact current/historical target
binding, pointer identity/capture/release-inside behavior, and focus scopes with
retained modality are implemented. There is no raw keyboard/text/IME stream,
authored-ID automation resolution, semantic accessibility mapping, trace
sink/export/replay, or complete trace-v2 normalization.

## Canonical target path

```text
Host event or synthetic command
  -> sequenced ingress envelope
  -> normalized event family/source/surface context
  -> target resolution and generational validation
  -> immutable mounted route snapshot
  -> capture / target / bubble
  -> semantic default behavior
  -> staged interaction commit
  -> commit-derived notifications
  -> sequenced actions/commands/mounted work
  -> update
  -> transient root rebuild and mounted reconciliation
  -> subscription reconciliation and committed effects
  -> wake/redraw scheduling and trace
```

The mounted tree remains the sole interaction and lifecycle authority. A routed
event cannot reconcile the tree underneath its route. Widget callbacks may
mutate only runtime-owned local state and collect requests. Application state
changes only through queued typed actions and `UiApp::update`.

## Event and command contract

M4C1 implements one intentionally narrow host-neutral protocol. `runenui_core`
owns `EventPhase::{Capture, Target, Bubble}`, the four direct
`EventSource` classes, direct/delegated `CommandDerivation`,
`CommandOrigin`, `SemanticCommand::{Activate, CancelOrBack, OpenMenu, OpenContextMenu}`,
`UiEvent::SemanticCommand`, `SemanticCommandEvent`, `WidgetEventOutput`, and
the borrowed `EventContext`. M4C2 separately adds core-owned surface identity
and input-context values without adding pointer, focus, keyboard, text, IME,
modality, scrolling, or platform-controller event placeholders.
M4C4 extends that neutral protocol with normalized keyboard origin, focus
commands, `FocusEvent`, scope/policy/reason/modality values, and no raw keyboard
or controller platform type.
Public `CommandOrigin` constructors create direct origins only. Delegated
derivation can be created only when the checked event bridge extracts a command
collected through `EventContext::emit_command`; external submission cannot
forge it. `UiEvent::as_semantic_command` returns an optional borrowed semantic
payload so later event variants do not invalidate callers.

One public `submit_command(exact_mounted_target, command, origin)` ingress
validates foreign/stale/missing/live target status and returns either a
runtime-issued sequence or exact owned recovery. At processing time the runtime
revalidates, snapshots one immutable owned root-to-target route, preflights every
checked event bridge, and admits the whole bounded transaction before invoking
capture root-to-parent, target once, and bubble parent-to-root. The original
target and accepted logical time/sequence remain immutable while phase and
current target change.

Submission-time full, closed, terminal, foreign, stale, missing, work-sequence-
exhausted, and trace-sequence-exhausted rejection consumes no work sequence, no
trace sequence, no trace record, and no wake authority. Accepted work that later
finds a stale or missing target is a distinct processing rejection recorded in
the accepted command's causal trace.

`EventContext` exposes phase, original/current/optional-related target,
origin, sequence, instant, cancelability, and current propagation/default state.
It provisionally collects owned typed actions, delegated commands, ordinary
invalidation, one coalesced exact-owner subscription invalidation, owner-local
tasks/timers/cancellation, stop propagation, and prevent default. It exposes no
application state, runtime, arena, host protocol, surface/pointer/focus/
composition state, queue, registry, or clock mutation. `WidgetEventOutput`
reports only explicit persistent-state mutation.

Routed admission is conservative maximum-safe preflight: before any mutable
callback, it reserves the complete route plus the configured aggregate output
allowance across every callback-accessible family, queue/work/generation/
reconciliation authority, and mandatory trace allowance. A callback that would
emit nothing can therefore still reject when a family required by that maximum
boundary is unavailable. This is deliberate M4C1 policy, not exact prediction
of callback output.

After acceptance, integrity trace records distinguish broken topology, event-
bridge mismatch, callback-bridge failure, output-allowance overflow, semantic-
default failure, and commit-invariant failure while retaining the accepted work
sequence, causal parent, targets, instant, and origin. Route-wide bridge
validation happens before the first callback; failures never report partial
commit as success.

Stopping propagation suppresses only later callbacks. Preventing default
suppresses only semantic default. Routed actions and commands preserve exact
interleaving and move non-`Clone` actions. Delegation targets the current node,
preserves source, changes derivation, appends a later envelope, and never recurses.

Unprevented `Activate` re-queries the original target after callback
invalidation, then invokes its still-live enabled/actionable activation
capability exactly once. Prevented, disabled, non-actionable, stale, missing, and
foreign targets do not invoke the factory. `CancelOrBack`, `OpenMenu`, and
`OpenContextMenu` are route-only with no default action, runtime mutation,
or second ancestor pass.

Programmatic, exact-target automation, exact-target accessibility-stub, and
normalized-controller origins converge on that same queue/route/default/update/
reconciliation/trace path. M4C1 does not resolve authored automation IDs,
semantic accessibility identities, or raw controller types.

The semantic authored callback remains `on_activate`. The old direct runtime,
pointer-press, pointer-focus, unchecked pointer-target, direct focus traversal,
and keyboard-focus helper authorities are removed. Normalized keyboard modality
uses a neutral command origin without adding raw keyboard routing.

M4C2 added displayed-generation surface context. M4C3 added the canonical
pointer/device protocol, pointer streams, physical/routed separation, capture,
boundaries, terminal cleanup, logical-scroll intent, and release-inside
activation. M4C4 adds the single focus/scope authority, modality, current-
publication directional selection, atomic focus transitions, and routed focus
notifications. The accepted later contract remains unimplemented: M4C5
keyboard/text/IME and authored automation resolution, M4D trace normalization/
export/replay, and M5 semantic accessibility mapping. See [ADR 0005](../adr/0005-canonical-event-routing-and-commands.md)
for those later behavioral rules.

## Application, effect, and subscription contract

The application lifecycle has three explicit output authorities:

- `initial_effects` describes one-time effects after successful initial
  mount/reconciliation;
- two-argument `update` mutates state and returns optional ordered effects;
- `subscriptions` declares desired state-derived ongoing streams after initial
  mount and every successful action/reconciliation.

`UiApp` has one associated `HostProtocol`. `()` is the no-effects result for
`initial_effects` and update. Simple applications therefore keep the
compact `update(state, action)` shape without an unused collector parameter,
while advanced applications return/build an ordered effect batch.

After initial reconciliation succeeds, the runtime commits one atomic initial
plan covering all mounted declaration owners, initial effects, application
subscriptions evaluated from current state, and mounted mount output. Queue order
is mounted declaration reconciliation in mounted preorder, initial effects in
collector order, application subscription starts in declaration order, then
mounted mount output in mounted preorder and collector order. Aggregate output,
queue, generation, family, and mandatory-trace admission covers the complete
plan; rejection consumes no partial sequence or generation and starts no work.

Eligible mounted callbacks receive only a restricted exact-owner work output for
actions, tasks, timers, and same-owner keyed cancellation. Mounted subscriptions
use a separate state-derived widget declaration capability whose invocation is a
complete desired set. Event and compatible-update contexts may provisionally
dirty that declaration for their exact owner, but cannot imperatively mutate a
registry or declare a partial set. Activation commits that invalidation together
with its primary action and auxiliary outputs through the same plan. Reusable
widgets do not become generic over
application host policy. Unmount remains cancellation-only.

Requests never execute inside update or widget callbacks. They append only after
the owning event or update/reconciliation transaction commits. An effect starts
only when its sequenced envelope reaches the queue front and its application or
exact mounted-generation owner/key remains valid.

## Work identity, tasks, timers, subscriptions, and host requests

Runtime-private generational IDs provide stale-completion safety. Application
and mounted code use validated owner-local `WorkKey` values for keyed
cancellation/replacement; private runtime IDs are never durable app identity.
Every committed keyed start allocates a new private generation. Cancellation
binds at transaction commit to the matching generation visible in the
transaction-local work view, and a queued cancellation carries that expected
generation. Processing it can never affect a later replacement. Collector order
therefore fixes cancel/start, start/cancel, start/replacement, and duplicate-
cancel outcomes without exposing private IDs to application state.

Local tasks and subscriptions may produce `Action` directly on the UI thread.
Send-capable work transports only concrete sendable payloads; after owner/token
validation, a retained UI-thread mapper converts payload to `Action`. Background
execution therefore does not impose `Action: Send` on the application.

Subscriptions retain only when owner, key, source type, and source-defined
configuration identity agree. Application declarations run after each successful
action/reconciliation against the current post-update state; complete
declaration values are never cached. The initial mounted declaration is part of
the atomic post-reconciliation initial plan; later declarations run only after
explicit owner-local invalidation from
compatible update, routed event, or another documented lifecycle seam. Unrelated
events do not re-declare them. A mounted declaration is evaluated only at the
front of its exact-owner reconciliation envelope, sees newest live mounted
state, and is suppressed when that owner is stale.
Equal declarations retain, changed declarations replace, absence cancels,
duplicates retain no ambiguous stream, and owner removal invalidates before
unmount completion. Arbitrary closures are never compared. Timers use
a monotonic host clock and deterministic manual headless clock; equal deadlines
order by creation sequence and repeating timers coalesce missed periods by
default.

Application host requests use one closed application-defined command/response
protocol whose `ResponseKind` is `Copy + Eq + 'static` and whose
`expected_response(command)`/`response_kind(response)` functions provide exact
validation. Each request retains command, expected kind, application owner,
private generation, and UI-thread mapper. A mismatched token, owner, or response
kind is diagnosed and never reaches that mapper. Only concrete cross-thread
methods add `Send` bounds, and widgets cannot issue application host commands.
Framework clipboard/cursor/IME/window/accessibility services remain separate M10
host capabilities.

One lock-protected state machine owns each live response generation. Registration
inserts `Open`; successful detached acceptance changes `Open -> DetachedQueued`;
successful direct acceptance changes `Open -> DirectClaimed`. Full detached
submission leaves `Open` for exact retry. Cancellation, replacement, owner
revocation, completion, terminal closure, and shutdown remove the generation's
retained response slot and any queued payload. No `Cancelled` tombstone is
retained. A missing slot is stale authority, and competing or late completion
paths are stale.

Removal, replacement, keyed cancellation, and shutdown invalidate work tokens
before cooperative executor cancellation. Late completions are stale and never
invoke UI-thread mappers.

A send-task start envelope makes exactly one validated executor attempt with a
`Started`, `Unavailable`, `Full`, `Closed`, or `Rejected` result. Refusal is
recoverable but terminal for that generation: no hidden retry/pending queue, no
poisoning, and no action unless the effect explicitly supplied a UI-thread start-
failure mapper. Retry is a new effect and generation; refused owned payloads are
returned for deterministic runtime-side handling when the adapter permits.

A send-subscription start envelope likewise makes one nonblocking attempt with
`Started`, `Unavailable`, `Full`, `Closed`, or `Rejected`. Refusal reclaims that
generation and never retries implicitly. During `Starting`, submission returns
`NotStarted` with the exact item. Only `Running` may accept an item successfully;
otherwise the sink returns `Full`, `Closed`, or `Stale` with exact ownership.

## Queue, failure, and saturation contract

One UI-thread-owned non-reentrant FIFO accepts events, commands, actions, timer
firings, commit-derived notifications, subscription reconciliation, and effect
starts.
Every accepted envelope goes to the queue tail; no source category overtakes work
already accepted. Each application action updates state and reconciles the
mounted tree before its outputs become visible.

One application transaction plan assigns accepted sequences in this order:
exact cancellation cleanup, mounted subscription reconciliation, update outputs
in collector order, application subscription starts in declaration order, then
mounted lifecycle outputs. Routed command commit uses the same planner for
mounted work while preserving subscription reconciliation, routed output,
semantic-default output, then mounted-work order.
Ready callback and mapper results allocate only their final application-action
envelope; no action-producing path depends on a second unreserved sequence.

`RoutedTransactionAdmissionPlan` uses checked arithmetic to reserve the
configured output ledger, queue slots/work sequences, reconciliation/work
generations, local/send/timer capacity, subscription reconciliation envelopes,
and mandatory trace records before the first event callback. Each routed action,
delegated command, default output, mounted effect/cancellation, and unique
exact-owner subscription invalidation spends one allowance; state-change facts,
ordinary invalidation, and flow control do not. A zero allowance rejects before
callback. Known refusal records the exact bounded authority; unexpected
post-mutation failure poisons instead of silently dropping output. Every accepted
external queue commit requests the coalesced wake edge, while publication-affecting
invalidation independently requests redraw.

A readiness checkpoint runs before the first envelope, after each processed
envelope while budget remains, and before quiescence. It imports cross-thread
completions in transport order, promotes timers by deadline/creation sequence,
polls each eligible local task or local subscription source at most once in
creation order, accepts ready
outputs in creation order, and assigns each accepted result a new UI-thread
global sequence at the queue tail. Producers never assign runtime sequences and
checkpoint results never execute recursively.

Processed envelopes, completion imports, local-work polls, and timer promotions
have separate budgets. Exhaustion preserves work/order, re-arms wake, and returns
non-quiescent progress. Quiescence additionally requires no imported completion,
due timer, immediately ready eligible local task under the allowed check, or
mandatory derived work; a future timer contributes only its next deadline.

M4 defines configured limits for queued envelopes, transaction outputs, live
work classes, and trace. External/full/closed outcomes are explicit. Cross-thread
senders retain unaccepted payloads. Typed actions and completions are never
silently dropped. Send tasks, send subscriptions, and host responses retain
live-only per-generation producer authority. Send-subscription ingress is
`Starting` during `start` and `Running` only after `Started`; a startup send
returns exact `NotStarted`. Cancellation, replacement, unmount, completion,
terminal closure, and shutdown centrally remove the generation and every
retained payload, source, and mapper.

Recoverable stale/full/closed outcomes leave the runtime coherent. If an
unexpected integrity failure occurs after unrollbackable application mutation,
the runtime enters a terminal poisoned state: no provisional external work
starts, all work is invalidated, and only inspection, trace export, state
extraction, and shutdown remain available.

## Wake and redraw

Wake and redraw use independent state machines. Wake has an explicit
request/acknowledge/re-arm handshake: the host clears the outstanding wake before
pumping, racing completions can request another wake, and the runtime rechecks
remaining work after the pump. This prevents lost wakeups without owning a native
event loop. Request state, installed transport, and delivery ownership share one
state mutex. A separate callback-in-flight fact serializes delivery without a
lock-held host boundary. The runtime claims an eligible request under the state
mutex, releases every RunenUI synchronization guard, and only then invokes the
transport. A request arriving while a callback is in flight remains undelivered
until that callback returns, then is claimed and delivered once. Transport
replacement cannot reclaim an already delivered request.

Close prevents every later delivery claim, removes the installed transport, and
returns without waiting for a callback claimed earlier. Such a callback may
finish after close; its completion clears in-flight bookkeeping but cannot
re-arm or reopen the closed state. Deterministic state-transition and blocking-
callback tests own this contract; repeated concurrent races are supplementary.

Redraw has a separate take/acknowledge generation. Invalidation racing with
publication leaves a new redraw armed. Wake does not imply redraw, and redraw
does not execute rendering inside a wake callback.

## Trace v2

The bounded trace foundation replaces the duplicate vectors with one canonical
record sequence. Its records carry monotonic trace and actual accepted work sequences,
causal linkage, reconciliation generations, routed targets, queue/
transaction/focus facts, saturation, terminal cancellation, and shutdown.
Scheduler work facts additionally expose a read-only opaque identity containing
the application or exact mounted owner, family, private generation value, and
optional authored key. M4C1 adds command acceptance, accepted-work processing
rejection, submission-rejection trace absence/non-consumption, route snapshot,
phase targets, propagation/default controls, state/invalidation/output facts,
semantic default, commit/poison/admission outcome, and parentage into later
actions and delegated commands. M4C2 adds surface-context acceptance,
selection, binding, rejection, and causal-parent facts. Complete trace v2 later
normalizes the full schema and adds deterministic export and replay.

Transaction semantic request/invalidation facts preserve callback collector
order separately from cleanup-before-start queue grouping. Mandatory trace
admission uses a checked operation-specific plan. Detached host completion
requires four records; send-task and send-subscription completion require three.
Capacity zero disables retention and allocation without changing scheduler
behavior. The accepted final action trace fact is recorded before append and causally parents the later
application transaction that processes that envelope.

Capacity is configurable. Dropping old records advances an explicit watermark.
Text/IME redaction, versioned JSONL projection, external sinks, and replay remain
blocked M4D scope. The canonical in-memory trace is the sole current per-command
outcome authority; `PumpReport` remains aggregate.

## Ownership boundaries

- `runenui_core` owns host-neutral authored/downstream protocols, including the
  opaque mounted identity values and later surface ID value types, events/commands,
  action mapping, `EventContext`, `UiApp`, `HostProtocol`, `WorkKey`, and
  effect/subscription descriptions. Hidden construction seams contain no live
  state authority.
- `runenui_runtime` owns routing, mounted interaction state, current proof focus,
  sequenced work allocation, effects, scheduler, clock/limits, lifecycle cancellation,
  wake/redraw state, poisoning, and trace.
- hosts own raw platform events, native device/controller mapping, monotonic
  clock/deadline/wake integration, optional send-capable execution, and typed
  application host-request completion.
- renderers own no event, application, task, timer, or host-command semantics.
- applications own durable state, typed actions,
  update/`initial_effects`/subscription logic,
  owner-local work keys, and their closed host protocol.
- reusable widgets emit actions or restricted runtime work and declare mounted
  subscriptions through the dedicated complete-set capability; later framework
  platform services use explicit M10 capabilities rather than app commands.

No ECS, Runenwerk, native window, GPU, renderer, controller library, or
platform-event dependency belongs in the neutral M4 runtime.

## M4 implementation gate

ADR 0005 and ADR 0006 were accepted by the repository owner on 2026-07-14. ADR
0005 remains routed-behavior authority, ADR 0006 scheduler-behavior authority,
the accepted M4C delivery charter is implementation/delivery authority, and the
[M4 conformance matrix](m4-conformance-matrix.md) is observable acceptance
authority.

M4C1, M4C2, and M4C3 are complete and owner-accepted. M4C4 is implemented and
its proof package is complete on the active feature branch, but owner acceptance
and merge remain pending. M4C5–M4D3 remain blocked in sequence, and M4 remains
active and incomplete.

M4 does not implement a platform host, accessibility tree/adapter, editable text
control, production renderer scene, production layout/style, broad control
library, or multi-surface windowing.
