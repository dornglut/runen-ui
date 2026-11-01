//! Explicit bounded pump and its single readiness-checkpoint authority.

#![allow(clippy::redundant_pub_crate)]

use crate::{
    RuntimeStatus, RuntimeTerminalReason, TraceRecordKind,
    queue::WorkEnvelope,
    runtime::{ProcessApplicationActionOutcome, Runtime, process_application_action},
};
use runenui_core::UiApp;

/// Processed-envelope limit for one explicit pump call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PumpBudget {
    processed_envelopes: usize,
    completion_imports: usize,
    local_polls: usize,
    timer_promotions: usize,
}

impl PumpBudget {
    #[must_use]
    pub const fn new(
        max_processed_envelopes: usize,
        max_completion_imports: usize,
        max_local_polls: usize,
        max_timer_promotions: usize,
    ) -> Self {
        Self {
            processed_envelopes: max_processed_envelopes,
            completion_imports: max_completion_imports,
            local_polls: max_local_polls,
            timer_promotions: max_timer_promotions,
        }
    }

    #[must_use]
    pub const fn max_processed_envelopes(self) -> usize {
        self.processed_envelopes
    }

    #[must_use]
    pub const fn max_completion_imports(self) -> usize {
        self.completion_imports
    }

    #[must_use]
    pub const fn max_local_polls(self) -> usize {
        self.local_polls
    }

    #[must_use]
    pub const fn max_timer_promotions(self) -> usize {
        self.timer_promotions
    }
}

/// Independent scheduler-budget exhaustion flags for one pump.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct PumpBudgetExhaustion {
    processed_envelopes: bool,
    completion_imports: bool,
    local_polls: bool,
    timer_promotions: bool,
}

impl PumpBudgetExhaustion {
    #[must_use]
    pub const fn processed_envelopes(self) -> bool {
        self.processed_envelopes
    }
    #[must_use]
    pub const fn completion_imports(self) -> bool {
        self.completion_imports
    }
    #[must_use]
    pub const fn local_polls(self) -> bool {
        self.local_polls
    }
    #[must_use]
    pub const fn timer_promotions(self) -> bool {
        self.timer_promotions
    }
    #[must_use]
    pub const fn any(self) -> bool {
        self.processed_envelopes
            || self.completion_imports
            || self.local_polls
            || self.timer_promotions
    }
}

/// Completion state of one bounded pump call.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PumpOutcome {
    Quiescent,
    BudgetExhausted,
    Closed,
    Terminal(RuntimeTerminalReason),
}

/// Exact observations from one bounded pump call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct PumpReport {
    processed_envelopes: usize,
    imported_completions: usize,
    polled_local_work: usize,
    promoted_timers: usize,
    remaining_queued_envelopes: usize,
    cancelled_by_terminal_transition: usize,
    exhausted_budgets: PumpBudgetExhaustion,
    completion_imports_pending: bool,
    due_timers_pending: bool,
    mandatory_derived_work_pending: bool,
    next_deadline: Option<crate::MonotonicInstant>,
    publication_dirty: bool,
    outcome: PumpOutcome,
}

impl PumpReport {
    #[must_use]
    pub const fn processed_envelopes(self) -> usize {
        self.processed_envelopes
    }
    #[must_use]
    pub const fn imported_completions(self) -> usize {
        self.imported_completions
    }
    #[must_use]
    pub const fn polled_local_work(self) -> usize {
        self.polled_local_work
    }
    #[must_use]
    pub const fn promoted_timers(self) -> usize {
        self.promoted_timers
    }
    #[must_use]
    pub const fn remaining_queued_envelopes(self) -> usize {
        self.remaining_queued_envelopes
    }
    #[must_use]
    pub const fn cancelled_by_terminal_transition(self) -> usize {
        self.cancelled_by_terminal_transition
    }
    #[must_use]
    pub const fn exhausted_budgets(self) -> PumpBudgetExhaustion {
        self.exhausted_budgets
    }
    #[must_use]
    pub const fn completion_imports_pending(self) -> bool {
        self.completion_imports_pending
    }
    #[must_use]
    pub const fn due_timers_pending(self) -> bool {
        self.due_timers_pending
    }
    #[must_use]
    pub const fn mandatory_derived_work_pending(self) -> bool {
        self.mandatory_derived_work_pending
    }
    #[must_use]
    pub const fn next_deadline(self) -> Option<crate::MonotonicInstant> {
        self.next_deadline
    }
    #[must_use]
    pub const fn publication_dirty(self) -> bool {
        self.publication_dirty
    }
    #[must_use]
    pub const fn outcome(self) -> PumpOutcome {
        self.outcome
    }
    #[must_use]
    pub const fn is_quiescent(self) -> bool {
        matches!(self.outcome, PumpOutcome::Quiescent)
    }
}

