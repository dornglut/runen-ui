# ADR 0006: Deterministic Effects, Scheduling, and Trace v2

> **Category: ADR**
>
> **Status:** Accepted
>
> **Accepted by repository owner:** 2026-07-14
>
> **Decision date:** 2026-07-13
>
> **Milestone:** M4
>
> **Reviewed baseline:** `83e3771c34e021ac2960cab2cfd926c1128998ca`

## Context

The M3 runtime applies one typed action synchronously:

```text
action
  -> UiApp::update
  -> transient root
  -> mounted reconciliation
```

That proof has no action/work queue, initial application work, effect result,
task executor, timer, subscription declaration, host-command boundary,
deterministic clock, wake/redraw handshake, lifecycle cancellation, saturation
contract, or trace v2. Its trace stores the same coarse activity in two unbounded
vectors and has no sequence, causal, generation, effect, or redaction facts.

M4 must add real application work without weakening these foundations:

- durable state changes only through typed application actions and `update`;
- mounted widget state and lifecycle remain runtime-owned;
- actions may be non-`Clone`, non-`Send`, and non-`Debug`;
- mounted identity and work identity are generational and runtime-local;
- runtime mutation occurs on one logical UI thread;
- headless time and execution are deterministic;
- standalone and embedded hosts retain event-loop ownership;
- renderers own no tasks, clocks, commands, or application policy;
- mounted work ends with its exact mounted lifetime;
- no application effect escapes an uncommitted update/reconciliation boundary.

## Decision

### Crate and module ownership

The dependency direction remains one-way:

```text
runenui_runtime -> runenui_core
```

`runenui_core` owns the public, host-neutral protocol vocabulary required to
describe application work: `UiApp`, `HostProtocol`, `Effects`, `IntoEffects`,
`SubscriptionSet`, `WorkKey`, public effect/subscription descriptions, action
mapping, the widget subscription-declaration capability, owner-local
subscription invalidation requests, and transaction-scoped mounted-work
requests. It contains no executor, clock, queue, timer wheel, host transport,
runtime namespace, mounted arena, or live work registry.

`runenui_runtime` owns every live mechanism: the global queue and sequences,
readiness checkpoints, work generations and owners, executor integration,
timers and the monotonic clock, subscription reconciliation, host-request
validation, wake/redraw state, trace storage, lifecycle cancellation, and
shutdown. No third crate or parallel scheduling authority is introduced for M4.

### One non-reentrant sequenced work loop

`AppRuntime` owns one UI-thread work loop and one FIFO sequence of owned work
envelopes:

```text
external event or semantic command
programmatic application action
commit-derived event notification
application action
local/send task completion
subscription item
host completion
scheduled timer firing
committed effect start or cancellation
internal wake/redraw bookkeeping
```

Every accepted envelope receives a non-wrapping sequence on the UI thread and is
appended to the queue tail. There is no source-class priority. A callback cannot
recursively execute another event, action, update, effect, or completion; it can
only collect provisional output that is appended after the current transaction
commits.

Every pump runs a readiness checkpoint before its first envelope, after each
processed envelope while the processed-envelope budget remains, and immediately
before declaring quiescence. A checkpoint performs these steps in order:

```text
1. import cross-thread completions in transport order, up to the import budget
2. promote due timers ordered by (deadline, creation sequence)
3. poll each eligible local task at most once, in creation order, up to the poll budget
4. accept ready outputs in creation order
5. append each accepted result at the queue tail with a new global sequence
```

Cross-thread producers never allocate runtime sequence numbers. Ordering becomes
authoritative only when the UI thread accepts their payloads. A promoted timer,
ready local task, or imported completion is an envelope like every other source;
none executes recursively during the checkpoint.

After a checkpoint the pump pops exactly one head envelope, executes exactly one
transaction, atomically appends its committed outputs at the tail, increments
the processed-envelope count, and checkpoints again if that budget permits.
Newly appended work never executes as part of the transaction that produced it.

