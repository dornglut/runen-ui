# M4 Conformance Matrix

> **Category: Target architecture**

This matrix is the normative proof contract for M4. It does not claim current
implementation support. ADR 0005 and ADR 0006 define behavior; this document
maps every roadmap requirement to observable public behavior and required proof
ownership so implementation cannot declare M4 complete through private seams or
partial examples.

## Completion rule

M4 is complete only when:

- every row marked **Required for M4** passes on stable Rust and the declared
  MSRV;
- Counter and the downstream conformance package use public framework APIs;
- direct M1–M3 input/activation/dispatch authorities are removed rather than
  retained as alternate paths;
- current status/support documents describe only implemented behavior;
- the full repository validation command passes at the exact reviewed head.

A test helper is acceptable only when it enters the canonical queue and preserves
normal target, owner, ordering, cancellation, trace, and reconciliation checks.

## Current M4B proof closure

M4A is complete and the M4B implementation correction is complete at the exact
reviewed head, pending owner acceptance. The table
below maps the implemented ADR 0006 slice to its current behavioral proof. It
does not mark the M4C routed-event rows or M4D export/replay rows below as
implemented, and therefore does not claim complete M4 conformance.

| Implemented M4B scope | Current proof | Gate result |
|---|---|---|
| Atomic initial/update work ordering and admission | [`application_work.rs`](../../crates/runenui_runtime/tests/application_work.rs) | Pass |
| Exact activation capacity, explicit state mutation, coalesced invalidation, and authoritative `NoEffect` | `activation_queue::conservative_activation_admission_rejects_every_bounded_authority_before_callback`, `mounted_work_output::coalesced_subscription_invalidation_is_an_effect_not_no_effect`, and `activation_result_counts_auxiliary_batches_and_separates_wake_from_redraw` | Pass |
| Exact-mounted-owner lifecycle and activation output | [`mounted_work_output.rs`](../../crates/runenui_runtime/tests/mounted_work_output.rs) and downstream [`subscriptions.rs`](../../tests/external_widget/tests/subscriptions.rs) | Pass |
| Same-batch keyed cancellation, replacement, and trace binding | [`transactional_cancellation.rs`](../../crates/runenui_runtime/tests/transactional_cancellation.rs) | Pass |
| `Starting -> Running` send subscriptions, exact `NotStarted`, refusal reclamation, and 3-vs-2 trace admission | `subscription_scheduler::send_subscription_start_outcomes_are_once_only_reclaimed_and_explicitly_retryable`, `send_subscription_item_admits_its_exact_three_record_trace_plan`, and `send_subscription_item_with_only_two_records_never_runs_mapper` | Pass |
| Registry reclamation and bounded retained state | [`registry_lifecycle.rs`](../../crates/runenui_runtime/tests/registry_lifecycle.rs) | Pass |
| Send-task stale ownership, 3-vs-2 trace admission, trace-disabled behavior, and executor outcomes | `scheduler_work::cancelled_send_completion_never_invokes_ui_mapper`, `send_task_completion_admits_its_exact_three_record_trace_plan`, `send_task_completion_with_only_two_records_never_runs_mapper`, and `disabled_trace_changes_no_send_completion_behavior` | Pass |
| Tombstone-free host response authority, 10,000 cancel/replace stress, and 4-vs-3 trace admission | `subscriptions_host::repeated_host_cancellation_and_replacement_retain_only_live_authority`, `detached_host_completion_admits_its_exact_four_record_trace_plan`, and `detached_host_completion_with_only_three_records_never_runs_mapper` | Pass |
| Before-unmount producer revocation | `transactional_cancellation::mounted_subscription_authority_is_stale_before_removal_unmount_callback_runs`, `mounted_subscription_authority_is_stale_before_keyed_replacement_unmount_callback_runs`, and `mounted_send_task_completion_is_stale_during_unmount_callback` | Pass |
| Lock-free host callback boundary, exact delivery claims, callback serialization, close linearization, and acknowledgment re-arm | Deterministic `wake::tests::{pending_request_is_claimed_once_after_transport_installation, transport_replacement_does_not_reclaim_delivered_request, wake_callback_can_close_same_state_without_deadlock, wake_callback_can_reenter_request_and_transport_setup, blocking_wake_callback_does_not_block_close, wake_callbacks_are_serialized_without_lock_held_invocation, request_during_in_flight_callback_is_delivered_after_callback_returns, close_prevents_new_delivery_claims, claimed_callback_may_finish_after_close_without_rearming, closed_wake_state_cannot_be_reopened}` plus `wake_redraw::pump_acknowledgment_and_rearm_do_not_strand_work`; repeated install/replacement races are supplementary | Pass |
| Sequence exhaustion, producer closure, and inspectable terminal state | [`terminal_integrity.rs`](../../crates/runenui_runtime/tests/terminal_integrity.rs) and [`wake_redraw.rs`](../../crates/runenui_runtime/tests/wake_redraw.rs) | Pass |
| Semantic transaction trace ordering and provisional cancellation binding | [`transactional_cancellation.rs`](../../crates/runenui_runtime/tests/transactional_cancellation.rs) | Pass |
| Per-family scheduler causal chains and opaque work identity | [`scheduler_work.rs`](../../crates/runenui_runtime/tests/scheduler_work.rs), [`subscription_scheduler.rs`](../../crates/runenui_runtime/tests/subscription_scheduler.rs), [`subscriptions_host.rs`](../../crates/runenui_runtime/tests/subscriptions_host.rs), and [`trace_scheduler.rs`](../../crates/runenui_runtime/tests/trace_scheduler.rs) | Pass |