struct ReachedInput {
    sequence: runenui_core::WorkSequence,
    family: crate::UiInputFamily,
    surface_id: Option<runenui_core::SurfaceId>,
    device_id: Option<runenui_core::InputDeviceId>,
    pointer_id: Option<runenui_core::PointerId>,
    finality: crate::UiInputFinality,
}

struct RecordedPump {
    records: Vec<crate::InputArbitrationRecord>,
    latest: crate::InputOwnershipSnapshot,
    processed_through: Option<runenui_core::WorkSequence>,
    progress: bool,
    prepared: Option<crate::runtime::InputSnapshotReservation>,
    revision_retired: bool,
    cancelled_for_revision: usize,
    began_running: bool,
}

impl RecordedPump {
    fn new<State, Action, Protocol: runenui_core::HostProtocol>(
        runtime: &mut Runtime<State, Action, Protocol>,
    ) -> Result<Self, crate::InputObservationError> {
        let began_running = matches!(runtime.status(), RuntimeStatus::Running);
        let latest = runtime.input_ownership()?;
        Ok(Self {
            records: Vec::new(),
            latest,
            processed_through: None,
            // The initial projection may publish pending outside-pump revision
            // changes before the first readiness reservation. It is therefore
            // already observable progress: subsequent capacity failures MUST
            // return its snapshot instead of discarding it via a bare Err.
            progress: true,
            prepared: None,
            revision_retired: false,
            cancelled_for_revision: 0,
            began_running,
        })
    }

    /// Reserve three ordered records and all nested ownership snapshot storage
    /// before reaching the next canonical mutation/checkpoint.
    fn admit<State, Action, Protocol: runenui_core::HostProtocol>(
        &mut self,
        runtime: &mut Runtime<State, Action, Protocol>,
    ) -> Result<bool, crate::InputObservationError> {
        if self.records.try_reserve(3).is_err() {
            if self.progress {
                return Ok(false);
            }
            return Err(crate::InputObservationError::Capacity);
        }
        let reserved = match runtime.reserve_input_observation() {
            Ok(reserved) => reserved,
            Err(crate::InputObservationError::Capacity) if self.progress => return Ok(false),
            Err(crate::InputObservationError::RevisionExhausted) if self.progress => {
                // Reserve even the final terminal snapshot before changing
                // any runtime authority; a temporary capacity refusal pauses
                // without discarding earlier settled input.
                let terminal_reservation = match runtime.reserve_input_terminal_projection() {
                    Ok(reservation) => reservation,
                    Err(crate::InputObservationError::Capacity) => return Ok(false),
                    Err(error) => return Err(error),
                };
                self.cancelled_for_revision =
                    runtime.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                let (latest, transition) = runtime.input_ownership_reserved(terminal_reservation);
                if latest.revision() != self.latest.revision() {
                    self.records
                        .push(crate::InputArbitrationRecord::OwnershipChanged(
                            crate::InputOwnershipTransition {
                                before_revision: self.latest.revision(),
                                after: transition,
                            },
                        ));
                }
                self.latest = latest;
                if let Some(retirement) = runtime.input_retirement_record() {
                    self.records
                        .push(crate::InputArbitrationRecord::ScopeRetired(retirement));
                }
                self.revision_retired = true;
                return Ok(false);
            }
            Err(error) => return Err(error),
        };
        self.prepared = Some(reserved);
        Ok(true)
    }