Four independently configurable budgets bound one pump: processed envelopes,
cross-thread completion imports, local-task polls, and due-timer promotions.
Exhausting any budget is progress, not an error or a drop: remaining order is
preserved, the runtime reports non-quiescent progress, and wake is re-armed.

The runtime is quiescent only when the queue is empty, no completion is waiting
to be imported, no timer is due, no eligible local task is immediately ready
under the permitted readiness check, and no mandatory derived work remains. A
future timer contributes only the returned next deadline; it does not by itself
make the runtime non-quiescent.

Headless tests drive the same pump explicitly. Standalone and embedded hosts call
it from their own integration. The runtime exposes progress, remaining work,
next logical deadline, dirty publication state, and quiescence rather than
hiding a native event loop.

### Global ordering and commit batches

Ordering is fixed when work is accepted. Newly emitted work never overtakes an
envelope already accepted.

ADR 0005 event transactions commit in this order:

1. interaction state changes become authoritative;
2. commit-derived capture/composition/focus notifications append;
3. one owner-local mounted-subscription reconciliation envelope appends for each
   exact owner whose provisional invalidation committed, coalesced per owner;
4. routed callback actions and commands append in emission order;
5. default-behavior actions and commands append after routed outputs;
6. mounted-owned task/timer and same-owner cancellation requests append last.

The subscription envelope is mandatory derived work. When it reaches the queue
head it validates the exact mounted owner, evaluates that owner's complete
declaration, commits the diff, and only appends any start envelopes; it never
starts subscription work recursively. Its position before the initiating
event's ordinary outputs ensures the declaration is evaluated before a later
output from that event can rely on it, while FIFO still prevents overtaking
older accepted work. A dirty owner also prevents quiescence until its declaration
has been evaluated or its lifetime has ended.

One application action transaction commits in this order:

1. preflight the mutable runtime authority and configured transaction allowance;
2. call `update` and collect provisional output;
3. build the transient root;
4. reconcile the mounted tree;
5. immediately invalidate dead/replaced owner and subscription tokens;
6. reconcile the application subscription declaration against the new state;
7. commit lifecycle-derived cancellation and interaction notifications;
8. append update outputs in their original order;
9. append new/replacement subscription starts in stable declaration order;
10. mark dirty publication and wake/redraw state.

Cancellation token invalidation is synchronous at commit and therefore precedes
all later work. Executor abort delivery may be queued/cooperative, but a late
completion cannot pass token validation after commit.

Commit-derived notifications append before update outputs so focus, capture,
composition, pointer-boundary, and unmount-related transitions caused by the new
mounted state are observable before a later output can act on the replacement
state. If an already queued older envelope references a removed generation, it
is rejected as stale when processed; FIFO ordering is never violated to rescue
it.

Source-specific deterministic order—equal-deadline timers, one local poll pass,
one route, or one subscription declaration—is resolved before append. The global
sequence is the final tie-break and trace authority.

### Application lifecycle work

Application work has three distinct authorities:

1. **Initial effects** are described once after the initial root mounts and
   reconciles successfully.
2. **Update effects** are returned from one action update and commit only after
   the resulting root reconciles successfully.
3. **Subscriptions** are a declarative function of current application state and
   are evaluated after initial mount and after every successful application
   action/reconciliation.

Conceptually the one application contract provides:

```rust
trait UiApp {
    type State;
    type Action;
    type HostProtocol: HostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action>;

    fn initial_effects(
        state: &Self::State,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        ()
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol>;

    fn subscriptions(
        state: &Self::State,
        subscriptions: &mut SubscriptionSet<Self::Action>,
    ) {
    }
}
```

The exact trait spelling may use associated output types instead of return-position
`impl Trait`, but these semantics are fixed:

- `initial_effects` defaults to no effects;
- `subscriptions` defaults to an empty declaration;
- `update` remains synchronous and owns durable-state mutation;
- `()` is a valid no-effects update result, preserving the simple two-argument
  update shape;
- one `Effects` batch and ordered composition implement `IntoEffects` without
  cloning actions;
- there is one canonical `UiApp` contract, not competing simple/advanced runtime
  authorities;