The repository gate runs all public/downstream proofs, internal integrity seams,
Clippy with denied warnings, and the declared MSRV through `cargo validate`.

## Event routing and downstream-widget participation

| Proof | Required observation | Proof owner | M4 requirement |
|---|---|---|---|
| Capture/target/bubble order | Root-to-parent capture, target once, parent-to-root bubble over one immutable mounted route | Downstream conformance widget | Required for M4 |
| Routing facts | Widget observes phase, original target, current target, related target where applicable, source/modality, logical sequence/time, and surface-input context | Downstream conformance widget | Required for M4 |
| Stop propagation | Later route callbacks stop while default behavior remains eligible | Downstream conformance widget | Required for M4 |
| Prevent default | Default behavior is suppressed while later routed callbacks still run | Downstream conformance widget | Required for M4 |
| Multiple non-`Clone` actions | All actions preserve callback/default emission order without cloning | Downstream conformance widget | Required for M4 |
| Same-transaction capability mutation | Callback invalidation changes default behavior in the same event transaction | Downstream conformance widget | Required for M4 |
| Reentrant submission | Callback-submitted event/action/command appends to queue tail and never recurses | Runtime integration test | Required for M4 |
| Stale/foreign target | No widget callback executes; structured target outcome and trace record are produced | Runtime integration test | Required for M4 |
| Bridge integrity | Invalid checked event bridge aborts before first routed callback and never partially mutates route state | Runtime integration test | Required for M4 |

Every event-policy row below is independently normative; `C/T/B` means
capture/target/bubble.

| Family | Required route/default observation | Proof owner | M4 requirement |
|---|---|---|---|
| Pointer down | C/T/B, bubbles, cancelable; eligible primary default may focus and establish pressed owner/capture | Runtime + downstream widget | Required for M4 |
| Pointer move | C/T/B, bubbles, non-cancelable; physical path and pressed-inside update | Runtime + downstream widget | Required for M4 |
| Pointer up | C/T/B, bubbles, cancelable; valid primary release may queue `Activate` | Runtime + Counter | Required for M4 |
| Pointer cancel | C/T/B to live capture/pressed owner, bubbles, non-cancelable; integrity cleanup always runs | Runtime integration test | Required for M4 |
| Wheel | C/T/B, bubbles, cancelable; default produces logical-scroll command behavior | Downstream conformance widget | Required for M4 |
| Keyboard down | C/T/B, bubbles, cancelable; focus/activation and Space-press policy | Runtime + Counter | Required for M4 |
| Keyboard up | C/T/B, bubbles, cancelable; valid matching Space completion policy | Runtime + Counter | Required for M4 |
| Committed text | C/T/B, bubbles, cancelable; no editable-text default is claimed in M4 | Downstream conformance widget | Required for M4 |
| IME start/update/end | C/T/B, bubbles, non-cancelable; composition-lifetime bookkeeping only | Runtime integration test | Required for M4 |
| Semantic command | C/T/B, bubbles, cancelable; documented command default | Downstream conformance widget | Required for M4 |
| Pointer enter/leave | Target once, non-bubbling, non-cancelable | Runtime + downstream widget | Required for M4 |
| Capture lost/gained | Target once, non-bubbling, non-cancelable | Runtime + downstream widget | Required for M4 |
| Focus out/in | C/T/B on the committed live old/new route, bubbling, non-cancelable | Runtime + downstream widget | Required for M4 |
| Composition cancellation | C/T/B to the old live owner route, bubbling, non-cancelable | Runtime + downstream widget | Required for M4 |
| Modality transition | Retained state and trace fact only; no widget event family is delivered | Runtime integration test | Required for M4 |
| Integrity versus prevention | `prevent_default` never suppresses capture/pressed/composition/stale-work/dead-focus cleanup | Runtime integration test | Required for M4 |