    fn observe<State, Action, Protocol: runenui_core::HostProtocol>(
        &mut self,
        runtime: &mut Runtime<State, Action, Protocol>,
        input: Option<ReachedInput>,
    ) {
        let prepared = self
            .prepared
            .take()
            .unwrap_or_else(|| unreachable!("canonical boundary was pre-reserved"));
        let (latest, transition) = runtime.input_ownership_reserved(prepared);
        if latest.revision() != self.latest.revision() {
            self.records
                .push(crate::InputArbitrationRecord::OwnershipChanged(
                    crate::InputOwnershipTransition {
                        before_revision: self.latest.revision(),
                        after: transition,
                    },
                ));
        }
        if let Some(ReachedInput {
            sequence,
            family,
            surface_id,
            device_id,
            pointer_id,
            finality,
        }) = input
        {
            self.records
                .push(crate::InputArbitrationRecord::InputSettled(
                    crate::UiInputSettlement {
                        scope: latest.scope().clone(),
                        sequence,
                        family,
                        surface_id,
                        device_id,
                        pointer_id,
                        finality,
                        ownership_revision: latest.revision(),
                    },
                ));
        }
        self.latest = latest;
        if let Some(retirement) = runtime.input_retirement_record() {
            self.records
                .push(crate::InputArbitrationRecord::ScopeRetired(retirement));
        }
        self.progress = true;
    }

    fn finish<State, Action, Protocol: runenui_core::HostProtocol>(
        self,
        runtime: &mut Runtime<State, Action, Protocol>,
        budget: PumpBudget,
        processed: usize,
        cancelled: usize,
        totals: ReadinessTotals,
        paused: bool,
    ) -> crate::InputPumpBatch {
        let paused = paused && !self.revision_retired;
        if paused {
            runtime.request_input_observation_retry_wake();
        }
        // A routed input/semantic callback can terminalize indirectly while
        // its envelope handler reports Completed. Use the exact one-time count
        // already computed by canonical cleanup, with no second cancellation
        // or credit on a later pump over this retired lifetime.
        let cancelled =
            if self.began_running && matches!(runtime.status(), RuntimeStatus::Terminal(_)) {
                runtime
                    .terminal_cancelled_work()
                    .unwrap_or_else(|| unreachable!("terminal cleanup recorded its work losses"))
            } else {
                cancelled.saturating_add(self.cancelled_for_revision)
            };
        let mut report = finish_report(runtime, budget, processed, cancelled, totals);
        if paused && matches!(report.outcome, PumpOutcome::Quiescent) {
            report.outcome = PumpOutcome::BudgetExhausted;
        }
        crate::InputPumpBatch {
            report,
            ordered_records: self.records,
            processed_through: self.processed_through,
            final_ownership: self.latest,
            pause_reason: paused.then_some(crate::InputPumpPauseReason::ObservationCapacity),
        }
    }
}