- collecting output never executes it;
- after successful initial mount/reconciliation, the runtime collects and commits
  `initial_effects`, evaluates subscriptions, appends initial effects in collector
  order, then appends subscription starts in declaration order before updating
  wake/redraw state;
- no initial work starts before initial reconciliation succeeds;
- bounds such as `Send`, `Sync`, and `'static` apply only to concrete work,
  payloads, mappers, or host transports that need them.

The M3 two-argument `update` call shape therefore remains ergonomic while its
result gains an optional ordered effects channel. Applications do not receive an
unused mutable collector parameter merely to express no effects.

### Provisional output and transaction integrity

Application and mounted callbacks collect provisional descriptions. They do not
start work, expose host commands, or allocate externally visible completion
authority before commit.

The runtime preflights every failure that can be known before invoking mutable
application code: reconciliation-generation capacity, owner validity, required
bridge integrity, internal mandatory-output allowance, and configured queue/
transaction capacity. Runtime-private generational work IDs are allocated when
provisional requests commit, not while application code is mutating state.

Application `update`, root construction, and widget callbacks are treated as
infallible application code; panics are unsupported. Reconciliation must either
complete from preflighted valid bridges or produce a recoverable replacement/
diagnostic defined by ADR 0004.

If an unexpected post-mutation integrity failure still prevents a coherent
commit—such as sequence/identity exhaustion, violated checked-erasure invariant,
or ignored transaction-output saturation—the runtime enters a terminal
`Poisoned` state. It starts no provisional external work, invalidates/cancels all
work tokens, refuses further callbacks or state mutation, and permits only
inspection, trace export, `into_state`, and shutdown. RunenUI does not pretend it
can roll back arbitrary application state mutation.

Sequence and generation counters never wrap. Their practical representation may
be wider than current counters, but exhaustion always follows the explicit
terminal policy rather than reusing identity.

### Runtime limits and saturation

M4 introduces a runtime limit configuration covering at least:

- queued work envelopes;
- provisional outputs per event/update transaction;
- live local/send tasks;
- live timers;
- live subscriptions;
- live host requests;
- canonical trace capacity;
- optional external trace-sink delivery capacity.

No application action or completion is silently dropped.

- External/programmatic ingress returns a structured `Full`/`Closed` result.
- Cross-thread completion senders return the unaccepted payload on `Full` or
  report disconnection so the producer controls retry/disposal.
- Internal mandatory transition capacity is reserved before a transaction starts.
- Fallible effect/output builders are `must_use`; exceeding their configured
  allowance marks the transaction failed. Continuing after application state
  mutation follows the terminal integrity policy above.
- Coalescible framework signals such as wake and redraw use their dedicated
  state machines rather than consuming unbounded duplicate queue entries.

M11 may tune production defaults and performance budgets, but the saturation
semantics and no-silent-drop rule are fixed in M4.

### Application effect output

An application effect batch can request, in order:

- another typed application action;
- a local task;
- a send-capable task;
- a one-shot or repeating timer;
- keyed cancellation or replacement;
- an application-specific typed host request;
- explicit wake/redraw-relevant invalidation where framework state does not
  derive it automatically.

A committed effect does not execute immediately. It becomes an effect-start or
cancellation envelope at the queue tail. When that envelope reaches the front,
the runtime revalidates the exact owner and key generation before starting or
cancelling work.

### Mounted work output

Mounted mount, compatible-update, activation, and event contexts may collect a
restricted exact-owner subset:

- typed application actions;
- owner-scoped local or send-capable tasks;
- owner-scoped timers;
- keyed cancellation of work owned by the same exact mounted generation.

They cannot imperatively add, remove, or partially redeclare subscriptions.
`EventContext` and the compatible-update lifecycle context may instead stage one
owner-local subscription invalidation request. That request commits or rolls
back with its transaction, starts no work, and schedules the dedicated complete-
set declaration pass below.

This output does **not** expose the application's arbitrary host protocol and
does not make reusable `Widget<Action>` implementations generic over application
platform policy. A widget needing application policy emits an action;
application `update` may then request the host.