## Surface-input, hit testing, and pointer boundaries

| Proof | Required observation | Proof owner | M4 requirement |
|---|---|---|---|
| Current surface generation | Input uses the exact current published hit-test snapshot | Runtime + publication integration test | Required for M4 |
| Previous retained generation | Input uses its exact retained previous snapshot and is not re-hit against current geometry, including when configured retention exceeds the default | Runtime + publication integration test | Required for M4 |
| Retired surface generation | Deterministic ring retirement returns `RetiredSurfaceContext` with no retarget | Runtime + publication integration test | Required for M4 |
| Foreign runtime context | Another namespace returns `ForeignSurfaceContext` | Runtime integration test | Required for M4 |
| Foreign logical surface | Another `SurfaceId` returns `ForeignSurface` | Runtime integration test | Required for M4 |
| Missing surface generation | Unknown non-retired generation returns `MissingSurfaceGeneration` | Runtime integration test | Required for M4 |
| Pointer validation order | With multiple invalid dimensions, validation proceeds namespace, logical surface, active `PointerId`, retained snapshot generation, then target; later checks cannot retarget or mutate an earlier rejection | Runtime integration test | Required for M4 |
| Retired-context pointer up activation | Active same-runtime/surface pointer up is not routed/re-hit-tested and never activates | Runtime + Counter integration | Required for M4 |
| Retired-context pointer up pressed cleanup | The same rejection performs integrity-only cancellation and clears pressed ownership | Runtime integration test | Required for M4 |
| Retired-context pointer up capture cleanup | The same rejection clears capture, closes the stream, and emits applicable loss/cancellation facts | Runtime integration test | Required for M4 |
| Missing-context pointer up cleanup | Missing generation follows the same no-route/no-activate integrity-only cancellation contract | Runtime integration test | Required for M4 |
| Pointer cancel without retained geometry | Same-runtime/surface active cancellation cleans up pressed/capture/stream state despite retired/missing snapshot | Runtime integration test | Required for M4 |
| Foreign-runtime pointer cancel | Foreign namespace cannot mutate the local pointer stream | Runtime integration test | Required for M4 |
| Foreign-surface pointer cancel | Foreign logical surface cannot mutate the local pointer stream | Runtime integration test | Required for M4 |
| Non-terminal unavailable context | Pointer move/down/wheel is rejected without route, retarget, or interaction mutation | Runtime integration test | Required for M4 |
| Terminal cleanup causality | Trace links the retired/missing context diagnosis to the integrity cleanup facts | Trace + runtime integration | Required for M4 |
| Pointer ingress bundle | Leave notifications run inner-to-outer, enter notifications outer-to-inner, then ordinary pointer event | Runtime integration test | Required for M4 |
| Captured versus physical target | Routed target remains capture owner while physical hit target/path continue to update | Downstream conformance widget | Required for M4 |
| Stationary pointer after geometry change | Layout/visibility/hit-generation change re-hit-tests retained pointer position and emits deterministic boundary updates without pointer movement | Runtime + publication integration test | Required for M4 |
| Multi-pointer isolation | Pointer identity, pressed state, hover contribution, and capture do not leak between active pointers | Runtime integration test | Required for M4 |
| Multi-pointer publication update | Retained pointers re-hit by registration sequence; each pointer leaves inner-to-outer then enters outer-to-inner | Runtime + publication integration test | Required for M4 |
| Accepted older transaction | New publication does not change the target of a transaction already accepted with a retained older context | Runtime + publication integration test | Required for M4 |
| Cancellation cleanup | Pointer cancel/removal/replacement/disable/shutdown clears incompatible capture and pressed state | Runtime integration test | Required for M4 |

## Activation and modality convergence

Every source below must reach the same routed semantic `Activate` command and
produce the same typed application action/update result when the target remains
eligible.

