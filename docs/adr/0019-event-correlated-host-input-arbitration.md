# ADR 0019: Event-correlated host input arbitration

> **Category:** ADR
>
> **Status:** Accepted target architecture upon owner-reviewed squash merge; **not implemented**
>
> **Milestone:** M13
>
> **Decision authority:** RunenUI input arbitration investigation #425; durable acceptance #427
>
> **Implementation:** separate blocked successor #428
>
> **Inherits:** [ADR 0005](0005-canonical-event-routing-and-commands.md), [ADR 0006](0006-effects-scheduling-and-trace-v2.md), [ADR 0018](0018-editable-text-and-interaction-services.md)

## Decision and motivation

An embedded game/application host must arbitrate ordinary native occurrences against UI routing **without reconstructing RunenUI focus, semantics, hit testing, captures, editing, shortcuts or presentation modality**. An existing `submit_*` success reports **queue admission**, not committed handling. `AppRuntime::focus` and `focused_text_input_capability` are partial current-state projections. Optional, bounded and redacted trace is diagnostic, never a gameplay authority.

Accept one **runtime-authored, event-correlated arbitration contract** over the existing canonical queue. The framework certifies *UI interaction conflict facts*; only the host/game owns native source identity, gameplay binding policy, physical-key lifetimes, held-action projection, fixed-tick admission and OS cursor modes. This ADR is the target public contract and does not claim the following API exists in current Rust.

## Authority boundaries

- `runenui_core` owns neutral authored `EventContext::claim_host_input()` vocabulary and opaque protocol value definitions; never a second focus or input manager.
- `runenui_runtime` owns one sequenced FIFO, mounted identity, routing, callback/default transaction, editing/composition, pressed/captured pointer, presentation modal barrier, reconciliation, logical publication, arbitration facts, ownership revision and terminal lifecycle.
- Native adapters translate physical facts and OS lifecycle. They never determine widget or game policy.
- Host/Runenwerk correlates `(InputArbitrationScope, WorkSequence)` to exact native window/source/physical occurrence, chooses which UI slot gets that occurrence, maps selected **completed Present** to displayed hit context, determines conflicting gameplay eligibility and owns its source-qualified held-key reducer. No global `ui_consumed` or source-erased `physical_key_down_anywhere` game-state projection.
- #353 independently owns future multiple surfaces/roots/focus; #307/#424 own relative/confined native pointer modes. This decision must not pretend those are implemented by arbitration.

## Input admission, processing and finality

Existing typed `submit_keyboard`, `submit_pointer`, `submit_text` and composition ingress keep their accepted receipts and rejection errors. A successful receipt carries the existing nonwrapping `WorkSequence`. An input is **Pending** until its exact sequence settles or its runtime scope is retired. Rejected admission produces **no** fabricated processing settlement; rejection or missing focus is not evidence that gameplay is permitted.

Each *reached host-submitted input envelope* yields at most one `UiInputSettlement` at its truthful final processing boundary, with `UiInputFamily`, exact `(scope, sequence)`, relevant optional surface/device/pointer identity, finality and the **effective post-operation ownership revision**:

- `Committed(UiInputRoutingFacts)` means the authoritative input operation committed, including an **integrity-only release**; it does **not** imply an actionable widget default, accepted application action, or transitive completion of commands enqueued afterward.
- `ProcessingRejected(UiInputProcessingRejection)` means no claimed routing/default commit, with exact missing/stale/foreign target, incompatible surface context, invalid stream, stale composition generation or bounded preflight cause when applicable.
- `Aborted(UiInputAbortReason)` means processing integrity cannot certify a normal input commit, including terminal poison/partial mutation; host conflict is `Undetermined` and must not be mapped to game eligibility.

An **unavailable-context pointer Up/Cancel is not automatically rejected**: accepted M4 release integrity may commit button-set/capture/stream cleanup with no retarget, hit-test or activation. Report `Committed` with its non-actionable release facts if that cleanup commits; only truly rejected operations use `ProcessingRejected`. Internal stationary pointer re-hit, derived commands and focus notifications can advance ownership and work fences but **never invent a native input receipt**.

The queue's existing terminal `cancel_all` erases queued envelopes and retains counts, not individual input identities. Therefore exactly one `ScopeRetired` fact for the exact runtime lifetime invalidates **all not-yet-settled host receipt correlations in that scope**; do not attempt after-the-fact per-receipt queue-cancellation reporting or add a second ledger. Host processes already emitted individual settlements once and discards all still-pending associations on retirement. `Drop` cannot emit a Rust return value: owner of the runtime slot MUST invalidate its scope and matching held actions synchronously when disposing it.

## Certified UI conflict versus host gameplay

The immutable committed `UiInputRoutingFacts` exposes `UiInputConflict` with four mutually distinct classifications: `Unclaimed`, `ObservedNonexclusive`, `ExclusiveUi` and `Undetermined`. Nonexclusive hover/hit/callback routing is *not* a conflict claim. The host may make a stricter game-input policy, but cannot reinterpret RunenUI's private widget semantics.

