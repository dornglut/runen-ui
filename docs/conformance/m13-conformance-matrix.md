# M13 Host Input Arbitration Conformance Matrix

> **Category:** Target architecture
>
> **Status:** Framework arbitration obligations owner-accepted; native/Engine integration proof remains blocked
>
> **Milestone:** M13
>
> **Architecture:** [ADR 0019](../adr/0019-event-correlated-host-input-arbitration.md)
>
> **Inheritance:** [ADR 0005](../adr/0005-canonical-event-routing-and-commands.md), [ADR 0006](../adr/0006-effects-scheduling-and-trace-v2.md), [M10](m10-conformance-matrix.md), [M11](m11-conformance-matrix.md)

These rows extend—not replace—M4 canonical routing, M7 host embedding, M10 editing/IME and M11 control conformance. The runtime/API obligations `M13ARB-01`–`M13ARB-07` have independently reviewed executable evidence, owner acceptance and accepted-main verification. `M13ARB-08`–`M13ARB-10` remain blocked on their independently required native/Engine integration evidence. Acceptance of either an architectural decision or the framework implementation does **not** certify that external proof.

```text
10 total unique rows
7 owner-accepted
0 implementation-complete
0 proof-complete
3 blocked
0 duplicate IDs
0 invalid statuses
0 invalid schemas
```

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| M13ARB-01 | Each accepted native-input submission remains Pending until one exact `(InputArbitrationScope, WorkSequence)` settles or the scope retires; ingress rejection is separate and not gameplay permission. Source/window/device mapping remains host-owned. | #428 public two-`UiApp` headless consumer correlates accepted keyboard/text/composition/pointer receipts across unrelated runtime slots and validates exactly-once finality. | Foreign/missing focus, stale surface, queue-full or rejected ingress creates no fake settled receipt; runtime-slot reuse and source-absent device do not alias identities. | Exact ingress/rejection and canonical work-sequence diagnostics, plus authoritative trace-disabled settlement results. | M13ARB | owner-accepted | Required |
| M13ARB-02 | One ordered pump record stream preserves before/after ownership revision and per-input settlement in actual commit/checkpoint order; input-triggered change precedes its own settlement, later derived action follows it, and `processed_through` does not mean causal closure. | #428 ordered public FIFO fixture with input→queued command/action→focus/modal transition across two bounded pumps and final snapshot revision oracle. | Separate-vector replay, zero/partial work budget, delayed downstream action, false quiescence on observation pause or skipped mandatory checkpoint fails. | Exact work/pump reports, record sequence/revisions and optional M4 causal trace compared with trace disabled. | M13ARB | owner-accepted | Required |
| M13ARB-03 | Committed routing exposes independently typed UI conflict claim and underlying neutral keyboard/text/IME/shortcut, pointer press/capture/selection, touch and modal evidence; downstream custom widget can explicitly claim host input without built-in privilege. | #428 public custom widget + TextField/Button/Menu/shortcut/IME/pointer fixtures establish `ExclusiveUi` vs `ObservedNonexclusive` vs `Unclaimed` and supported reasons. | Hover, no-op Move/Cancel, bare hit/callback, propagation-stop, default-prevention, trace `PointerDefaultApplied` alone and merely queued action cannot manufacture a claim; failures are `Undetermined`. | Routed/default/presentation/capture/selection diagnostics; independent ordinary public outcome with trace off/redacted. | M13ARB | owner-accepted | Required |
| M13ARB-04 | Revisioned immutable snapshots cover text/focus/composition/Space, per-pointer ownership and retained surface/modal authority, including changes from submission, publication, reconciliation and direct terminal outside pump. A successful direct publication may also carry the exact contiguous transition described in [ADR 0019](../adr/0019-event-correlated-host-input-arbitration.md#direct-publication-continuity). | #428 query after accepted pending composition, focus/reconcile, modal, pointer cleanup and logical publication with no new native input; verify revision coherence. Add chained direct-publication proof from a materialized baseline, checking each transition against the prior witness and preserving it through product extraction. | No revision on no-op; revision cannot wrap/reuse; missing retained context and stale/foreign surface cannot advertise valid authority; logical publication does not imply completed Present. Absent transitions remain conservative across unmaterialized state, pending/coalesced boundaries, composition ABA, scope mismatch, or terminal failure. | Ownership revision/snapshot oracle with optional lifecycle/publication trace, verified without trace reliance. | M13ARB | owner-accepted | Required |
| M13ARB-05 | Every actually reached input terminates in truthful `Committed`, `ProcessingRejected` or `Aborted` with exact reason; accepted unavailable-context pointer Up/Cancel can commit **integrity-only cleanup** without activation. | #428 stale focus/composition, valid and unavailable-context pointer release, target disappearance and bounded routed processing fixtures; assert at-most-once individual settlement. | Integrity poison/partial mutation cannot be `Unclaimed`/`Committed` normally; no retarget or activation on unavailable release; internal pointer re-hit emits no native receipt. | Existing routed/pointer rejection, cancellation and lifecycle trace compared with authoritative outcome when trace disabled. | M13ARB | owner-accepted | Required |
| M13ARB-06 | Observation records/snapshots are explicitly bounded by pump budget, checkpoint count, queue and pointer limits; capacity is preflighted before mutation; a mid-pump shortage returns all already committed records as a successful **paused** partial batch. Direct-publication transition storage follows the precommit rule in [ADR 0019](../adr/0019-event-correlated-host-input-arbitration.md#direct-publication-continuity). | #428 exact cardinality and allocation/fault-injection oracle, tiny capacities, zero work budget/readiness, repeated pumps, full upfront preflight or bounded partial progress with typed pause. Verify direct-publication capacity refusal preserves retained context and ownership revision; public zero/partial `AppRuntime::pump` proofs retain canonical ordering and wake behavior. | No silently dropped/duplicated record, error after earlier mutation, changed queue order, false `Quiescent` or lost wake, unbounded log, optional-trace dependency or overflowed revision. Publication observation-capacity failure must precede retained-context commit. | Per-pump count/sequence/revision/pause/report checks with trace disabled, redacted and truncated. | M13ARB | owner-accepted | Required |
| M13ARB-07 | Terminal/shutdown returns final closed ownership and one scope retirement marker invalidating all pending receipts; already settled receipts remain settled once. Direct terminal query and host disposal invalidate scope without fabricating Drop callbacks. | #428 explicit/repeated shutdown, direct submit terminal, readiness terminal at zero envelopes, `into_state` and slot dispose/cancel fixtures; preserve M4 cleanup order. | Queue's aggregate cancellation counts cannot become fake per-receipt statuses; error after irreversible shutdown, missing terminal marker, stale holds and double retirement are rejected. | Lifecycle/pointer/composition shutdown traces and explicit snapshot/status/retirement proof without trace. | M13ARB | owner-accepted | Required |
| M13ARB-08 | Host source-qualified press and continuously held gameplay state are gated by exact UI settlement and revisions; valid source B holds remain eligible while source A's same physical key is UI-owned. | #428 RunenUI public boundary + separately owned Runenwerk #1239 two-source/window W/Space/IME/modal fixed-tick action projection, Down-repeat-Up/Cancel and focus-loss proof. | No ghost held-key re-arm after modal closes, cross-source masking, source-erased `key_down_anywhere`, stuck key after source loss, fabricated text-commit key press or game mutation inside `UiApp::update`. | UI settlement and ownership proof plus Engine input-context/action admission/fixed-tick diagnostic oracle. | M13ARB | blocked | Required |
| M13ARB-09 | Logical retained RunenUI context and host-completed Present/display association remain distinct; input is mapped through exact runtime/surface/host-lifetime identity across publication, detach and rebind. | #428 public input-context proof and separately owned Runenwerk #1239 native target/Present association; delayed/reordered Present and valid prior retained generation or publication retry fixtures. | New logical publication without successful Present is not displayed; rejected/foreign/retired generation, lost device, unrelated runtime/overlay cannot silently retarget or acquire second focus. | Existing exact publication/hit generation and native completed-Present ledger; no trace-as-routing authority. | M13ARB | blocked | Required |
| M13ARB-10 | The target API is usable by independent public Rust consumers, generic widgets and real embedded/native hosts without private RunenUI types or a second authority, with complete canonical exact-head validation. | #428 public-only external crate with two typed apps/custom widget plus RunenUI canonical stable validation and separately reviewed Runenwerk/native host execution. | No private test seams, duplicate UI queue/focus, new core↔runtime cycle, OS/Engine dependency, bypassed host policy or claimed native execution from source inspection. | Public compile/negative conformance, repository architecture audit, trace-independent deterministic result and exact CI/native execution evidence as applicable. | M13ARB | blocked | Required |

## Gate and proof custody

The accepted RunenUI runtime/API slice qualifies `M13ARB-01`–`M13ARB-07`; `M13ARB-08`–`M13ARB-10` additionally require independently owned Runenwerk/native/external proof. No row is `owner-accepted` merely because this matrix or its ADR is merged. Before changing any status, qualify the complete positive, negative and diagnostic/trace evidence required by that row on the exact reviewed feature head, independent critical review, canonical validation, guarded merge and accepted-main verification. [Current status](../status.md) owns capability maturity; this matrix owns the exact observable and proof obligations.

## Accepted framework proof

The public-only [input fixtures](../../tests/external_widget/tests/input.rs), [pointer fixtures](../../tests/external_widget/tests/pointer.rs) and [external host consumer](../../tests/external_host/src/lib.rs) exercise the ordinary core/runtime contracts. [Two unrelated runtime slots](../../tests/external_widget/tests/input_arbitration_slot_families.rs) correlate all four native families; [causal finality](../../tests/external_widget/tests/input_arbitration_finality.rs), [capture transfer](../../tests/external_widget/tests/input_arbitration_pointer_route.rs) and [text capability withdrawal](../../tests/external_widget/tests/input_arbitration_text_boundary.rs) distinguish event-time settlement from subsequent work and effective ownership.

[Classification](../../tests/external_widget/tests/input_arbitration_classification.rs), [focus-group controls](../../tests/external_widget/tests/input_arbitration_menu.rs) and [range/scroll controls](../../tests/external_widget/tests/input_arbitration_controls.rs) establish supported claims and nonclaims with disabled, truncated and full trace. The [runtime arbitration fault tests](../../crates/runenui_runtime/src/app/input_arbitration_tests.rs) and [terminal integrity tests](../../crates/runenui_runtime/tests/terminal_integrity.rs) supply bounded reservation, revision and zero-envelope readiness-retirement evidence. Private fault seams remain excluded from public-consumer compilation.


## #474 — exact focus/text processing-finality qualification (candidate)

`UiInputProcessingRejection::FocusBindingInvalidated` and
`CommittedTextRecipientUnavailable` are distinct typed processing outcomes,
never positive game-input permission. A pending ambiguous text recipient is rejected
at admission with `SubmitTextErrorKind::PendingFocusHandoff` and receives no
settlement. Reached input retains its exact original scope/sequence; one ordered
M13 `InputSettled` is reported after the FIFO processing boundary even with
zero trace retention or partial pumps. No derived focus action creates an input
receipt. The registered #474 `focus_fifo_fence` suite is additive to M13ARB-01/02/05.


### #474 accelerator original-input finality distinction

The keyboard event's original `(InputArbitrationScope, WorkSequence)`
settles exactly once as `ProcessingRejected` when its source focus lifetime
retired before processing. Do not manufacture a later derived-command receipt
or trace on this path. The already-derived command remains ordinary canonical
FIFO work and is neither prematurely cancelled nor retrospectively retargeted.


### #474 focus allocator exhaustion and host finality

`RuntimeTerminalReason::FocusSessionExhausted` is distinct from
`WorkSequenceExhausted`; refusal at action preflight occurs before
`UiApp::update` and retires pending input scope under ordinary terminal
`ScopeRetired` semantics. No invented `InputSettled` receipt, native
device cancellation, second focused-input ownership or false gameplay
permission follows the terminal transition.