| Source | Required input path | Required cancellation/negative proof | Proof owner |
|---|---|---|---|
| Primary pointer | Down -> focus/press/capture -> release inside -> `Activate` | Release outside, cancel, remove, replace, disable, or non-actionable transition never activates | Counter + runtime integration |
| Keyboard Enter | Accepted non-repeat key down -> `Activate` | Repeated key down does not duplicate activation unless policy explicitly requests it | Counter |
| Keyboard Space | Key down establishes pressed state; matching key up -> `Activate` | Focus/lifetime/eligibility loss before key up cancels | Counter |
| Normalized controller | Controller mapping -> semantic `Activate` | No raw gamepad type enters widget/runtime contract | Headless command proof |
| Accessibility stub | Semantic action mapping -> routed `Activate` | Stale semantic-to-mounted mapping is rejected | M4 stub, expanded in M5 |
| Automation | Automation source -> routed `Activate` | Authored-ID ambiguity and stale mounted target are diagnosed | Public/runtime proof |
| Programmatic API | `activate` helper -> routed `Activate` | Helper cannot bypass route/default/update order | Counter + runtime integration |

The authored callback is `on_activate`. No `on_press` compatibility authority
remains after migration.

## Semantic commands and modality

| Command/fact | Required behavior | Proof owner | M4 requirement |
|---|---|---|---|
| `FocusNext` / `FocusPrevious` | Deterministic logical-order traversal and root wrapping | Headless focus corpus | Required for M4 |
| Directional focus | Every linked corpus vector passes without exposing the private score/formula | Directional conformance corpus | Required for M4 |
| Scope delegation | Nested default delegates to parent; explicit wrap/trap/stop/delegate policies behave deterministically | Headless focus corpus | Required for M4 |
| Focus restoration | Exact remembered generation restores only while live/eligible; stale restoration falls back to traversal | Headless focus corpus | Required for M4 |
| Unconsumed `CancelOrBack` | One normal route completes with no action or runtime navigation mutation | Downstream conformance widget | Required for M4 |
| Unconsumed `OpenMenu` | One normal route completes with no action or menu mutation | Downstream conformance widget | Required for M4 |
| Unconsumed `OpenContextMenu` | One normal route completes with no action or context-menu mutation | Downstream conformance widget | Required for M4 |
| Unconsumed logical scrolling | One normal route completes with no production scrolling mutation | Downstream conformance widget | Required for M4 |
| No second ancestor delegation | Capture/bubble ancestor participation is not followed by another implicit ancestor pass | Downstream conformance widget | Required for M4 |
| Explicit command delegation | Callback-emitted replacement command/action receives a new sequence at normal output position and never recurses | Downstream conformance widget | Required for M4 |
| Prevented wheel | Preventing default emits no logical-scroll command | Downstream conformance widget | Required for M4 |
| Unprevented wheel | Default emits exactly one logical-scroll command, which then follows route-only behavior | Downstream conformance widget | Required for M4 |
| Modality tracking | Pointer, keyboard, controller, accessibility, automation, and programmatic transitions are deterministic and do not alter command semantics | Runtime integration test | Required for M4 |

The [directional-focus corpus](m4-directional-focus-corpus.md) is enumerated so
no subset can satisfy the aggregate directional row:

| Vector | Required result | Proof owner | M4 requirement |
|---|---|---|---|
| DF-01 | Direct Right selects `A` | Directional corpus test | Required for M4 |
| DF-02 | Direct Left selects `A` | Directional corpus test | Required for M4 |
| DF-03 | Direct Up selects `A` | Directional corpus test | Required for M4 |
| DF-04 | Direct Down selects `A` | Directional corpus test | Required for M4 |
| DF-05 | In-beam `A` beats nearer off-beam `B` | Directional corpus test | Required for M4 |
| DF-06 | Partial-overlap `A` wins | Directional corpus test | Required for M4 |
| DF-07 | Unequal-size in-beam `A` wins | Directional corpus test | Required for M4 |
| DF-08 | Eligible overlapping-bounds `A` wins | Directional corpus test | Required for M4 |
| DF-09 | Mounted-order tie selects `B` | Directional corpus test | Required for M4 |
| DF-10 | Nested default delegates to parent `P` | Directional corpus test | Required for M4 |
| DF-11 | Nested trap returns `None` | Directional corpus test | Required for M4 |
| DF-12 | Nested directional wrap selects `A` | Directional corpus test | Required for M4 |
| DF-13 | Root linear Next wraps from `B` to `O` | Directional corpus test | Required for M4 |
| DF-14 | Root directional boundary returns `None` | Directional corpus test | Required for M4 |
| DF-15 | Exact live remembered generation `A@7` restores | Directional corpus test | Required for M4 |
| DF-16 | Stale `A@7` is not retargeted; fallback selects `B@3` | Directional corpus test | Required for M4 |
| DF-17 | Disabled nearer `A` is excluded; `B` wins | Directional corpus test | Required for M4 |
| DF-18 | Hidden nearer `A` is excluded; `B` wins | Directional corpus test | Required for M4 |
| DF-19 | All candidates outside half-plane returns `None` | Directional corpus test | Required for M4 |
| DF-20 | Zero-gap edge-touching `A` wins | Directional corpus test | Required for M4 |