Exclusive UI claims require actual runtime-certified reasons, including focused text/IME domain, semantic navigation/Space activation, accepted scope-specific shortcut/default, pointer pressed owner/capture/drag/text-selection, touch gesture, modal outside blocking/dismissal, and an ordinary downstream widget's explicit `EventContext::claim_host_input()`. Generic custom widget claim is independent of propagation and default prevention and applies only to the currently routed *external input event*. The runtime OR-aggregates it transactionally; no callback-time host side channel or special built-in control registry.

`stop_propagation`, `prevent_default`, arbitrary callback invocation, ordinary hover/hit, diagnostic `PointerDefaultApplied` on a no-op Move/Cancel, and a queued-but-not-yet-executed application action **never independently prove `ExclusiveUi`**. Default/shortcut status and propagation remain orthogonal typed facts. Do not leak sensitive committed text, secret preedit or payload into host observations.

## One ordered observation stream, not two replay lists

`InputArbitrationRecord` has `InputSettled`, `OwnershipChanged` and `ScopeRetired`. A single `InputPumpBatch::ordered_records()` array reflects the actual runtime processing/checkpoint **commit order**. No sort by timestamp or native occurrence, and no second queue.

An input may itself synchronously change focus, composition, Space ownership or pointer capture. In that case its **committed `OwnershipChanged` precedes its own `InputSettled`**, whose `ownership_revision` names the new revision. A later derived semantic command/application action is a separate canonical envelope and emits later revisions when *it* commits. A settlement is never a claim that causal descendants have drained. `processed_through` names the last actually processed work sequence (if any), **not** queued, cancelled or all transitive work. `final_ownership` agrees with the last externally observable revision at batch return.

The host MUST gate only conflicting native/game actions until required UI decisions are settled. An unresolved pending receipt, unknown required owner state, terminal scope or processing failure may not turn into game permission. A budget-exhausted pump is progress, not a negative UI decision. Runenwerk's action reducer maintains eligible game Down **per source/window/device/press lifetime** and evaluates both press edges and continuously held action state. UI focus/modal changes revoke incompatible game holds; physically still-held keys never ghost-reactivate when a menu closes. Up/Cancel/source/window loss retires the matching game hold independent of UI admission; a separate eligible source's W remains valid.

## Read-only revisioned ownership, including non-pump boundaries

`InputArbitrationScope` is opaque, unforgeable by a host and unique per mounted runtime lifetime. Work sequences are **not globally unique** across scopes. `InputOwnershipRevision` changes only for meaningful externally observable UI arbitration ownership and never wraps; checked exhaustion terminalizes/invalidate scope instead of reusing an old revision.

`InputOwnershipSnapshot` exposes read-only runtime status, exact scope/revision, focused mounted owner, host-neutral `WidgetTextInput`, pending and active composition, Space activation owner, per-pointer pressed/captured identities, modal presentation barrier and retained surface input-context readiness. The relevant runtime owns each value. A snapshot MUST NOT include engine source/window policy, cursor grab or game-action mapping.

Not all mutations happen during pump. `start_composition` binds a pending generation during accepted ingress; public surface publication changes retained input context; explicit or direct terminal, shutdown, reconciliation and other synchronous operations can change ownership. `AppRuntime::input_ownership()` MUST observe the revision and status after those operations, even without an intervening pump or newly admitted input. The host MUST query it after relevant synchronous mutation boundaries and before dependent game projection. If intermediate non-pump state transitions are coalesced into a later snapshot, the host conservatively revokes eligibility when a revision gap prevents continuity proof; it cannot assume skipped states were absent.

**Logical RunenUI publication is not completed native Present.** A `SurfaceInputOwnership::latest_retained_context()` describes a retained RunenUI snapshot only. Its presence does not certify displayed pixels; the host owns completed-Present→actually displayed snapshot mapping and must route pointer ingress with that **exact** compatible context, including delayed/reordered/rejected Present and detach. Do not name a retained candidate `displayed_context` or promote it on publication alone.

## Boundedness, partial progress and observation failure

No global settlement backlog, second event queue, producer callback, trace dependency or unbounded receipt ledger. The output is **owned by its caller** per pump/shutdown; its input-settlement cardinality is at most the number of reached external input envelopes. Ownership transitions consolidate at most once per committed processing/checkpoint boundary; snapshot pointer cardinality is bounded by `RuntimeLimits::pointer_streams` and hosted work/checkpoints by `PumpBudget` and existing capacity limits. Include terminal retirement in the preflight bound; overflow never silently drops a required fact.

Required observation capacity MUST be admitted **before** the corresponding readiness checkpoint, envelope pop, transaction or shutdown mutation. If an initial reservation fails before any mutation, `Err(InputObservationError::Capacity)` is permitted. If an earlier envelope/checkpoint **already committed in this same call**, a later capacity shortage MUST return `Ok(InputPumpBatch)` containing all prior records, a precise `pause_reason() == Some(InputPumpPauseReason::ObservationCapacity)`, and leave the next boundary untouched. A proved whole-call worst-case reservation *before any mutation* is an alternative. A plain error after partial progress would destroy receipts and is forbidden. A paused report cannot claim `Quiescent` or suppress the re-wake obligation. Observation allocation or checked-revision exhaustion fails closed; do not manufacture `Unclaimed`.