/// Canonical in-order pump including exact native receipts, non-input owner
/// transitions, retirement and reached-work fence. This is the *only* executor.
///
/// For an envelope budget N, the loop reaches at most N envelopes and 2N+3
/// readiness checkpoints (including initial, empty-FIFO and final checks).
/// Each admitted boundary produces at most one ownership transition, each
/// external input at most one settlement, and the lifetime at most one
/// retirement: at most 4N+4 returned records. Every snapshot contains at most
/// `pointer_streams` pointers and one current logical surface. Temporary
/// snapshot buffers are reserved per boundary and released/replaced there;
/// only the caller's batch and the latest derived projection remain retained.
#[allow(clippy::too_many_lines)]
pub(crate) fn pump_recorded<App: UiApp>(
    runtime: &mut Runtime<App::State, App::Action, App::HostProtocol>,
    budget: PumpBudget,
) -> Result<crate::InputPumpBatch, crate::InputObservationError> {
    let mut observed = RecordedPump::new(runtime)?;
    let mut processed = 0usize;
    let mut cancelled = 0usize;
    let mut totals = ReadinessTotals::default();

    if !observed.admit(runtime)? {
        // Even with zero reached FIFO work, the initial immutable ownership
        // snapshot may have advanced a synchronous revision. Retain that fact
        // and preserve all queued receipts for a later exact retry.
        return Ok(observed.finish(runtime, budget, processed, cancelled, totals, true));
    }
    readiness_checkpoint(runtime, budget, &mut totals);
    observed.observe(runtime, None);
    while processed < budget.max_processed_envelopes() {
        if runtime.queue_is_empty() {
            if !observed.admit(runtime)? {
                return Ok(observed.finish(runtime, budget, processed, cancelled, totals, true));
            }
            readiness_checkpoint(runtime, budget, &mut totals);
            observed.observe(runtime, None);
            if runtime.queue_is_empty() {
                return Ok(observed.finish(runtime, budget, processed, cancelled, totals, false));
            }
        }
        if !observed.admit(runtime)? {
            return Ok(observed.finish(runtime, budget, processed, cancelled, totals, true));
        }
        let Some(envelope) = runtime.pop_work() else {
            // A canonical pop can be empty after its preflight; no envelope
            // was consumed. The previously reserved boundary is sufficient.
            readiness_checkpoint(runtime, budget, &mut totals);
            observed.observe(runtime, None);
            return Ok(observed.finish(runtime, budget, processed, cancelled, totals, false));
        };
        if runtime.queue.is_closed() {
            // Pop detected an invalid root ledger after removing the front
            // envelope. Do not invoke any mutable application or input callback.
            let cancelled = runtime.enter_terminal(RuntimeTerminalReason::Poisoned, 1);
            observed.observe(runtime, None);
            return Ok(observed.finish(runtime, budget, processed, cancelled, totals, false));
        }
        let sequence = envelope.sequence();
        let root_quota = envelope.root_quota();
        let application_action = matches!(&envelope, WorkEnvelope::ApplicationAction(_));
        let pointer_source = match &envelope {
            WorkEnvelope::Pointer(work) => match &work.payload {
                crate::queue::PointerEnvelopePayload::Event(event) => Some(event.pointer_id()),
                crate::queue::PointerEnvelopePayload::StationaryRehit(_) => None,
            },
            _ => None,
        };
        let (mut result, settled) = match envelope {
            WorkEnvelope::ApplicationAction(envelope) => {
                (process_application_action::<App>(runtime, envelope), None)
            }
            WorkEnvelope::SemanticCommand(envelope) => {
                runtime.process_semantic_command(envelope);
                (ProcessApplicationActionOutcome::Completed, None)
            }
            WorkEnvelope::ApplicationCommand(envelope) => {
                runtime.process_application_command(envelope);
                (ProcessApplicationActionOutcome::Completed, None)
            }
            WorkEnvelope::Pointer(envelope) => {
                let identity = match &envelope.payload {
                    crate::queue::PointerEnvelopePayload::Event(event) => Some((
                        crate::UiInputFamily::Pointer,
                        Some(event.surface_context().surface_id().clone()),
                        event.device_id(),
                        Some(event.pointer_id()),
                    )),
                    crate::queue::PointerEnvelopePayload::StationaryRehit(_) => None,
                };
                let (outcome, finality) = runtime.process_pointer_envelope(envelope);
                (
                    outcome,
                    identity
                        .zip(finality)
                        .map(
                            |((family, surface, device, pointer), finality)| ReachedInput {
                                sequence,
                                family,
                                surface_id: surface,
                                device_id: device,
                                pointer_id: pointer,
                                finality,
                            },
                        ),
                )
            }
            WorkEnvelope::Input(envelope) => {
                let (family, device) = match &envelope.payload {
                    crate::queue::InputEnvelopePayload::Keyboard(event) => {
                        (crate::UiInputFamily::Keyboard, event.device_id())
                    }
                    crate::queue::InputEnvelopePayload::CommittedText(event) => {
                        (crate::UiInputFamily::CommittedText, event.device_id())
                    }
                    crate::queue::InputEnvelopePayload::Composition(event) => (
                        crate::UiInputFamily::Composition,
                        match event {
                            runenui_core::CompositionEvent::Start(start) => start.device_id(),
                            _ if runtime.composition.generation() == Some(event.generation()) => {
                                runtime.composition.device_id()
                            }
                            _ => None,
                        },
                    ),
                };
                let finality = runtime.process_input_envelope(envelope);
                (
                    ProcessApplicationActionOutcome::Completed,
                    Some(ReachedInput {
                        sequence,
                        family,
                        surface_id: None,
                        device_id: device,
                        pointer_id: None,
                        finality,
                    }),
                )
            }
            WorkEnvelope::EffectStart(work) => {
                runtime.process_effect_start(work.sequence, work.generation);
                (ProcessApplicationActionOutcome::Completed, None)
            }
            WorkEnvelope::WorkCancellation(work) => {
                runtime.process_work_cancellation(
                    work.sequence,
                    work.generation,
                    work.identity,
                    work.causal_parent,
                );
                (ProcessApplicationActionOutcome::Completed, None)
            }
            WorkEnvelope::FrameworkServiceResponse(envelope) => (
                runtime
                    .process_framework_service_response(envelope)
                    .map_or(ProcessApplicationActionOutcome::Completed, |action| {
                        process_application_action::<App>(runtime, action)
                    }),
                None,
            ),
            WorkEnvelope::TimerFiring(work) => {
                runtime.process_timer_firing(work.sequence, work.generation, work.root_quota);
                (ProcessApplicationActionOutcome::Completed, None)
            }
            WorkEnvelope::MountedSubscriptionReconcile {
                sequence,
                owner,
                causal_parent,
                root_quota,
            } => {
                runtime.process_mounted_subscription_reconcile(
                    sequence,
                    &owner,
                    causal_parent,
                    root_quota,
                );
                (ProcessApplicationActionOutcome::Completed, None)
            }
        };
        if matches!(runtime.status(), RuntimeStatus::Running)
            && settled
                .as_ref()
                .is_some_and(|input| matches!(input.finality, crate::UiInputFinality::Committed(_)))
            && let (Some(pointer), Some(root)) = (pointer_source, root_quota)
            && runtime
                .bind_reached_pointer_continuations(root, sequence, pointer)
                .is_err()
        {
            runtime.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
        }
        if matches!(runtime.status(), RuntimeStatus::Running)
            && let Some(root) = root_quota
            && runtime
                .bind_reached_space_continuation(root, sequence)
                .is_err()
        {
            runtime.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
        }
        runtime.retire_input_continuation_claims();
        if !application_action
            && matches!(runtime.status(), RuntimeStatus::Running)
            && let Some(root_quota) = root_quota
            && runtime.queue.finish_root_envelope(root_quota).is_err()
        {
            let reason = RuntimeTerminalReason::Poisoned;
            let terminal_cancelled = runtime.enter_terminal(reason, 0);
            result = ProcessApplicationActionOutcome::Terminal {
                reason,
                cancelled: terminal_cancelled,
            };
        }
        processed += 1;
        observed.processed_through = Some(sequence);
        observed.observe(runtime, settled);
        if let ProcessApplicationActionOutcome::Terminal {
            reason,
            cancelled: terminal_cancelled,
        } = result
        {
            debug_assert_eq!(runtime.status(), RuntimeStatus::Terminal(reason));
            cancelled = terminal_cancelled;
            if !observed.admit(runtime)? {
                return Ok(observed.finish(runtime, budget, processed, cancelled, totals, true));
            }
            readiness_checkpoint(runtime, budget, &mut totals);
            observed.observe(runtime, None);
            return Ok(observed.finish(runtime, budget, processed, cancelled, totals, false));
        }
        if processed < budget.max_processed_envelopes() {
            if !observed.admit(runtime)? {
                return Ok(observed.finish(runtime, budget, processed, cancelled, totals, true));
            }
            readiness_checkpoint(runtime, budget, &mut totals);
            observed.observe(runtime, None);
        }
    }
    if !observed.admit(runtime)? {
        return Ok(observed.finish(runtime, budget, processed, cancelled, totals, true));
    }
    readiness_checkpoint(runtime, budget, &mut totals);
    observed.observe(runtime, None);
    Ok(observed.finish(runtime, budget, processed, cancelled, totals, false))
}