## Focus, capture, and composition transition order

| Proof | Required order/behavior | Proof owner | M4 requirement |
|---|---|---|---|
| Capture transfer | Old owner loses before new owner gains | Runtime integration test | Required for M4 |
| Composition and focus | Composition cancellation precedes `FocusOut`; `FocusOut` precedes `FocusIn` | Runtime integration test | Required for M4 |
| Commit notifications versus actions | Commit-derived notifications are queued before initiating event/update application outputs | Runtime integration test | Required for M4 |
| Removed notification target | Removed node receives no post-unmount callback; trace records suppressed delivery | Runtime integration test | Required for M4 |
| IME stale completion | Late update/commit for old composition generation is rejected and never retargeted | Runtime integration test | Required for M4 |
| Focus-within | Old/new ancestor routes produce exact invalidation without post-removal mutation | Runtime integration test | Required for M4 |

## Application lifecycle and update contract

| Proof | Required observation | Proof owner | M4 requirement |
|---|---|---|---|
| No-effects update | Two-argument `update` returning `()` performs one state mutation and reconciliation with no effect boilerplate | Counter | Required for M4 |
| Initial effects | `initial_effects` collects only after successful initial mount/reconciliation and starts no work before commit | Runtime integration test | Required for M4 |
| Initial work ordering | One atomic plan appends mounted declarations in mounted preorder, initial effects in collector order, application subscription starts in declaration order, then mounted mount output in mounted preorder/collector order | `application_work` | Required for M4 |
| Initial application subscriptions | State-derived application subscriptions are declared after initial mount without a synthetic first action | Runtime integration test | Required for M4 |
| Update effects | Outputs remain provisional until update/root/reconciliation commit | Runtime integration test | Required for M4 |
| One action per transaction | Each action completes state update and reconciliation before the next action/output becomes visible | Runtime integration test | Required for M4 |
| Exact application transaction order | Cancellation cleanup, mounted declaration reconciliation, update outputs, application subscription starts, then mounted lifecycle outputs receive consecutive global sequences | `trace_scheduler::application_transaction_assigns_the_global_adr_order_exactly` | Required for M4 |
| Unexpected post-mutation failure | Runtime enters terminal poisoned state, starts no provisional external work, and permits only inspection/extraction/shutdown | Integrity test seam | Required for M4 |

## Work identity, cancellation, and subscriptions