Future framework-owned clipboard, cursor, IME, window, drag/drop, and
accessibility services require explicit M10 host capabilities rather than reuse
of an application's command enum. The read-only unmount context remains
cancellation-only and cannot start work. Renderers receive no effect output.

Mounted work requested during an event route is provisional until propagation,
default behavior, and staged interaction commit finish. If an earlier queued
action removes/replaces the owner before its effect-start envelope runs, start
is suppressed as stale without retargeting.

### Work keys, runtime identity, and explicit cancellation

Every executable item has a private non-forgeable generational runtime ID and
one owner:

- `Application`;
- `Mounted(MountedNodeId)`;
- a later surface owner only after real multi-surface lifecycle exists.

Runtime IDs distinguish foreign, stale, missing, cancelled, and live work. They
are not authored, serialized, or used as application durable identity.

Application and mounted code control restartable/cancelable work through an
optional validated owner-local `WorkKey` plus work family. The pair
`(owner, work kind, WorkKey)` is the declarative cancellation/replacement
identity:

- starting a keyed task, timer, or host request replaces the previous live item
  of the same owner/kind/key by invalidating it before the replacement starts;
- every committed keyed start allocates a new private runtime generation;
- cancellation binds at transaction commit to the matching generation visible
  in that transaction's provisional work view, and the queued cancellation
  carries that expected generation;
- processing a cancellation invalidates only its expected generation; an already
  completed, cancelled, or replaced generation is an idempotent stale no-op and
  a later generation is never affected;
- anonymous one-shot work has no application-visible handle and ends only by
  completion, owner lifetime, or shutdown;
- subscriptions always have an owner-local key and additional source/configuration
  identity.

This separates stable application intent from runtime completion safety. An
application that needs dynamic handles stores/generates its own `WorkKey` in
state; it never stores private arena IDs. `WorkKey` is a cloneable, hashable,
validated textual identifier using the repository's existing Unicode identifier
grammar; it is durable owner-local intent, not a private generation.

Collector order is normative when one committed batch addresses the same
`(owner, work family, WorkKey)`:

| Batch order | Required commit result |
|---|---|
| cancel, then start | Bind cancellation to the previous generation, then allocate the replacement generation. |
| start, then cancel | Allocate the new generation, then bind cancellation to that generation. |
| start, then replacement start | Invalidate the first start before the replacement becomes startable. |
| cancel, then cancel | Both bind consistently; the second is idempotent and cannot reach a newer generation. |

Application-owned work lasts until completion, keyed cancellation/replacement,
subscription absence, or runtime shutdown. Mounted-owned work lasts only while
the exact mounted generation remains live.

Before a mounted subtree unmounts or is replaced, the runtime:

1. marks descendant owners closing;
2. rejects new work for them;
3. invalidates completion tokens and applies keyed cancellation in deterministic
   descendant-before-ancestor order;
4. suppresses late completions from cancelled/stale generations;
5. runs existing postorder unmount hooks while mounted IDs remain live;
6. removes arena nodes and private work records.

Replacement cancels the old generation before mounting the replacement.
`into_state` and `Drop` cancel every remaining owner exactly once before state is
moved out. Shutdown delivers no new application action after extraction begins.

### Local and send-capable tasks

A task is one-shot work that produces zero or one application action. Normal
failures are application data, not an untyped global error channel.

M4 supports two paths:

- **Local task:** may capture non-`Send` state, is polled on the logical UI
  thread, and may produce `Action` directly.
- **Send-capable task:** the background future/work unit and its concrete output
  are `Send + 'static`; the runtime retains an owned UI-thread mapper from that
  output to `Action`. The executor never receives the application action type,
  app state, widget state, or runtime mutation authority.

A send-capable task therefore does not imply `Action: Send`. An application may
have non-`Send` action variants while consuming sendable background results on
the UI thread.

No executor belongs in `runenui_core`, no Tokio dependency is implicit, and no
thread pool is mandatory. The headless profile includes a deterministic local
executor. A host may provide an optional send-capable executor adapter.