#[derive(Clone, Copy, Default)]
struct ReadinessTotals {
    imported: usize,
    polled: usize,
    promoted: usize,
}

fn readiness_checkpoint<State, Action, Protocol: runenui_core::HostProtocol>(
    runtime: &mut Runtime<State, Action, Protocol>,
    budget: PumpBudget,
    totals: &mut ReadinessTotals,
) {
    #[cfg(test)]
    runtime.note_readiness_checkpoint();
    let report = runtime.readiness_checkpoint(
        budget
            .max_completion_imports()
            .saturating_sub(totals.imported),
        budget.max_local_polls().saturating_sub(totals.polled),
        budget
            .max_timer_promotions()
            .saturating_sub(totals.promoted),
    );
    totals.imported = totals.imported.saturating_add(report.imported_completions);
    totals.polled = totals.polled.saturating_add(report.polled_local_work);
    totals.promoted = totals.promoted.saturating_add(report.promoted_timers);
}

fn finish_report<State, Action, Protocol: runenui_core::HostProtocol>(
    runtime: &mut Runtime<State, Action, Protocol>,
    budget: PumpBudget,
    processed: usize,
    cancelled: usize,
    totals: ReadinessTotals,
) -> PumpReport {
    let remaining = runtime.queued_len();
    let observation = runtime.scheduler_observation();
    let exhausted_budgets = PumpBudgetExhaustion {
        processed_envelopes: remaining > 0 && processed >= budget.max_processed_envelopes(),
        completion_imports: observation.completion_imports_pending
            && totals.imported >= budget.max_completion_imports(),
        local_polls: observation.local_polls_pending && totals.polled >= budget.max_local_polls(),
        timer_promotions: observation.due_timers_pending
            && totals.promoted >= budget.max_timer_promotions(),
    };
    let immediately_serviceable = remaining > 0
        || observation.completion_imports_pending
        || observation.due_timers_pending
        || observation.local_polls_pending
        || observation.mandatory_derived_work_pending;
    let outcome = match runtime.status() {
        RuntimeStatus::Closed => PumpOutcome::Closed,
        RuntimeStatus::Terminal(reason) => PumpOutcome::Terminal(reason),
        RuntimeStatus::Running if exhausted_budgets.any() || immediately_serviceable => {
            runtime.record_optional(TraceRecordKind::PumpBudgetExhausted, None, None, None);
            PumpOutcome::BudgetExhausted
        }
        RuntimeStatus::Running => PumpOutcome::Quiescent,
    };
    PumpReport {
        processed_envelopes: processed,
        imported_completions: totals.imported,
        polled_local_work: totals.polled,
        promoted_timers: totals.promoted,
        remaining_queued_envelopes: remaining,
        cancelled_by_terminal_transition: cancelled,
        exhausted_budgets,
        completion_imports_pending: observation.completion_imports_pending,
        due_timers_pending: observation.due_timers_pending,
        mandatory_derived_work_pending: observation.mandatory_derived_work_pending,
        next_deadline: observation.next_deadline,
        publication_dirty: observation.publication_dirty,
        outcome,
    }
}