| Proof | Required observation | Proof owner | M4 requirement |
|---|---|---|---|
| Keyed replacement | Same owner/kind/key invalidates old generation before replacement starts | Runtime integration test | Required for M4 |
| Stale cancellation | Cancellation for old generation/key cannot cancel newer replacement | Runtime integration test | Required for M4 |
| Same-batch cancel then start | Previous generation is cancelled, then replacement generation is allocated | Runtime integration test | Required for M4 |
| Same-batch start then cancel | Cancellation binds to and cancels the generation started earlier in the batch | Runtime integration test | Required for M4 |
| Same-batch start then replacement | First start is invalid before the replacement becomes startable | Runtime integration test | Required for M4 |
| Same-batch cancel then cancel | Both bind consistently; the second is idempotent and cannot affect newer work | Runtime integration test | Required for M4 |
| Mounted work owner removal | Descendant-before-ancestor cancellation; late completion rejected before mapper | Downstream conformance + runtime | Required for M4 |
| Removal before start | Committed effect-start envelope suppresses execution when owner became stale first | Runtime integration test | Required for M4 |
| Anonymous work | Completes or ends with owner/shutdown; no private runtime ID is stored as durable app identity | Runtime integration test | Required for M4 |
| Application subscription retention | Equal owner/key/source/config stays live | Runtime integration test | Required for M4 |
| Application subscription replacement | Changed/unidentifiable config invalidates old before new start | Runtime integration test | Required for M4 |
| Application subscription absence | Removed desired declaration cancels stream | Runtime integration test | Required for M4 |
| Application duplicate subscription key | No ambiguous subscription survives; structured diagnostic emitted | Runtime integration test | Required for M4 |
| Application state change | Application subscription declaration is reevaluated from current post-update state before starts are planned | `subscription_scheduler::initial_effect_action_replaces_the_old_subscription_before_its_start_callback` | Required for M4 |
| Mounted initial declaration | Downstream widget's public declaration capability supplies one complete desired set after successful mount | Downstream conformance widget | Required for M4 |
| Mounted no pre-commit start | No mounted subscription work starts before its owner mount commits | Downstream conformance + runtime | Required for M4 |
| Mounted equal declaration | Owner/key/source/config equality retains the existing exact subscription generation | Runtime integration test | Required for M4 |
| Mounted changed declaration | Changed config invalidates old generation before replacement start envelope | Runtime integration test | Required for M4 |
| Mounted absent declaration | Missing entry in the complete desired set cancels the old generation | Runtime integration test | Required for M4 |
| Mounted duplicate key | Duplicate owner-local key emits a structured diagnostic and retains no ambiguous subscription | Downstream conformance + runtime | Required for M4 |
| Event-triggered declaration invalidation | Committed owner-local invalidation schedules one later declaration evaluation before dependent output/quiescence | Downstream conformance + runtime | Required for M4 |
| Compatible-update declaration invalidation | Committed lifecycle-context invalidation schedules exactly one complete owner-local declaration evaluation | Downstream conformance + runtime | Required for M4 |
| Activation declaration invalidation | Activation invalidation, primary action, and auxiliary work commit once with reconciliation ordered first | `mounted_work_output::activation_commits_one_subscription_first_primary_then_auxiliary_batch` + downstream conformance | Required for M4 |
| Mounted newest-state declaration | A queued declaration callback observes newest live widget state rather than a cached declaration | `external_widget::queued_mounted_reconciliation_observes_the_newest_live_widget_state` | Required for M4 |
| Mounted stale-owner declaration | Removal before a dirty envelope suppresses the declaration callback and records its exact stale owner | `external_widget::removed_dirty_owner_suppresses_the_declaration_callback_at_its_envelope` | Required for M4 |
| Unrelated event retention | Event without owner-local invalidation neither re-declares nor cancels a retained mounted subscription | Runtime integration test | Required for M4 |
| Mounted owner removal | Subscription generation invalidates before the owner's unmount callback completes | Downstream conformance + runtime | Required for M4 |
| Mounted replacement stale item | Item from declaration generation replaced by a changed declaration never invokes its mapper | Runtime integration test | Required for M4 |
| Public mounted declaration bridge | External widget implements and exercises the dedicated capability without privileged registry access | Downstream conformance widget | Required for M4 |

## Tasks, timers, and host requests

