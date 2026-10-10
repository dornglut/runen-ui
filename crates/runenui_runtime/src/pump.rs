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

/// One canonical execution path; the outer legacy report adapter is temporary
/// draft-only until #429/M11 shared-file serialization permits the public cutover.
/// It must be removed before #428 acceptance.
pub(crate) fn pump<App: UiApp>(
    runtime: &mut Runtime<App::State, App::Action, App::HostProtocol>,
    budget: PumpBudget,
) -> PumpReport {
    match pump_recorded::<App>(runtime, budget) {
        Ok(batch) => *batch.report(),
        Err(_) => {
            // A legacy report-only caller cannot receive a typed capacity
            // error. Fail closed instead of claiming successful game admission.
            let cancelled = runtime.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            finish_report(runtime, budget, 0, cancelled, ReadinessTotals::default())
        }
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
}

impl RecordedPump {
    fn new<State, Action, Protocol: runenui_core::HostProtocol>(
        runtime: &mut Runtime<State, Action, Protocol>,
    ) -> Result<Self, crate::InputObservationError> {
        Ok(Self {
            records: Vec::new(),
            latest: runtime.input_ownership()?,
            processed_through: None,
            progress: false,
        })
    }

    /// Reserve a strict upper bound of three ordered records *before* the boundary.
    /// A pending storage failure never consumes another canonical envelope.
    fn admit(&mut self) -> Result<bool, crate::InputObservationError> {
        if self.records.try_reserve(3).is_err() {
            if self.progress {
                return Ok(false);
            }
            return Err(crate::InputObservationError::Capacity);
        }
        Ok(true)
    }

    fn observe<State, Action, Protocol: runenui_core::HostProtocol>(
        &mut self,
        runtime: &mut Runtime<State, Action, Protocol>,
        input: Option<ReachedInput>,
    ) -> Result<(), crate::InputObservationError> {
        let latest = runtime.input_ownership()?;
        if latest.revision() != self.latest.revision() {
            self.records
                .push(crate::InputArbitrationRecord::OwnershipChanged(
                    crate::InputOwnershipTransition {
                        before_revision: self.latest.revision(),
                        after: latest.clone(),
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
        Ok(())
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
#[allow(clippy::too_many_lines)]
pub(crate) fn pump_recorded<App: UiApp>(
    runtime: &mut Runtime<App::State, App::Action, App::HostProtocol>,
    budget: PumpBudget,
) -> Result<crate::InputPumpBatch, crate::InputObservationError> {
    let mut observed = RecordedPump::new(runtime)?;
    let mut processed = 0usize;
    let mut cancelled = 0usize;
    let mut totals = ReadinessTotals::default();

    if !observed.admit()? {
        unreachable!("initial admission has no prior progress");
    }
    readiness_checkpoint(runtime, budget, &mut totals);
    observed.observe(runtime, None)?;
    while processed < budget.max_processed_envelopes() {
        if runtime.queue_is_empty() {
            if !observed.admit()? {
                return Ok(observed.finish(runtime, budget, processed, cancelled, totals, true));
            }
            readiness_checkpoint(runtime, budget, &mut totals);
            observed.observe(runtime, None)?;
            if runtime.queue_is_empty() {
                return Ok(observed.finish(runtime, budget, processed, cancelled, totals, false));
            }
        }
        if !observed.admit()? {
            return Ok(observed.finish(runtime, budget, processed, cancelled, totals, true));
        }
        let Some(envelope) = runtime.pop_work() else {
            // A popped envelope is never silently lost. An empty pop only
            // happens after an ordinary checkpoint; preserve its work fence.
            if !observed.admit()? {
                return Ok(observed.finish(runtime, budget, processed, cancelled, totals, true));
            }
            readiness_checkpoint(runtime, budget, &mut totals);
            observed.observe(runtime, None)?;
            return Ok(observed.finish(runtime, budget, processed, cancelled, totals, false));
        };
        let sequence = envelope.sequence();
        let (result, settled) = match envelope {
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
                runtime.process_timer_firing(work.sequence, work.generation);
                (ProcessApplicationActionOutcome::Completed, None)
            }
            WorkEnvelope::MountedSubscriptionReconcile {
                sequence,
                owner,
                causal_parent,
            } => {
                runtime.process_mounted_subscription_reconcile(sequence, &owner, causal_parent);
                (ProcessApplicationActionOutcome::Completed, None)
            }
        };
        processed += 1;
        observed.processed_through = Some(sequence);
        observed.observe(runtime, settled)?;
        if let ProcessApplicationActionOutcome::Terminal {
            reason,
            cancelled: terminal_cancelled,
        } = result
        {
            debug_assert_eq!(runtime.status(), RuntimeStatus::Terminal(reason));
            cancelled = terminal_cancelled;
            if !observed.admit()? {
                return Ok(observed.finish(runtime, budget, processed, cancelled, totals, true));
            }
            readiness_checkpoint(runtime, budget, &mut totals);
            observed.observe(runtime, None)?;
            return Ok(observed.finish(runtime, budget, processed, cancelled, totals, false));
        }
        if processed < budget.max_processed_envelopes() {
            if !observed.admit()? {
                return Ok(observed.finish(runtime, budget, processed, cancelled, totals, true));
            }
            readiness_checkpoint(runtime, budget, &mut totals);
            observed.observe(runtime, None)?;
        }
    }
    if !observed.admit()? {
        return Ok(observed.finish(runtime, budget, processed, cancelled, totals, true));
    }
    readiness_checkpoint(runtime, budget, &mut totals);
    observed.observe(runtime, None)?;
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

    use super::{PumpBudget, pump};
    use crate::{RuntimeConfig, TraceActionCategory, runtime::Runtime};

    struct App;

    impl UiApp for App {
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
        let mut runtime = Runtime::mount(0, App::root, RuntimeConfig::default());
        let receipt = runtime
            .submit_action((), TraceActionCategory::DirectSubmission, None, None)
            .unwrap_or_else(|_| unreachable!("bounded action ingress succeeds"));
        let work = super::pump_recorded::<App>(&mut runtime, PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("bounded processing observation succeeds"));
        assert_eq!(work.processed_through(), Some(receipt));
        assert_eq!(work.report().processed_envelopes(), 1);
        assert!(work.ordered_records().is_empty());

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
        assert!(again.ordered_records().is_empty());
    }

    #[test]
    fn one_checkpoint_authority_runs_before_and_at_the_final_boundary() {
        let mut runtime = Runtime::mount(0, App::root, RuntimeConfig::default());
        runtime
            .submit_action((), TraceActionCategory::DirectSubmission, None, None)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(runtime.readiness_checkpoint_count_for_test(), 0);
        let report = pump::<App>(
            &mut runtime,
            PumpBudget::new(0, usize::MAX, usize::MAX, usize::MAX),
        );
        assert_eq!(report.processed_envelopes(), 0);
        assert_eq!(runtime.readiness_checkpoint_count_for_test(), 2);
        let report = pump::<App>(
            &mut runtime,
            PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX),
        );
        assert_eq!(report.processed_envelopes(), 1);
        assert_eq!(runtime.readiness_checkpoint_count_for_test(), 4);
    }
}