#[cfg(test)]
mod tests {
    #![allow(refining_impl_trait)]

    use runenui_core::{Element, NoHostProtocol, UiApp, View, text};

    use super::{PumpBudget, pump_recorded};
    use crate::{
        RuntimeConfig, RuntimeStatus, RuntimeTerminalReason, TraceActionCategory, runtime::Runtime,
    };

    struct App;

    impl UiApp for App {
        fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
            runenui_core::ProducerBudget::new(1, 0)
                .with_reconciliation(1, 0, 1)
                .with_subscriptions(0, 0)
                .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
        }

        fn action_producer_budget(
            _state: &Self::State,
            _action: &Self::Action,
        ) -> runenui_core::ProducerBudget {
            runenui_core::ProducerBudget::new(1, 0)
                .with_subscriptions(0, 0)
                .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
        }

        fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
            runenui_core::ProducerBudget::new(1, 0)
        }

        type State = usize;
        type Action = ();
        type HostProtocol = NoHostProtocol;

        fn root(_: &Self::State) -> Element<Self::Action> {
            text("root").into_element()
        }

        fn update(state: &mut Self::State, (): Self::Action) {
            *state += 1;
        }
    }

    #[test]
    fn processed_fence_and_shutdown_retirement_are_exact_and_nonrepeating() {
        let mut runtime = Runtime::mount(
            0,
            App::root,
            RuntimeConfig::default(),
            App::initial_producer_budget(&0),
            |_, _| {
                Ok(crate::runtime::PreparedInitialApplicationWork {
                    application: crate::transaction::TransactionLedger::from_outputs(Vec::new(), 0)
                        .unwrap_or_else(|_| unreachable!("empty initial ledger fits")),
                    subscriptions: Vec::new(),
                })
            },
        )
        .unwrap_or_else(|_| unreachable!("internal test mount inputs are valid"));
        runtime.initialize_application_work();
        let mounted = super::pump_recorded::<App>(
            &mut runtime,
            PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX),
        )
        .unwrap_or_else(|_| unreachable!("initial mounted work settles before the action fence"));
        assert!(mounted.report().is_quiescent());
        let receipt = runtime
            .submit_action((), TraceActionCategory::DirectSubmission, None, None)
            .unwrap_or_else(|_| unreachable!("bounded action ingress succeeds"));
        let work = super::pump_recorded::<App>(&mut runtime, PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("bounded processing observation succeeds"));
        assert_eq!(work.processed_through(), Some(receipt));
        assert_eq!(work.report().processed_envelopes(), 1);
        assert_eq!(work.ordered_records(), []);

        let _ = runtime.shutdown();
        let retired = super::pump_recorded::<App>(&mut runtime, PumpBudget::new(0, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("shutdown requires no individual cancelled receipt"));
        assert_eq!(retired.processed_through(), None);
        assert!(matches!(
            retired.ordered_records(),
            [crate::InputArbitrationRecord::ScopeRetired(_)]
        ));
        let again = super::pump_recorded::<App>(&mut runtime, PumpBudget::new(0, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("observed retirement stays retired"));
        assert_eq!(again.ordered_records(), []);
    }

    #[test]
    fn invalid_root_pop_closes_before_application_mutation_or_false_processing_receipt() {
        let mut runtime = Runtime::mount(
            0,
            App::root,
            RuntimeConfig::default(),
            App::initial_producer_budget(&0),
            |_, _| {
                Ok(crate::runtime::PreparedInitialApplicationWork {
                    application: crate::transaction::TransactionLedger::from_outputs(Vec::new(), 0)
                        .unwrap_or_else(|_| unreachable!("empty initial ledger fits")),
                    subscriptions: Vec::new(),
                })
            },
        )
        .unwrap_or_else(|_| unreachable!("internal test mount inputs are valid"));
        runtime.initialize_application_work();
        let mounted = pump_recorded::<App>(
            &mut runtime,
            PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX),
        )
        .unwrap_or_else(|_| unreachable!("initial work settles"));
        assert!(mounted.report().is_quiescent());
        runtime
            .submit_action((), TraceActionCategory::DirectSubmission, None, None)
            .unwrap_or_else(|_| unreachable!("qualified action admission"));
        runtime.queue.invalidate_front_root_pending_for_test();

        let batch = pump_recorded::<App>(&mut runtime, PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("terminal observation is retained"));
        assert_eq!(*runtime.state(), 0, "invalid pop must not invoke UiApp::update");
        assert_eq!(runtime.status(), RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned));
        assert_eq!(batch.report().processed_envelopes(), 0);
        assert_eq!(batch.report().cancelled_by_terminal_transition(), 1);
        assert_eq!(batch.processed_through(), None);
    }

    #[test]
    fn one_checkpoint_authority_runs_before_and_at_the_final_boundary() {
        let mut runtime = Runtime::mount(
            0,
            App::root,
            RuntimeConfig::default(),
            App::initial_producer_budget(&0),
            |_, _| {
                Ok(crate::runtime::PreparedInitialApplicationWork {
                    application: crate::transaction::TransactionLedger::from_outputs(Vec::new(), 0)
                        .unwrap_or_else(|_| unreachable!("empty initial ledger fits")),
                    subscriptions: Vec::new(),
                })
            },
        )
        .unwrap_or_else(|_| unreachable!("internal test mount inputs are valid"));
        runtime.initialize_application_work();
        runtime
            .submit_action((), TraceActionCategory::DirectSubmission, None, None)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(runtime.readiness_checkpoint_count_for_test(), 0);
        let report = pump_recorded::<App>(
            &mut runtime,
            PumpBudget::new(0, usize::MAX, usize::MAX, usize::MAX),
        )
        .unwrap_or_else(|_| unreachable!("bounded pump snapshot"));
        assert_eq!(report.report().processed_envelopes(), 0);
        assert_eq!(runtime.readiness_checkpoint_count_for_test(), 2);
        let report = pump_recorded::<App>(
            &mut runtime,
            PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX),
        )
        .unwrap_or_else(|_| unreachable!("bounded pump snapshot"));
        assert_eq!(report.report().processed_envelopes(), 1);
        assert_eq!(runtime.readiness_checkpoint_count_for_test(), 4);
    }
}