| Proof | Required observation | Proof owner | M4 requirement |
|---|---|---|---|
| Local non-`Send` task | Captures `Rc`/non-`Send` state and produces a non-`Send` action on UI thread | Runtime integration test | Required for M4 |
| Successful send-task start | One validated executor attempt returns `Started`; sendable payload crosses boundary and non-`Send` action is created only after UI-thread validation | Runtime integration test | Required for M4 |
| Executor unavailable | `Unavailable` immediately terminates the exact generation without poisoning | Executor-stub test | Required for M4 |
| Executor full | `Full` immediately terminates the exact generation without hidden pending storage | Executor-stub test | Required for M4 |
| Executor closed | `Closed` immediately terminates the exact generation coherently | Executor-stub test | Required for M4 |
| Executor rejected | `Rejected` immediately terminates the exact generation and deterministically returns/drops refused ownership | Executor-stub test | Required for M4 |
| Refused payload ownership | Any refused start that returns its owned payload transfers it to the runtime-side effect record for deterministic drop or typed failure handling, with no leak or hidden retention | Executor-stub test | Required for M4 |
| No hidden executor retry | One start envelope performs exactly one attempt; refusal never retries automatically | Executor-stub test | Required for M4 |
| Refusal generation terminality | Failed generation cannot transition to running or accept later completion | Runtime integration test | Required for M4 |
| Refusal stale completion | Completion submitted after refusal is rejected before mapper | Runtime integration test | Required for M4 |
| Typed start-failure mapping | Explicit UI-thread failure mapper queues exactly one action after failure commits, without recursion or `Action: Send` | Executor-stub test | Required for M4 |
| Absent start-failure mapper | Refusal queues no application action by default | Executor-stub test | Required for M4 |
| Refusal integrity | Ordinary executor refusal remains recoverable and never poisons runtime | Integrity test | Required for M4 |
| Explicit executor retry | A new application effect allocates a new generation and performs a new single attempt | Executor-stub test | Required for M4 |
| Executor refusal trace | Trace causally relates request, committed start envelope, one attempt, structured refusal, terminal transition, and any queued typed failure action | Trace + executor-stub test | Required for M4 |
| Invalid send completion | Stale/cancelled token never invokes mapper | Runtime integration test | Required for M4 |
| Send-subscription start outcomes | Started/unavailable/full/closed/rejected are one-attempt terminal outcomes; refusal reclaims the generation and retry requires a new revision | `subscription_scheduler::send_subscription_start_outcomes_are_once_only_reclaimed_and_explicitly_retryable` | Required for M4 |
| Send-subscription sink rejection | Full, closed, and stale rejection return the exact item without invoking the mapper | `subscription_scheduler::send_source_starts_once_and_full_or_closed_sink_returns_the_exact_item` + `cancelled_send_subscription_sink_returns_the_exact_stale_item` | Required for M4 |
| Equal-deadline timers | Fire by creation sequence | Manual-clock test | Required for M4 |
| Repeating timer | Advances from previous deadline; missed ticks coalesce by default | Manual-clock test | Required for M4 |
| Host response | Token, owner, and expected response discriminator validate before mapper/action | Host-stub test | Required for M4 |
| Host mismatch | Wrong response kind produces structured diagnostic and no action | Host-stub test | Required for M4 |
| Host kind contract | `expected_response(command)` and `response_kind(response)` exact equality is required before mapper invocation | Host-stub + compile proof | Required for M4 |
| Host response linearization | One lock-protected `Open` authority admits exactly one detached, direct, or cancellation winner; full detached ingress remains retryable and cancellation removes an accepted queued payload before mapping | `subscriptions_host` | Required for M4 |
| Widget host boundary | Reusable mounted widget cannot issue app-specific host request directly | Compile-fail/downstream proof | Required for M4 |

## Queue, wake, redraw, and saturation

| Proof | Required observation | Proof owner | M4 requirement |
|---|---|---|---|
| FIFO no overtaking | Later event/effect/completion never overtakes accepted envelope | Runtime integration test | Required for M4 |
| Initial readiness checkpoint | Before the first envelope: import completions, promote timers, poll eligible local tasks once, accept outputs, append new sequences | Headless pump test | Required for M4 |
| Between-envelope checkpoint | The same ordered checkpoint runs after each processed envelope while envelope budget remains | Headless pump test | Required for M4 |
| Quiescence checkpoint | A final allowed readiness check precedes any quiescent result | Headless pump test | Required for M4 |
| Cross-thread import budget | Exhaustion preserves transport order, leaves payloads pending, re-arms wake, and reports non-quiescent progress | Headless pump test | Required for M4 |
| Local poll budget | Each eligible local task is polled at most once by creation order per checkpoint; exhaustion preserves readiness | Headless pump test | Required for M4 |
| Timer promotion budget | Due timers promote by deadline then creation sequence; exhaustion leaves due timers pending | Manual-clock pump test | Required for M4 |
| Processed-envelope budget | Exactly one envelope executes per count; exhaustion never executes appended work recursively | Headless pump test | Required for M4 |
| Quiescence criteria | Empty queue plus no import, due timer, immediately ready allowed local task, or mandatory derived work; future timer only returns deadline | Headless pump test | Required for M4 |
| Wake coalescing | Multiple readiness changes before acknowledgment request one transport wake | Host-stub test | Required for M4 |
| Wake clear race | Completion arriving after acknowledgment/clear and during pump requests/re-arms a wake; no work is stranded | Deterministic race harness | Required for M4 |
| Wake callback lock boundary | Delivery is claimed under wake-state authority, then `WakeTransport::request_wake` runs with no RunenUI mutex guard held | `wake::tests::wake_callback_can_close_same_state_without_deadlock`, `wake_callback_can_reenter_request_and_transport_setup`, and `blocking_wake_callback_does_not_block_close` | Required for M4 |
| Wake callback serialization | A new acknowledged epoch remains pending behind an in-flight callback and is claimed once after it returns; callbacks never overlap | `wake::tests::wake_callbacks_are_serialized_without_lock_held_invocation` | Required for M4 |
| Wake close linearization | Close prevents later claims, does not wait for a prior claim, and prior callback completion cannot reopen or re-arm `Closed` | `wake::tests::blocking_wake_callback_does_not_block_close`, `claimed_callback_may_finish_after_close_without_rearming`, and `closed_wake_state_cannot_be_reopened` | Required for M4 |
| Redraw independence | Wake without dirty output does not redraw; dirty output can request redraw without executing work | Host-stub test | Required for M4 |
| Publication race | Invalidation racing with publication leaves a new redraw request armed | Host-stub test | Required for M4 |
| External saturation | Full/closed ingress returns structured result and drops no accepted action | Limit test | Required for M4 |
| Cross-thread saturation | Sender receives unaccepted payload on full/closed result | Limit test | Required for M4 |
| Callback final-sequence admission | With exactly one sequence left, local/send task, timer, typed refusal, local/send subscription, and host mapper each enqueue the final action directly | `scheduler_work::one_remaining_sequence_is_consumed_only_by_each_final_scheduler_action` + subscription/host counterparts | Required for M4 |
| Host cancellation exhaustion | Cancellation sequence exhaustion terminalizes, closes scheduling authority, returns later actions, and shutdown remains idempotent | `subscriptions_host::host_cancellation_sequence_exhaustion_terminalizes_and_closes_authority` | Required for M4 |
| Transaction output limit | Limit is reserved/enforced; ignored overflow follows terminal integrity policy | Integrity test seam | Required for M4 |
| Mutable activation admission | Complete configured callback allowance is reserved before mutation across queue, sequences, generations, every mounted-accessible family, and mandatory trace; rejection invokes no callback | `activation_queue` | Required for M4 |
| Activation batch result and signals | Result exposes first sequence, optional primary-action sequence, and total queued envelopes; accepted queue work wakes and publication dirtiness redraws | `mounted_work_output` | Required for M4 |

