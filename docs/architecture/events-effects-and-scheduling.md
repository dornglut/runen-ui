# Events, Effects, and Scheduling

> **Category: Target architecture**

This document is the canonical M4 overview. It separates current implementation
facts from the accepted implementation contracts in
[ADR 0005](../adr/0005-canonical-event-routing-and-commands.md) and
[ADR 0006](../adr/0006-effects-scheduling-and-trace-v2.md). The normative proof
inventory is the [M4 conformance matrix](m4-conformance-matrix.md). Acceptance
authorizes implementation but does not make any target implemented; support
exists only after its public behavioral proofs pass.

## Current application-work and scheduler implementation

The current implementation provides one bounded runtime-owned generalized FIFO,
non-wrapping work sequences, an explicit four-budget pump, queue-backed
proof activation, and one bounded canonical trace sequence. Each processed
action completes mounted reconciliation and focus validation before the next
begins. Compatible nodes retain focus; stale and foreign mounted targets are
rejected; each node owns proof interaction slots; and activation can mutate
persistent widget state only after queue/generation/trace preflight. The proof
input helpers still provide only typed pointer/keyboard vocabulary, linear
traversal focus, rectangle targeting, and press-based button activation.

Core-owned initial/update effects, local/send tasks, deterministic timers,
application and mounted complete-set subscriptions, keyed generational
cancellation, typed host requests, configured limits, completion ingress,
race-free wake, revisioned redraw, lifecycle cancellation, and scheduler trace
facts are implemented. Scheduler callbacks preflight queue/work/trace capacity;
send work becomes running only after executor acceptance; repeating-deadline
overflow terminates only that timer after its current valid firing; and
work-specific trace records carry opaque exact owner/family/generation/key
identity. There is no routed propagation, surface-input generation
context, pointer identity/true capture, release-inside policy, text/IME stream,
trace sink/export/replay, or complete trace-v2 normalization.

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

The event model uses separate pointer, keyboard, committed-text, IME
composition, semantic-command, pointer-boundary, focus-transition, and capture
notification families. Keyboard events do not pretend to be text input.

Every publication exposes an opaque runtime-issued `SurfaceInputContext`, not
merely a position plus unchecked mounted target. It identifies the runtime
namespace, logical `SurfaceId`, coordinate-space revision, and exact displayed
hit-test generation. The bounded snapshot ring retains the current and
immediately previous generations by default and may be configured larger. Every
retained generation is interpreted against its exact snapshot; retired,
foreign-runtime, foreign-surface, and missing generations have distinct
rejection outcomes and are never retargeted through current geometry. Hosts map
platform coordinates to RunenUI logical coordinates for the supplied context
without exposing pixel/DPI/window types in the neutral protocol.

A routed callback can inspect phase, original target, current target, related
target where relevant, source/modality, logical sequence/time, surface-input
context, and pointer physical-hit facts. Propagation control and default behavior
are independent. Callback invalidation is re-applied before same-transaction
default behavior reads capabilities.

Pointer events carry stable stream identity and optional device identity. The
runtime owns per-pointer capture, retains the physical hit path separately from
the captured route target, and releases capture on explicit release, up,
cancellation, owner removal/replacement, and shutdown.

Pointer ingress expands deterministic leave/enter notifications before the
ordinary pointer event. The runtime also retains hover-capable pointer positions
and re-hit-tests them when authoritative hit-test geometry changes, so a
stationary pointer cannot retain stale hover or release-inside state after layout
or visibility changes. Multiple retained pointers re-hit in registration order;
each pointer emits leave inner-to-outer and then enter outer-to-inner. The new
publication does not retarget an already accepted older-context transaction.

Pointer validation orders namespace, surface, active pointer ownership, snapshot
generation, then target. Foreign-runtime/surface events never mutate local
pointer state. A same-runtime/surface active pointer up with a retired or missing
snapshot is not routed or re-hit-tested and never activates; it records the
context rejection and performs causally traced integrity-only cancellation that
clears pressed/capture state and closes the stream. Same-runtime/surface pointer
cancel performs that cleanup without retained geometry. Non-terminal unavailable-
context input remains a pure rejection.

Pointer, keyboard, normalized controller/navigation, accessibility-stub,
automation, and programmatic activation converge on routed semantic commands.
Default pointer activation is press, capture, pressed-state tracking, and
activation only on a still-valid release inside the same mounted lifetime.
Release outside, cancellation, removal/replacement, or disablement does not
activate.

