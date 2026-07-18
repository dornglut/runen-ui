# Vocabulary

> **Category: Current contract**

This vocabulary marks current and target terms explicitly. Target terms do not imply an implemented API.

## Current terms

| Term | Meaning |
|---|---|
| State | Application-owned durable data used to derive UI. |
| Action | Application-owned typed intent passed to `update`. |
| `update` | Application function that mutates state in response to one action. |
| `View<Action>` | Public conversion protocol for a typed transient authored value that produces one erased element. |
| `Element<Action>` | Owned transient erased node derived from state; it is consumed as reconciliation input and never retained as parallel runtime authority. |
| `Text`, `Button<Action>` | Typed built-in authored views that transfer common facts into elements and install private behavior-only widgets. |
| `Container<Action>` | Canonical typed authored view for any built-in or downstream `ChildLayoutWidget`; owns children and container-only gap before atomic erasure. |
| `Widget<Action>` | Concrete state-aware runtime participant declaring persistent state, lifecycle, activation, measurement, paint, semantic, and diagnostic proof behavior. |
| `ChildLayoutWidget<Action>` | Widget contract required for child ownership; contributes one `ChildLayout` policy independently of intrinsic measurement. |
| `ChildLayout` | Non-exhaustive M2 child arrangement proof; currently linear by axis, with descendant-preserving vertical fallback for future unknown variants. |
| Component | Ordinary Rust view composition that may use a local action and recursively map it into a parent action; it is not automatically mounted identity or state. |
| `element!` / `children!` | Thin `View` erasure and arity-free heterogeneous child collection; no parallel property grammar. |
| `LogicalLength` | Finite, non-negative device-independent distance; host scale factors later map logical to physical pixels. |
| `ElementId` | Unicode-validated optional authored debug/test/automation handle with text-based identity across static/owned storage; tree-wide duplicates are diagnosed. |
| `ElementKey` | Unicode-validated sibling-local reconciliation key; unique keyed siblings preserve mounted lifetime across reorder, while duplicates preserve no ambiguous state. |
| `TokenId` | Unicode-validated textual token identity; static literals and dynamic construction compare, order, and hash identically. |
| `UiApp` / `AppRuntime` | Current headless application contract and bound runtime wrapper. |
| `MountedNodeId` | Non-`Copy`, process-local and runtime-instance-local `(Arc token, arena slot, generation)` identity; not authored, semantic, serialized, or preorder identity. |
| `SemanticNodeId` | Distinct read-only identity namespace sharing one mounted lifetime triplet; foundation only, not the M5 semantic tree or accessibility identity contract. |
| `MountedTreeIndex` / `MountedNodeRef` | Read-only logical-mounted-preorder inspection; arena slot order is never traversal order. |
| `WidgetTypeId` | Process-local wrapped Rust `TypeId` of a concrete widget implementation; separate from authored and runtime identity and not serialized. |
| `WidgetStateTypeId` | Process-local declared widget-state type fact used with widget implementation type for compatibility. Persistent erased state is private runtime plumbing. |
| `WidgetInvalidation` | Public manual bitset selecting interaction, layout, paint, semantic, and diagnostic capability invalidation. |
| `ReconciliationGeneration` / `ReconciliationReport` | Non-forgeable completed-generation identity, exact mounted lifetime/update/move counts, and structured reconciliation diagnostics. |
| `LayoutConstraints` | Normalized finite/unbounded measurement limits. |
| `MeasurementProvider` | Borrowed synchronous intrinsic text-measurement seam with explicit stable cache identity and behavior revision. |
| `SurfacePublication` | One publication containing aligned frame, style report, and layout report. |
| `SurfacePhaseReport` | Inspectable record of proof-level tree/style/layout/hit-test/paint/semantics/diagnostics/focus work executed by the latest runtime operation. |
| `SurfaceFrame` | Current bounds/style plus open paint/semantic/diagnostic proof product; not a paint scene or accessibility tree. |
| `on_activate` | Repeatable button callback that produces a fresh typed action for each accepted proof activation; routed semantic-command convergence remains later M4 work. |
| Sequenced work queue | One runtime-owned bounded FIFO for actions, effect starts/cancellation, timer firings, and subscription reconciliation; readiness/completion callbacks map directly to their final action envelope, and every accepted envelope receives a non-wrapping `WorkSequence`. |
| `WorkSequence` | Runtime-issued non-zero identity for accepted work, beginning at 1 and never wrapping; it is distinct from trace and reconciliation sequences. |
| Pump | Explicit iterative runtime operation that processes queued envelopes; the current runtime never pumps implicitly from submission or activation. |
| Pump budgets | Four independent explicit `PumpBudget` limits for processed envelopes, completion imports, local-work polls shared by tasks and local subscription sources, and timer promotions. |
| Runtime terminal state | Non-resettable running-but-inspectable state after work, reconciliation, or enabled-trace sequence exhaustion; it rejects new work and mutable callbacks until explicit shutdown closes the runtime. |
| `TraceSequence` | Runtime-issued non-zero identity for a canonical trace record, beginning at 1 and never wrapping when tracing is enabled. |
| Trace watermark | Exclusive `dropped_before_sequence`: `Some(S)` means every trace sequence less than `S` has been evicted from bounded retention. |
| Bounded canonical trace | One retained record sequence for queue, activation, application/work transactions, scheduler checkpoints, wake/redraw, reconciliation/focus, terminal, cancellation, and shutdown; export/replay and full trace-v2 normalization remain M4D. |
| `TraceWorkIdentity` | Read-only application-or-mounted owner, family, exact private generation value, and optional authored key attached to scheduler work facts; it is diagnostic identity, not a runtime capability. |