Cancellation is cooperative: token invalidation is immediate, while executor
abort/drop may take effect at the next yield. A late payload with a cancelled or
stale token is discarded before its mapper runs. Mapper execution occurs only
on the UI thread after token/owner validation.

Task starts follow effect-start sequence. Ready local tasks in one deterministic
poll pass are accepted by creation sequence. Cross-thread completion order is
defined only when the UI thread accepts payloads into the queue; trace records
that accepted order rather than pretending thread finish time is deterministic.

When a committed send-task start envelope reaches the queue head, the runtime:

1. validates its exact owner, work generation, and key generation;
2. performs exactly one executor start attempt;
3. records one structured `Started`, `Unavailable`, `Full`, `Closed`, or
   `Rejected` outcome (exact enum spelling may differ);
4. transitions that work generation to running only for `Started`, otherwise to
   terminal immediately.

`Started` makes the task live; later completion still requires exact token and
owner validation. Every refusal is recoverable and non-poisoning, accepts no
later completion for that generation, creates no action by default, performs no
automatic retry, and owns no hidden executor-pending queue. Retry requires a new
application effect and therefore a new generation. Trace records the request,
committed start envelope, one attempt, and terminal outcome.

A send-task description may explicitly retain a UI-thread mapper from structured
start failure to `Action`. When present, after the failure transition commits,
the runtime invokes that mapper and appends exactly one resulting action through
the canonical queue; it never executes recursively and imposes no `Action: Send`
bound. Without that mapper the refusal remains diagnostic/trace information
only.

If an executor adapter can return the owned sendable work payload after refusal,
it returns that ownership to the runtime-side effect record for deterministic
drop or typed failure handling. Refusal never leaks, forgets, or silently retains
owned work.

### Timers and deterministic clock

Scheduling uses a monotonic clock, never wall time. The headless clock advances
only when a test requests it; hosts provide monotonic `now` and wake integration.

Timer IDs are owner-scoped and generational. Due timers order by:

1. deadline;
2. creation sequence.

A one-shot timer produces at most one action. A repeating timer advances from its
previous logical deadline rather than callback completion. The default missed-
tick policy coalesces missed periods into one firing and advances to the first
future deadline. Any catch-up mode must be explicit and bounded.

Key replacement or owner cancellation invalidates a pending timer before it can
map to an action.

### Declarative subscriptions

Application subscriptions are re-declared from state after initial mount and
every successful action/reconciliation.

Mounted subscriptions have one separate state-derived declaration authority in
the open widget protocol, conceptually:

```rust
fn subscriptions(
    &self,
    state: &Self::State,
    output: &mut SubscriptionSet<Action>,
);
```

The object-safe checked-erasure spelling may differ, but one invocation declares
the complete desired subscription set for that exact mounted generation. A
widget cannot access or mutate the live registry, and ordinary mount/update/
event work output cannot represent a partial desired set.

`runenui_core` owns this public declaration protocol and its subscription
description types. `runenui_runtime` owns declaration evaluation, identity and
reconciliation, private generations, execution, cancellation, completion
validation, and lifecycle enforcement.

The runtime evaluates a mounted declaration only:

1. after the owner's mount transaction commits successfully;
2. after a compatible state update whose lifecycle context committed
   subscription invalidation;
3. after a routed event transaction whose `EventContext` committed owner-local
   subscription invalidation;
4. after another explicitly documented lifecycle seam changes subscription
   state.

Unrelated events do not re-evaluate or implicitly cancel mounted subscriptions.
Invalidation is provisional, owner-local, and coalesced for the exact generation.
It schedules one later declaration evaluation before quiescence and, by the
commit ordering above, before a later output from the invalidating transaction
can rely on the declaration. Evaluation commits a diff; actual starts remain
sequenced start envelopes and cannot run before the mount/update/event commit.

A subscription identity includes:

- exact owner generation;
- owner-local validated `WorkKey`;
- process-local source type identity;
- source-defined deterministic configuration identity/revision sufficient to
  distinguish materially different streams.

The runtime never compares arbitrary closures or opaque source objects. A source
must provide stable configuration identity to retain across declaration passes;
without it, the source is replaced conservatively.