## Trace v2 and shutdown

| Proof | Required observation | Proof owner | M4 requirement |
|---|---|---|---|
| Single trace authority | No duplicate compatibility vector remains | API/runtime test | Required for M4 |
| Bounded retention | Capacity evicts oldest and advances exact dropped-before watermark | Trace test | Required for M4 |
| Causality | Input/command -> route/default -> transition -> action/update/reconcile -> effect -> wake/redraw -> publication reconstructs from records | End-to-end Counter trace | Required for M4 |
| Scheduler causal foundation | Application transaction -> request -> generation commit -> actual-sequence start -> readiness/completion/cancellation -> final accepted action -> next application transaction is reconstructable for local/send tasks, timers, local/send subscriptions, and host requests | `scheduler_work` + `subscription_scheduler` + `subscriptions_host` + `trace_scheduler` causal proofs | Required for M4 |
| Same-batch semantic trace order | Request/invalidation facts follow callback collector order even when cleanup envelopes group before starts; provisional commit precedes its same-batch cancellation and invalidation | `transactional_cancellation` | Required for M4 |
| Redaction | Text/IME payloads redacted by default; explicit opt-in is separate from generic debug | Trace test | Required for M4 |
| No global debug bound | Non-`Debug` action application builds and traces | Compile/runtime proof | Required for M4 |
| Deterministic JSONL | Versioned export is stable for identical logical execution | Snapshot test | Required for M4 |
| Bounded sink full | Try/bounded full outcome loses only external copy and records a structured canonical diagnostic | Trace sink test | Required for M4 |
| Closed sink | Closed delivery is diagnosed and no later shutdown delivery is attempted | Trace sink test | Required for M4 |
| Failing sink | Delivery failure is structured and cannot submit runtime work | Trace sink test | Required for M4 |
| Sink transaction isolation | Sink delivery never blocks or runs arbitrary work inside a mutable transaction | Trace sink test | Required for M4 |
| Canonical trace preservation | Sink refusal/failure does not remove or reorder the authoritative retained record | Trace sink test | Required for M4 |
| Sink-copy behavioral isolation | Losing the external copy changes no application/runtime behavior | Trace sink test | Required for M4 |
| Sink diagnostic recursion guard | Failure diagnostic is not redelivered through the same failing path and cannot recurse indefinitely | Trace sink test | Required for M4 |
| Shutdown | Explicit shutdown, `into_state`, and `Drop` cancel each owner once; no later action is delivered | Runtime integration test | Required for M4 |
| Poison trace | Terminal integrity transition and cancellation are reconstructable | Integrity test seam | Required for M4 |