Focus movement and `Activate` have framework defaults. `CancelOrBack`, menu/
context-menu, and logical-scroll commands are route-only: once capture/target/
bubble finishes unconsumed, their exact default is no action or runtime mutation.
There is no second ancestor pass. Explicit callback/scope delegation emits a new
queued command/action and never recurses. An unprevented wheel emits exactly one
logical-scroll command; a prevented wheel emits none, and unconsumed scroll never
performs production scrolling in M4.

The semantic authored callback is `on_activate`. The M1–M3 `on_press` proof term
is removed without an alias because activation may originate from pointer
release, keyboard, controller, accessibility, automation, or programmatic APIs.

Focus navigation includes explicit scopes, deterministic next/previous
traversal, a beam/overlap-aware directional policy backed by the normative
[directional-focus corpus](m4-directional-focus-corpus.md),
and exact transition ordering. Composition cancellation precedes `FocusOut`,
which precedes `FocusIn`. IME composition is owned by the exact focused mounted
generation and cannot be retargeted after focus/lifetime change.

See [ADR 0005](../adr/0005-canonical-event-routing-and-commands.md) for surface
validation, routing facts, output order, default behavior, focus scope defaults,
composition lifetime, pointer capture, geometry-triggered boundary updates,
activation, and migration of overlapping proof paths.

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
ingress leaves `Open` for exact retry. Cancellation, replacement, owner
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
mounted lifecycle outputs. Activation uses the same authority with cleanup,
mounted subscription reconciliation, primary action, then auxiliary outputs.
Ready callback and mapper results allocate only their final application-action
envelope; no action-producing path depends on a second unreserved sequence.

Mutable activation reserves the complete configured callback allowance before
the callback: one reconciliation generation, `2 * transaction_outputs + 1`
queue slots, `transaction_outputs`
work generations and free slots in every mounted-accessible family, and
`4 * transaction_outputs + 1` mandatory trace records. A committed activation
reports the first accepted sequence, optional primary-action sequence, and total
queued envelope count. Auxiliary-only work is therefore `Queued`; explicit
widget-state mutation or a coalesced subscription invalidation is `Activated`;
only an authoritative empty `WidgetActivationOutput` with no context effect is
`NoEffect`. Saturation returns the exact `ActivationCapacity`. Every accepted
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
causal linkage, reconciliation generations, activation targets, queue/
transaction/focus facts, saturation, terminal cancellation, and shutdown.
Scheduler work facts additionally expose a read-only opaque identity containing
the application or exact mounted owner, family, private generation value, and
optional authored key. Complete trace v2 later adds routed-event and surface
facts, logical scheduler time, deterministic export, and replay.

Transaction semantic request/invalidation facts preserve callback collector
order separately from cleanup-before-start queue grouping. Mandatory trace
admission uses a checked operation-specific plan. Detached host completion
requires four records; send-task and send-subscription completion require three.
Capacity zero disables retention and allocation without changing scheduler
behavior. The accepted final action trace fact is recorded before append and causally parents the later
application transaction that processes that envelope.

Capacity is configurable. Dropping old records advances an explicit watermark.
Text and IME payloads are redacted by default. A versioned deterministic JSONL
projection and optional bounded/try-based sink provide the M5 testing/replay
foundation without requiring `Action: Debug` or authoritative wall-clock time.
The canonical in-memory trace remains authoritative. Sink full/closed/failure
can lose only the external copy, never blocks or changes runtime behavior, owns
no unbounded queue, and reports a structured canonical diagnostic. A recursion
guard records that diagnostic without sending it back through the same failing
delivery path.

## Ownership boundaries

- `runenui_core` owns host-neutral authored/downstream protocols, including the
  opaque mounted/surface ID value types, events/commands, action mapping,
  `EventContext`, `UiApp`, `HostProtocol`, `WorkKey`, and effect/subscription
  descriptions. Hidden construction seams contain no live state authority.
- `runenui_runtime` owns routing, mounted interaction state, focus/capture,
  sequenced work, effects, scheduler, clock/limits, lifecycle cancellation,
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

ADR 0005 and ADR 0006 were accepted by the repository owner on 2026-07-14 as the
M4 implementation charter. Their implementation-defining gaps are closed by the
revised decisions and the normative
[M4 conformance matrix](m4-conformance-matrix.md). M4 implementation may begin
only from updated `master` after this architecture pull request is merged.

Acceptance completes only the architecture gate. M4 remains incomplete until the
canonical authorities are implemented, obsolete paths are removed, every
required matrix row passes through public APIs, stable/MSRV validation succeeds,
and current status/support records are updated.

M4 does not implement a platform host, accessibility tree/adapter, editable text
control, production renderer scene, production layout/style, broad control
library, or multi-surface windowing.