An identical identity remains running. A changed identity invalidates/cancels the
old generation before the replacement starts. Absence from the complete desired
set cancels it. Duplicate owner-local keys are diagnosed and preserve no
ambiguous stream. Declaration order is stable and determines equal-priority
start order.

A local subscription may map items to `Action` on the UI thread. A send-capable
source emits `Item: Send + 'static`; after owner/token validation, a retained
UI-thread mapper converts the item to `Action`. The source/executor never needs
`Action: Send`. Late items after replacement, cancellation, or unmount are stale
and never invoke the mapper.

Owner removal invalidates every mounted subscription generation before the
owner's unmount callback completes. Downstream widgets use the same public
declaration capability and checked bridge as built-ins; there is no privileged
registry seam.

### Application host protocol

Application host requests are application-owned effects, not renderer work and
not callable from widget callbacks.

M4 chooses one closed application-defined host protocol per `UiApp`: a command
type, a response type, and response-kind discriminator:

```rust
trait HostProtocol {
    type Command;
    type Response;
    type ResponseKind: Copy + Eq + 'static;

    fn expected_response(command: &Self::Command) -> Self::ResponseKind;
    fn response_kind(response: &Self::Response) -> Self::ResponseKind;
}
```

Each committed request records the command, its expected response kind, the
application owner, a private request generation, and an owned UI-thread mapper.
A host later returns a response, application-level failure, or cancellation
against the opaque request token. Only after token generation, owner, and exact
response kind validate may the runtime invoke the mapper and append an action.

A mismatched response is a structured host-protocol diagnostic and never reaches
application update or a newer request generation. Concrete response payloads
may cross threads when their types satisfy required bounds; `Action` itself need
not be `Send`. Headless tests inspect and complete requests through deterministic
stubs.

No global `Send` bound is placed on the protocol. Bounds are applied by concrete
cross-thread submission methods only when a particular command or response
crosses threads. M4 does not use an unchecked string channel or force reusable
widgets to understand application commands. Widget contexts cannot issue the
application host protocol. Later standard platform services use explicit
RunenUI host capabilities at M10.

### Wake and redraw handshake

Wake and redraw are framework scheduling states, not arbitrary application
commands:

- **wake:** queued, due, or potentially ready runtime work requires host service
  before another unrelated host event;
- **redraw:** one or more surface outputs are dirty and should be published at an
  appropriate host frame opportunity.

They are independent and idempotently coalesced. Wake does not imply redraw;
redraw does not render inside a wake callback.

Wake uses an explicit race-free handshake shared with completion senders:

1. a producer that may make work ready atomically transitions wake state from
   `Idle` to `Requested` and invokes the host wake transport only on that edge;
2. when servicing the wake, the UI thread acknowledges/clears `Requested`
   **before** pumping;
3. completions racing after that clear observe `Idle` and request another wake;
4. after the pump, the runtime rechecks queue/readiness/deadline state and
   re-arms a wake if work remains;
5. shutdown closes the wake handle so later producers receive `Closed`.

This prevents a completion from being stranded between host acknowledgment and
pump completion. The host transport may be a native user event, channel, callback,
or embedded-loop signal; the runtime owns only the state machine.

Redraw has a separate take/acknowledge state. Publication clears the dirty request
for the generation it consumed; invalidation racing with publication leaves a
new redraw request armed.

A host provides narrow capabilities:

- monotonic clock and next-deadline integration;
- request-one-wake transport and acknowledgment;
- optional send-capable task execution;
- delivery/completion of the application host protocol;
- explicit pump and dirty-surface publication calls.

The neutral runtime owns no native event loop and assumes no specific framework
such as winit. External senders cannot mutate runtime or application state; they
submit typed payloads/tokens that the UI thread validates, sequences, and maps.

### Failure and integrity policy

Checked-erasure failures, foreign/stale tokens, payload/response mismatches,
queue saturation, sequence exhaustion, and executor contract violations are
structured runtime diagnostics. They never retarget a newer generation. An
ordinary `Unavailable`/`Full`/`Closed`/`Rejected` executor start outcome is the
recoverable terminal result defined above, not an executor contract violation
and not a poisoned-runtime condition.