`on_press` was removed without an alias when `on_activate` became the authored
semantic activation callback.
`map_action` is typed and recursive. `element!` accepts the same
builder expression as direct authoring and introduces no separate binding names.
Identifiers reject empty or Unicode-whitespace-only text, surrounding Unicode
whitespace, and Unicode control characters while accepting ordinary Unicode.

## M4 contract terms

These terms are fixed by accepted ADR 0005 and ADR 0006. Application-work and
scheduler terms below are implemented by the application-work slice;
routed-event terms remain M4C.

Milestone status is M4A complete, M4B implemented and pending owner acceptance,
M4C blocked pending M4B acceptance, and M4D blocked pending M4B acceptance and
M4C. M4 is incomplete.

| Term | Meaning |
|---|---|
| Event transaction | One non-reentrant target/route/default/interaction-commit unit over an immutable mounted route snapshot. |
| Event phase | Capture, target, or bubble invocation position for one routed event. |
| Original target / current target / related target | Separate generational routing facts exposed to downstream widgets where applicable. |
| Runtime namespace | Opaque non-extractable process/runtime-local identity authority created only by the runtime; namespace/slot/generation validation makes downstream-created identity values foreign without a global registry. |
| `SurfaceId` | Opaque logical surface identity carried by M4 input/publication protocol even though only one surface exists until M10. |
| `SurfaceInputContext` | Opaque runtime-issued runtime namespace, `SurfaceId`, coordinate-space revision, and exact displayed hit-test generation carried by ingress. Retained snapshots are interpreted exactly and unavailable input is never retargeted; same-runtime/surface terminal pointer integrity cleanup is separate from ordinary targeting. |
| Semantic command | Device-independent focus, activation, cancellation, menu, context, or scroll intent shared by pointer/keyboard/controller/accessibility/automation/programmatic sources. |
| Route-only semantic command | `CancelOrBack`, menu/context-menu, or logical-scroll command whose single normal route ends with no action/runtime mutation when unconsumed; delegation is explicit queued output. |
| `PointerId` | Runtime-session identity for one active pointer stream; separate from optional device identity and mounted target identity. |
| Pointer capture | Runtime-owned `PointerId -> MountedNodeId` routing override with staged transfer and deterministic release. |
| Composition owner | Exact focused mounted generation that accepted IME composition start; focus/lifetime change invalidates later updates rather than retargeting them. |
| Commit-derived notification | Later canonical capture/composition/focus/boundary event appended from an atomic interaction or reconciliation commit before the initiating transaction's application outputs. |
| `initial_effects` | Default-empty one-time application work collected only after successful initial mount/reconciliation inside one atomic plan ordered after mounted declarations and before application subscription starts and mounted mount output. |
| Update effects | Ordered optional output returned by two-argument `update`; `()` is the no-effects result. |
| Application subscriptions | Default-empty desired stream set derived from application state after initial mount and every successful action/reconciliation. |
| Effect | Typed request recorded during application or eligible mounted work, appended only after its owning transaction commits, and executed only after owner/key revalidation. |
| Mounted work output | Restricted exact-mounted-owner imperative output supporting actions, tasks, timers, and same-owner keyed cancellation, but neither application host requests nor subscription declarations. |
| Mounted subscriptions | Dedicated state-derived complete desired set evaluated at the front of an exact-owner reconciliation envelope after committed mount or owner-local invalidation; declarations are not cached across mounted-state changes. |
| Subscription invalidation | Provisional owner-local event/update request that schedules one later mounted declaration evaluation; it neither declares nor starts subscription work. |
| Send subscription start | One nonblocking attempt returning started, unavailable, full, closed, or rejected; refusal reclaims the exact generation and is never retried implicitly. |
| Producer authority | Live-only exact-generation permission to submit a task completion, send-subscription item, or host response; cancellation, replacement, unmount, completion, terminal closure, and shutdown remove it rather than retaining a tombstone. |
| `NotStarted` | Exact send-subscription item recovery while its generation exists in `Starting` but has not committed `Started -> Running`. |
| `WidgetActivationOutput` | Independent optional action and persistent-state-change facts returned by mutable widget activation. |
| Activation capacity | Exact bounded authority (`WaitingEnvelopes`, `LocalTasks`, `SendTasks`, or `Timers`) that refused conservative activation admission. |
| Work owner | Application lifetime or one exact mounted generation responsible for task/timer/subscription/host-request cancellation. |
| `WorkKey` | Cloneable/hashable validated textual owner-local durable cancellation/replacement identity paired with work kind; private commit-bound generations remain stale completion/cancellation authority. |
| Local task | UI-thread-polled one-shot work that may hold non-`Send` state and produce `Action` directly. |
| Send-capable task | Background one-shot work producing a concrete sendable payload that is validated and mapped to `Action` on the UI thread; it does not require `Action: Send`. |
| Send-executor start outcome | Exactly one validated `Started`, `Unavailable`, `Full`, `Closed`, or `Rejected` attempt; refusal terminates that generation without retry/poisoning and maps to an action only when explicitly requested. |
| Subscription | Declarative owner/key/source-type/configuration identity for an ongoing stream whose validated items map to actions on the UI thread. |
| Application host protocol | One closed application-defined command/response/`ResponseKind` protocol; token, owner, and exact expected/actual kind validate before its UI-thread mapper. |
| Readiness checkpoint | Ordered UI-thread import, due-timer promotion, at-most-once eligible local-task polling, ready-output acceptance, and queue-tail sequencing run before/between envelopes and before quiescence. |
| Remaining pump budgets | Separate limits for cross-thread imports, local-task polls, and timer promotions; exhaustion preserves order, re-arms wake, and reports non-quiescent progress. |
| Wake request | Coalesced host signal that runtime work remains, using explicit request/acknowledge/re-arm semantics; it does not imply redraw. |
| Mandatory trace plan | Checked operation-specific exact or maximum record requirement admitted before the corresponding mutable scheduler boundary; capacity zero disables it behavior-neutrally. |
| Redraw request | Independently coalesced dirty-publication signal with take/acknowledge generation; it does not own frame timing. |
| Runtime limits | Configured waiting-envelope, transaction-output, per-family live-work, completion-ingress, and canonical-trace bounds with explicit full/closed outcomes and no silent accepted-work drop. |
| Complete terminal integrity policy | Sequence exhaustion becomes terminal before mutation; an unrollbackable post-mutation transaction-capacity failure poisons the runtime, closes producers, and cancels queued/live work. |
| Trace v2 | One bounded structured causal record sequence with sequence/transaction/reconciliation/surface/owner facts, saturation and wake/redraw records, and redacted deterministic export. |
| Trace sink | Optional bounded/try-based external copy destination subordinate to canonical trace; full/closed/failure affects only the copy and its guarded diagnostic is not recursively redelivered. |

## Accepted target terms

| Term | Meaning |
|---|---|
| Multi-surface runtime | Later support for multiple mounted roots, independent focus domains, surface lifecycle, and per-surface publication generations; M3 has one of each domain. |
| Semantic tree | Renderer-independent accessibility/automation roles, state, relationships, and actions. |
| Layout result | Computed geometry/baselines/extents independent of paint and semantics. |
| Hit-test scene | Ordered hit shapes, clips, transforms, visibility, inertness, and pointer policy. |
| Paint scene | Renderer-neutral primitives and resource references with order, clips, transforms, and metadata. |
| Host | Owner of platform/window lifecycle, normalized events, services, timing, resources, and wakeups. |
| Renderer backend | Consumer of paint primitives/resources; never owner of widget semantics or behavior. |