Explicit `shutdown()` is **terminal**: reserve its bounded final/retirement observations before irreversible cleanup, preserve existing M4 composition/pointer/Space/focus ordering and honest suppressed callback facts, return immutable final closed ownership and a single scope retirement. A fallible shutdown MUST NOT mutate/close then return an error that loses its retirement evidence. Repeated shutdown is idempotent; direct terminal during other public ingress is visible from `input_ownership().status()` without another pump. `into_state` and owner-driven runtime removal follow the same retirement responsibility.

## Public API target and evolution

The following signatures and accessor names are the **selected target**, not a claim about current Rust exports. Fields of returned types remain private with checked runtime construction. New public enums are non-exhaustive; an unknown future variant fails closed in host arbitration. Existing `submit_*` signatures and `WorkSequence` remain. No compatibility-only alternate pump or independent `pump_with_input_arbitration` path.

```rust
// runenui_core, downstream-widget authoring:
EventContext::<Action>::claim_host_input(&mut self);

// runenui_runtime: target clean public cutover:
AppRuntime::<App>::input_ownership(
    &mut self,
) -> Result<InputOwnershipSnapshot, InputObservationError>;
AppRuntime::<App>::pump(
    &mut self, budget: PumpBudget,
) -> Result<InputPumpBatch, InputObservationError>;
AppRuntime::<App>::shutdown(
    &mut self,
) -> Result<InputShutdownBatch, InputObservationError>;

// Target public types:
InputArbitrationScope; InputOwnershipRevision; InputOwnershipSnapshot;
KeyboardInputOwnership; PointerInputOwnership; SurfaceInputOwnership;
InputOwnershipTransition; UiInputSettlement; UiInputFamily;
UiInputFinality; UiInputProcessingRejection; UiInputAbortReason;
UiInputRoutingFacts; UiInputConflict; UiInputClaimReason; UiInputRoute;
UiDefaultDisposition; InputScopeRetirement; InputScopeRetirementReason;
InputArbitrationRecord; InputPumpBatch; InputPumpPauseReason;
InputShutdownBatch; InputObservationError;

// InputArbitrationRecord:
//   InputSettled(UiInputSettlement)
//   OwnershipChanged(InputOwnershipTransition)
//   ScopeRetired(InputScopeRetirement)
//
// UiInputFinality:
//   Committed(UiInputRoutingFacts)
//   ProcessingRejected(UiInputProcessingRejection)
//   Aborted(UiInputAbortReason)
//
// UiInputConflict:
//   Unclaimed | ObservedNonexclusive | ExclusiveUi | Undetermined
//
// InputObservationError:
//   Capacity | RevisionExhausted
```

Required public accessors: `InputOwnershipSnapshot::{scope,revision,status,keyboard,surfaces,pointers}`; `InputOwnershipTransition::{before_revision,after}`; `UiInputSettlement::{scope,sequence,family,surface_id,device_id,pointer_id,finality,ownership_revision}`; `UiInputRoutingFacts::{conflict,reasons,route,propagation_stopped,default_prevented,default_disposition}`; `InputPumpBatch::{report,ordered_records,processed_through,final_ownership,pause_reason}`; `InputShutdownBatch::{report,ordered_records,final_ownership}`; `InputScopeRetirement::{scope,reason}`; `SurfaceInputOwnership::{surface_id,latest_retained_context,modal_blocker}`. Neutral keyboard and pointer projections must expose existing `WidgetTextInput`, composition/Space owner and exact active pointer pressed/capture owner respectively. Do not assert `Send`/`Sync`, OS source identity, host cursor controls or downstream availability without proof.

The later implementation #428 owns **exact compile-time signatures**, required trait derivations and legal method placement in the public crates as a clean cutover. Any required semantic departure from this target is an owner-reviewed ADR change; mechanical signature adjustments preserving this exact ownership/ordering/finality law must be documented in the implementation review and external consumer proof.

## Tradeoffs, counterexamples and conformance

Rejected: a synchronous `will_consume` preflight that predicts future FIFO callbacks, one global consumed boolean, host reimplementation of widget hit/modal/focus, pushing every game/raw input through UI, a second arbitration queue, and debug trace as a required settlement channel. Bounded immutable results and snapshot projection add API surface and finite per-pump allocation to avoid hidden duplicated semantic state.

Independent critical review of #425 corrected M13-ARB-001 through M13-ARB-010: ordered mixed records; outside-pump mutations; queue-erased terminal receipts; readiness/follow-up fences; no-op pointer-default ambiguity; typed negative/poison paths; runtime namespace; generic widget claim and built-in ownership; retained versus displayed publication; partial-result capacity loss.

The [M13 conformance matrix](../conformance/m13-conformance-matrix.md) is the **single permanent observable proof authority**, rows `M13ARB-01`–`M13ARB-10`. **All ten remain `blocked` on acceptance of this target ADR**; they require separately accepted executable public positive/negative/diagnostic evidence and applicable Runenwerk/native integration before promotion. Current API maturity remains owned by source/tests and [status](../status.md), not by this target decision.