Normal task/host failures are typed application data. Panic recovery remains
unsupported: the neutral runtime does not attempt partial unwind recovery from
application update, root construction, widget callbacks, task polling,
subscription sources, mappers, or host adapters.

Recoverable stale/full/closed outcomes leave the runtime coherent. Violations
that occur after unrollbackable application mutation use the terminal poisoned
policy defined above.

### Trace v2

M4 replaces the duplicate current vectors with one canonical bounded
`VecDeque<TraceRecord>` and projections/iterators over it. There is no
compatibility store.

`TraceConfig` defines canonical capacity and payload redaction. Capacity zero
disables retention without changing behavior. At capacity, the oldest record is
dropped and an explicit `dropped_before_sequence` watermark advances. Separate
optional sink configuration fixes its bounded delivery capacity or try-delivery
policy; enabling a sink never changes canonical retention.

Every record contains where applicable:

- non-wrapping trace and transaction/work sequences;
- causal parent sequence;
- reconciliation generation before/after;
- mounted target, surface-input context, or work owner;
- pointer/task/timer/subscription/host-request identity;
- structured record kind and logical scheduler time.

Record kinds cover at least ingress acceptance/rejection, event normalization,
route/phase/default outcomes, focus/modality/pointer/capture/composition
transitions, commands, action queue/start/update, reconciliation, invalidation,
queue saturation, wake/redraw request/acknowledgment, effect request/commit/start/
completion/cancellation, timers, subscriptions, host requests, publication,
send-executor start outcomes, trace-sink diagnostics, integrity failure,
poisoning, and shutdown.

Trace never requires `Action: Debug`. It stores type/category and causal identity;
an application may opt into a redacted label provider. Text commit and IME
payloads are redacted by default. Full text capture requires explicit
configuration and is never enabled by a generic debug flag.

Trace time is logical monotonic time. External sinks may add wall-clock metadata
only as non-authoritative context. The runtime exposes borrowed iteration,
deterministic versioned JSONL projection, and an optional subordinate external
sink. The canonical bounded in-memory trace remains the only ordering authority.

Sink delivery is bounded or try-based, never blocks inside a mutable runtime
transaction, owns no unbounded internal queue, cannot recursively submit runtime
work, never changes canonical trace ordering, and closes during shutdown. Full,
closed, and other delivery failures produce structured sink diagnostics. Only
the external copy may be lost; the canonical retained record and
application/runtime behavior are unchanged.

The sink-failure path has an explicit recursion guard: where canonical capacity
permits, the runtime records the delivery failure in the canonical trace, but
does not send that diagnostic back through the failing sink during the same
failure path. A full/closed/failing sink therefore cannot recursively generate
unbounded diagnostics or delivery attempts.

Trace v2 must reconstruct:

```text
input/command
  -> normalization and route/default
  -> interaction transition commit
  -> global work order
  -> update and reconciliation generation
  -> subscription diff
  -> effect start/completion/cancellation
  -> wake/redraw request and acknowledgment
  -> publication or terminal failure
```

M5 builds the public testing and stronger replay surface on this foundation.

## Implementation migration and proofs

M4 implementation must remove or replace:

- immediate recursive `Runtime::dispatch` as the public processing authority;
- overlapping direct input/focus/activation paths;
- duplicate `Trace.events` and `Trace.records` storage;
- unbounded trace retention;
- effect execution inside callbacks/update;
- work without exact application/mounted ownership;
- application work with no initial/subscription authority;
- string-only or runtime-ID-as-durable cancellation conventions;
- renderer- or event-loop-specific wake assumptions;
- wake coalescing without acknowledgment/re-arm semantics.

Existing `UiApp` implementations, Counter, and the external-widget conformance
package migrate together without a compatibility runtime. The application API
keeps the simple two-argument update shape with `()` as the no-effects result.

The normative proof matrix is
[`../architecture/m4-conformance-matrix.md`](../architecture/m4-conformance-matrix.md).
It includes initial work, state-derived subscriptions, keyed replacement,
dedicated mounted complete-set declarations/invalidation, keyed replacement,
transaction ordering, non-`Send` actions, local/send tasks, terminal executor
start outcomes and optional failure mapping, timer order, lifecycle cancellation,
host mismatch rejection, saturation, wake races, redraw races, bounded trace-sink
backpressure/recursion, trace causality, and idempotent shutdown.

## Consequences

Applications gain deterministic work without allowing executors or hosts to
mutate state outside `update`. Mounted lifetimes become the cancellation
authority promised by M3. Initial effects and subscriptions no longer depend on
a synthetic first action. Simple applications retain a compact update shape,
while advanced applications gain ordered effects.

The application trait, widget contexts, runtime internals, input API, and trace
API change incompatibly. That is intentional before 1.0 and avoids preserving
the current immediate-dispatch/duplicate-trace proof as a second authority.

No new crate is justified by this ADR alone. Effects, scheduler, clock, limits,
wake state, and trace begin in `runenui_runtime`; extraction requires real
ownership, dependency, or independent-consumer pressure.

M4 selects no Tokio, async-std, winit, AccessKit, renderer, native host, or
production text stack.

## Rejected alternatives

- **Execute effects inside update/callbacks:** external work could escape an
  uncommitted transaction.
- **Recursive dispatch:** ordering, stack growth, and reconciliation boundaries
  become implicit.
- **Source-class priority:** later work could overtake accepted work.
- **Require an effects collector parameter in every update:** simple applications
  would pay permanent API noise; a no-effects return preserves one authority and
  the original two-argument shape.
- **No initial/subscription callback:** startup and state-derived streams would
  require fake actions or imperative duplication.
- **Global `Action: Clone + Send + Sync + 'static`:** local UI actions do not need
  those bounds.
- **Send `Action` through background executors:** one non-`Send` variant would
  unnecessarily disable all background work.
- **Expose runtime generational IDs as application cancellation handles:**
  durable application intent must use owner-local keys; runtime IDs remain
  private stale-completion authority.
- **Make widgets generic over application host commands:** reusable widget
  behavior would depend on app-specific policy.
- **Require a multithreaded executor or bake in Tokio/winit:** headless and
  embedded single-threaded profiles are required.
- **Compare arbitrary closures for subscription retention:** equality is
  undefined; sources provide explicit configuration identity or are replaced.
- **Imperative unkeyed subscriptions:** rebuilds would duplicate streams and
  ownership would be ambiguous.
- **String-only cancellation identity:** stale completion safety requires
  generational runtime identity behind validated keys.
- **Wall-clock ordering:** deterministic tests require monotonic controllable
  time.
- **Wake flag without acknowledge/re-arm handshake:** a completion can be lost
  in the clear/pump race.
- **Silent queue/output dropping:** typed actions and completions require explicit
  saturation outcomes.
- **Renderer-owned tasks/redraw policy:** rendering consumes output and does not
  own application work.
- **Unbounded/duplicated trace or mandatory `Action: Debug`:** these leak memory,
  diverge, add global bounds, and can expose sensitive data.
- **Deliver late completion to a replacement owner:** ownership ends with the
  exact generation.

## Research basis

The design adapts rather than copies these primary sources:

- [iced `Task`](https://docs.rs/iced/latest/iced/struct.Task.html) for
  runtime-executed work producing typed messages;
- [iced `Subscription`](https://docs.rs/iced/latest/iced/struct.Subscription.html)
  for state-derived desired streams with identity-based lifetime;
- [futures local spawning](https://docs.rs/futures/latest/futures/task/trait.LocalSpawn.html)
  for local non-`Send` execution and cooperative cancellation boundaries;
- [winit `EventLoopProxy`](https://docs.rs/winit/latest/winit/event_loop/struct.EventLoopProxy.html)
  as evidence that wake transport remains a host adapter while RunenUI defines
  its own race-free request/acknowledgment state;
- M3 [ADR 0004](0004-mounted-runtime-reconciliation.md) for generational
  lifetime, unmount ordering, and checked erasure.
