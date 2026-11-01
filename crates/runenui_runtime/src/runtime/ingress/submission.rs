use runenui_core::{
    ApplicationCommandId, EventSource, FocusReason, MonotonicInstant, ProducerBudget,
    SemanticActionTarget,
};

use crate::{
    TraceActionCategory, TraceActionIdentity, TraceContext, TraceSurfaceContext,
    TraceSurfaceIngressKind,
    queue::{
        ApplicationActionOrigin, ApplicationCommandQueuePayload, FocusRequestOverride,
        QueueReservation, QueueReservationError, RootQuotaId, SemanticCommandQueuePayload,
        SemanticCommandQueueTarget,
    },
    trace::{TraceRecordDraft, TraceReservation},
};

use super::{
    ActionCommitError, CommandOrigin, CommandSubmission, HashMap, HostProtocol, MandatoryTracePlan,
    MountedNodeId, Runtime, RuntimeStatus, RuntimeTerminalReason, SemanticCommand,
    SubmitActionError, SubmitActionResult, SubmitCommandError, SubmitCommandErrorKind,
    SubscriptionDiagnostic, TargetStatus, TraceRecordKind, TraceSequence, TraceTarget,
    TraceWorkIdentity, UnacceptedCommand, WorkEnvelope, WorkSequence,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SurfaceCommandTrace {
    ingress: TraceSurfaceIngressKind,
    surface: TraceSurfaceContext,
}

impl SurfaceCommandTrace {
    pub(super) const fn new(
        ingress: TraceSurfaceIngressKind,
        surface: TraceSurfaceContext,
    ) -> Self {
        Self { ingress, surface }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CommandTrace {
    Direct { parent: Option<TraceSequence> },
    Surface(SurfaceCommandTrace),
}

struct PreflightedCommand<'a> {
    target: &'a MountedNodeId,
    command: SemanticCommand,
    origin: CommandOrigin,
    semantic_target: Option<SemanticActionTarget>,
    focus_request_override: Option<FocusRequestOverride>,
    presentation_cancel_origin: Option<super::super::presentation::PresentationCancelOrigin>,
    instant: MonotonicInstant,
    trace: CommandTrace,
}

#[derive(Clone, Copy, Debug, Default)]
struct InteractionRouteBudget {
    outputs: usize,
    starts: [usize; 5],
    future_credits: usize,
}

impl InteractionRouteBudget {
    fn include_widget_budget(
        &mut self,
        budget: runenui_core::WidgetProducerBudget,
        count: usize,
    ) -> bool {
        let Some(phase_outputs) = budget.outputs().checked_mul(count) else {
            return false;
        };
        let Some(outputs) = self.outputs.checked_add(phase_outputs) else {
            return false;
        };
        self.outputs = outputs;
        for (index, family) in [
            runenui_core::WorkFamily::LocalTask,
            runenui_core::WorkFamily::SendTask,
            runenui_core::WorkFamily::Timer,
            runenui_core::WorkFamily::HostRequest,
            runenui_core::WorkFamily::FrameworkService,
        ]
        .into_iter()
        .enumerate()
        {
            let Some(phase_starts) = budget.work_starts().for_family(family).checked_mul(count)
            else {
                return false;
            };
            let Some(starts) = self.starts[index].checked_add(phase_starts) else {
                return false;
            };
            self.starts[index] = starts;
            let future_per_start = match family {
                runenui_core::WorkFamily::LocalTask
                | runenui_core::WorkFamily::SendTask
                | runenui_core::WorkFamily::HostRequest => 1,
                runenui_core::WorkFamily::Timer => 3,
                runenui_core::WorkFamily::FrameworkService => 2,
                _ => unreachable!("core work families are exhaustively classified here"),
            };
            let Some(family_future) = phase_starts.checked_mul(future_per_start) else {
                return false;
            };
            let Some(future_credits) = self.future_credits.checked_add(family_future) else {
                return false;
            };
            self.future_credits = future_credits;
        }
        true
    }

    fn total_starts(self) -> Option<usize> {
        self.starts.into_iter().try_fold(0usize, usize::checked_add)
    }
}

const fn is_focus_command(command: SemanticCommand) -> bool {
    matches!(
        command,
        SemanticCommand::RequestFocus
            | SemanticCommand::FocusNext
            | SemanticCommand::FocusPrevious
            | SemanticCommand::FocusLeft
            | SemanticCommand::FocusRight
            | SemanticCommand::FocusUp
            | SemanticCommand::FocusDown
            | SemanticCommand::RestoreFocus
            | SemanticCommand::FocusGroupNext
            | SemanticCommand::FocusGroupPrevious
            | SemanticCommand::FocusGroupFirst
            | SemanticCommand::FocusGroupLast
    )
}

const fn is_focus_navigation(command: SemanticCommand) -> bool {
    matches!(
        command,
        SemanticCommand::FocusNext
            | SemanticCommand::FocusPrevious
            | SemanticCommand::FocusLeft
            | SemanticCommand::FocusRight
            | SemanticCommand::FocusUp
            | SemanticCommand::FocusDown
            | SemanticCommand::RestoreFocus
            | SemanticCommand::FocusGroupNext
            | SemanticCommand::FocusGroupPrevious
            | SemanticCommand::FocusGroupFirst
            | SemanticCommand::FocusGroupLast
    )
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn release_unused_reservation(&mut self, reservation: &QueueReservation) {
        if self.queue.release_reservation(reservation).is_err() {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
        }
    }

    #[cfg(test)]
    pub(crate) fn submit_action(
        &mut self,
        action: Action,
        category: TraceActionCategory,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
    ) -> SubmitActionResult<Action> {
        match self.status {
            RuntimeStatus::Closed => {
                self.record_optional(
                    TraceRecordKind::ActionSubmissionRejectedClosed,
                    None,
                    None,
                    None,
                );
                return Err(SubmitActionError::Closed(action));
            }
            RuntimeStatus::Terminal(reason) => {
                self.record_optional(
                    TraceRecordKind::ActionSubmissionRejectedTerminal,
                    None,
                    None,
                    None,
                );
                return Err(SubmitActionError::Terminal { action, reason });
            }
            RuntimeStatus::Running => {}
        }
        if self.queue.is_full() {
            self.record_optional(
                TraceRecordKind::ActionSubmissionRejectedFull,
                None,
                None,
                target,
            );
            return Err(SubmitActionError::Full(action));
        }
        let instant = self.now();
        let sequence = match self.commit_preflighted_action(
            action,
            causal_parent,
            target,
            category,
            instant,
        ) {
            Ok(sequence) => sequence,
            Err(ActionCommitError::WorkCapacity(action, family)) => {
                return Err(SubmitActionError::WorkCapacity { action, family });
            }
            Err(ActionCommitError::QueueFull(action)) => {
                self.record_optional(
                    TraceRecordKind::ActionSubmissionRejectedFull,
                    None,
                    None,
                    None,
                );
                return Err(SubmitActionError::Full(action));
            }
            Err(ActionCommitError::WorkSequenceExhausted(action)) => {
                let reason = RuntimeTerminalReason::WorkSequenceExhausted;
                self.enter_terminal(reason, 0);
                return Err(SubmitActionError::Terminal { action, reason });
            }
            Err(ActionCommitError::TraceSequenceExhausted(action)) => {
                let reason = RuntimeTerminalReason::TraceSequenceExhausted;
                self.enter_terminal(reason, 0);
                return Err(SubmitActionError::Terminal { action, reason });
            }
            Err(ActionCommitError::Integrity(action)) => {
                let reason = RuntimeTerminalReason::Poisoned;
                self.enter_terminal(reason, 0);
                return Err(SubmitActionError::Terminal { action, reason });
            }
        };
        self.external_queue_commit_accepted();
        Ok(sequence)
    }

    pub(crate) fn submit_profiled_action(
        &mut self,
        action: Action,
        budget: ProducerBudget,
        category: TraceActionCategory,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
    ) -> SubmitActionResult<Action> {
        let (action, reservation) =
            self.prepare_profiled_action(action, budget, target.as_ref())?;
        let instant = self.now();
        let sequence = match self.commit_preflighted_action_from_reservation(
            action,
            causal_parent,
            target,
            category,
            instant,
            budget,
            &reservation,
        ) {
            Ok(sequence) => sequence,
            Err(ActionCommitError::WorkCapacity(action, family)) => {
                self.release_unused_reservation(&reservation);
                return Err(SubmitActionError::WorkCapacity { action, family });
            }
            Err(ActionCommitError::QueueFull(action)) => {
                self.release_unused_reservation(&reservation);
                self.record_optional(
                    TraceRecordKind::ActionSubmissionRejectedFull,
                    None,
                    None,
                    None,
                );
                return Err(SubmitActionError::Full(action));
            }
            Err(ActionCommitError::WorkSequenceExhausted(action)) => {
                self.release_unused_reservation(&reservation);
                self.enter_terminal(RuntimeTerminalReason::WorkSequenceExhausted, 0);
                return Err(SubmitActionError::Terminal {
                    action,
                    reason: RuntimeTerminalReason::WorkSequenceExhausted,
                });
            }
            Err(ActionCommitError::TraceSequenceExhausted(action)) => {
                self.release_unused_reservation(&reservation);
                self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
                return Err(SubmitActionError::Terminal {
                    action,
                    reason: RuntimeTerminalReason::TraceSequenceExhausted,
                });
            }
            Err(ActionCommitError::Integrity(action)) => {
                self.release_unused_reservation(&reservation);
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                return Err(SubmitActionError::Terminal {
                    action,
                    reason: RuntimeTerminalReason::Poisoned,
                });
            }
        };
        if matches!(self.status, RuntimeStatus::Running) {
            self.external_queue_commit_accepted();
        }
        Ok(sequence)
    }

    fn prepare_profiled_action(
        &mut self,
        action: Action,
        budget: ProducerBudget,
        target: Option<&TraceTarget>,
    ) -> Result<(Action, QueueReservation), SubmitActionError<Action>> {
        match self.status {
            RuntimeStatus::Closed => {
                self.record_optional(
                    TraceRecordKind::ActionSubmissionRejectedClosed,
                    None,
                    None,
                    None,
                );
                return Err(SubmitActionError::Closed(action));
            }
            RuntimeStatus::Terminal(reason) => {
                self.record_optional(
                    TraceRecordKind::ActionSubmissionRejectedTerminal,
                    None,
                    None,
                    None,
                );
                return Err(SubmitActionError::Terminal { action, reason });
            }
            RuntimeStatus::Running => {}
        }
        if !self.application_output_budget_fits(budget)
            || !self.application_identity_budget_fits(budget)
        {
            return Err(SubmitActionError::InvalidProducerProfile(action));
        }
        if let Err(reason) = self.preflight_application_turn_identity(budget) {
            self.enter_terminal(reason, 0);
            return Err(SubmitActionError::Terminal { action, reason });
        }
        let Some(trace_plan) = self.application_turn_trace_plan(budget, false) else {
            return Err(SubmitActionError::InvalidProducerProfile(action));
        };
        if !self.trace.can_admit(trace_plan) {
            let reason = RuntimeTerminalReason::TraceSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(SubmitActionError::Terminal { action, reason });
        }
        let reservation = match self.reserve_producer(budget) {
            Ok(reservation) => reservation,
            Err(QueueReservationError::WorkCapacity(family)) => {
                return Err(SubmitActionError::WorkCapacity { action, family });
            }
            Err(QueueReservationError::Capacity | QueueReservationError::Closed) => {
                self.record_optional(
                    TraceRecordKind::ActionSubmissionRejectedFull,
                    None,
                    None,
                    target.cloned(),
                );
                return Err(SubmitActionError::Full(action));
            }
            Err(QueueReservationError::SequenceExhausted) => {
                self.enter_terminal(RuntimeTerminalReason::WorkSequenceExhausted, 0);
                return Err(SubmitActionError::Terminal {
                    action,
                    reason: RuntimeTerminalReason::WorkSequenceExhausted,
                });
            }
            Err(
                QueueReservationError::Foreign
                | QueueReservationError::Stale
                | QueueReservationError::Exhausted,
            ) => {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                return Err(SubmitActionError::Terminal {
                    action,
                    reason: RuntimeTerminalReason::Poisoned,
                });
            }
        };
        Ok((action, reservation))
    }

    pub(in crate::runtime) fn commit_preflighted_action(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        category: TraceActionCategory,
        instant: MonotonicInstant,
    ) -> Result<WorkSequence, ActionCommitError<Action>> {
        let budget = self.action_producer_budget.map_or_else(
            || ProducerBudget::new(1, 0),
            |profile| profile(self.state(), &action),
        );
        if !self.application_output_budget_fits(budget) {
            return Err(ActionCommitError::Integrity(action));
        }
        let reservation = match self.reserve_producer(budget) {
            Ok(reservation) => reservation,
            Err(QueueReservationError::WorkCapacity(family)) => {
                return Err(ActionCommitError::WorkCapacity(action, family));
            }
            Err(QueueReservationError::Capacity | QueueReservationError::Closed) => {
                return Err(ActionCommitError::QueueFull(action));
            }
            Err(QueueReservationError::SequenceExhausted) => {
                return Err(ActionCommitError::WorkSequenceExhausted(action));
            }
            Err(
                QueueReservationError::Foreign
                | QueueReservationError::Stale
                | QueueReservationError::Exhausted,
            ) => {
                return Err(ActionCommitError::Integrity(action));
            }
        };
        let result = self.commit_preflighted_action_from_reservation(
            action,
            causal_parent,
            target,
            category,
            instant,
            budget,
            &reservation,
        );
        if result.is_err() {
            self.release_unused_reservation(&reservation);
        }
        result
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "Keep the action, exact causal trace identity, producer profile, and owned reservation explicit at the atomic commit boundary"
    )]
    fn commit_preflighted_action_from_reservation(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        category: TraceActionCategory,
        instant: MonotonicInstant,
        producer_budget: ProducerBudget,
        reservation: &QueueReservation,
    ) -> Result<WorkSequence, ActionCommitError<Action>> {
        if !self.application_output_budget_fits(producer_budget) {
            return Err(ActionCommitError::Integrity(action));
        }
        if !self
            .trace
            .can_admit(MandatoryTracePlan::action_acceptance())
        {
            return Err(ActionCommitError::TraceSequenceExhausted(action));
        }
        let Some(sequence) = self.queue.next_sequence() else {
            return Err(ActionCommitError::Integrity(action));
        };
        let trace_enabled = self.trace.is_enabled();
        let action_label = if trace_enabled {
            self.trace_action_labeler
                .and_then(|labeler| labeler(&action))
        } else {
            None
        };
        let accepted = self.trace.record_draft(
            TraceRecordDraft::action_fact(
                TraceRecordKind::ActionSubmissionAccepted,
                instant,
                TraceContext::action_record(TraceActionIdentity::of_labeled::<Action>(
                    category,
                    action_label,
                )),
            )
            .with_work_sequence(Some(sequence))
            .with_causal_parent(causal_parent)
            .with_target(target.clone()),
        );
        if trace_enabled && accepted.is_none() {
            return Err(ActionCommitError::TraceSequenceExhausted(action));
        }
        self.queue
            .push_preflighted_with_reservation(
                action,
                accepted,
                target,
                ApplicationActionOrigin::Ordinary,
                producer_budget,
                reservation,
            )
            .map_err(ActionCommitError::Integrity)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "Preserve the exact application action commit context at the rooted runtime boundary"
    )]
    pub(in crate::runtime) fn commit_preflighted_action_from_root(
        &mut self,
        action: Action,
        producer_budget: ProducerBudget,
        root_quota: RootQuotaId,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        category: TraceActionCategory,
        instant: MonotonicInstant,
    ) -> Result<WorkSequence, ActionCommitError<Action>> {
        self.commit_preflighted_action_from_root_with_host_credit(
            action,
            producer_budget,
            root_quota,
            None,
            None,
            None,
            None,
            causal_parent,
            target,
            category,
            instant,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "Preserve the exact host response generation and causal commit context at the rooted boundary"
    )]
    pub(in crate::runtime) fn commit_preflighted_host_response_action_from_root(
        &mut self,
        action: Action,
        producer_budget: ProducerBudget,
        root_quota: RootQuotaId,
        generation: crate::work::WorkGeneration,
        causal_parent: Option<TraceSequence>,
        category: TraceActionCategory,
        instant: MonotonicInstant,
    ) -> Result<WorkSequence, ActionCommitError<Action>> {
        self.commit_preflighted_action_from_root_with_host_credit(
            action,
            producer_budget,
            root_quota,
            Some(generation),
            None,
            None,
            None,
            causal_parent,
            None,
            category,
            instant,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "Preserve the exact local task generation and causal commit context at the rooted boundary"
    )]
    pub(in crate::runtime) fn commit_preflighted_local_task_action_from_root(
        &mut self,
        action: Action,
        producer_budget: ProducerBudget,
        root_quota: RootQuotaId,
        generation: crate::work::WorkGeneration,
        causal_parent: Option<TraceSequence>,
        category: TraceActionCategory,
        instant: MonotonicInstant,
    ) -> Result<WorkSequence, ActionCommitError<Action>> {
        self.commit_preflighted_action_from_root_with_host_credit(
            action,
            producer_budget,
            root_quota,
            None,
            Some(generation),
            None,
            None,
            causal_parent,
            None,
            category,
            instant,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "Preserve the exact send task generation and causal commit context at the rooted boundary"
    )]
    pub(in crate::runtime) fn commit_preflighted_send_task_action_from_root(
        &mut self,
        action: Action,
        producer_budget: ProducerBudget,
        root_quota: RootQuotaId,
        generation: crate::work::WorkGeneration,
        causal_parent: Option<TraceSequence>,
        category: TraceActionCategory,
        instant: MonotonicInstant,
    ) -> Result<WorkSequence, ActionCommitError<Action>> {
        self.commit_preflighted_action_from_root_with_host_credit(
            action,
            producer_budget,
            root_quota,
            None,
            None,
            Some(generation),
            None,
            causal_parent,
            None,
            category,
            instant,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "Preserve the exact timer generation and causal commit context at the rooted boundary"
    )]
    pub(in crate::runtime) fn commit_preflighted_timer_action_from_root(
        &mut self,
        action: Action,
        producer_budget: ProducerBudget,
        root_quota: RootQuotaId,
        generation: crate::work::WorkGeneration,
        causal_parent: Option<TraceSequence>,
        category: TraceActionCategory,
        instant: MonotonicInstant,
    ) -> Result<WorkSequence, ActionCommitError<Action>> {
        self.commit_preflighted_action_from_root_with_host_credit(
            action,
            producer_budget,
            root_quota,
            None,
            None,
            None,
            Some(generation),
            causal_parent,
            None,
            category,
            instant,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn commit_preflighted_action_from_root_with_host_credit(
        &mut self,
        action: Action,
        producer_budget: ProducerBudget,
        root_quota: RootQuotaId,
        host_response_generation: Option<crate::work::WorkGeneration>,
        local_task_generation: Option<crate::work::WorkGeneration>,
        send_task_generation: Option<crate::work::WorkGeneration>,
        timer_generation: Option<crate::work::WorkGeneration>,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        category: TraceActionCategory,
        instant: MonotonicInstant,
    ) -> Result<WorkSequence, ActionCommitError<Action>> {
        if !self.application_output_budget_fits(producer_budget)
            || !self
                .queue
                .root_covers_action_budget(root_quota, producer_budget)
        {
            return Err(ActionCommitError::Integrity(action));
        }
        if !self
            .trace
            .can_admit(MandatoryTracePlan::action_acceptance())
        {
            return Err(ActionCommitError::TraceSequenceExhausted(action));
        }
        let Some(sequence) = self.queue.next_sequence() else {
            return Err(ActionCommitError::Integrity(action));
        };
        let trace_enabled = self.trace.is_enabled();
        let action_label = if trace_enabled {
            self.trace_action_labeler
                .and_then(|labeler| labeler(&action))
        } else {
            None
        };
        let accepted = self.trace.record_draft(
            TraceRecordDraft::action_fact(
                TraceRecordKind::ActionSubmissionAccepted,
                instant,
                TraceContext::action_record(TraceActionIdentity::of_labeled::<Action>(
                    category,
                    action_label,
                )),
            )
            .with_work_sequence(Some(sequence))
            .with_causal_parent(causal_parent)
            .with_target(target.clone()),
        );
        if trace_enabled && accepted.is_none() {
            return Err(ActionCommitError::TraceSequenceExhausted(action));
        }
        let mut action = Some(action);
        let envelope = |sequence, root_quota| {
            WorkEnvelope::ApplicationAction(crate::queue::ApplicationActionEnvelope {
                sequence,
                action: action.take().unwrap_or_else(|| unreachable!()),
                producer_budget,
                root_quota,
                causal_parent: accepted,
                target,
                origin: ApplicationActionOrigin::Ordinary,
            })
        };
        let pushed = if let Some(generation) = host_response_generation {
            self.queue
                .push_host_response_action_rooted(root_quota, generation, envelope)
        } else if let Some(generation) = local_task_generation {
            self.queue
                .push_local_task_outcome_action_rooted(root_quota, generation, envelope)
        } else if let Some(generation) = send_task_generation {
            self.queue
                .push_send_task_outcome_action_rooted(root_quota, generation, envelope)
        } else if let Some(generation) = timer_generation {
            self.queue
                .push_timer_callback_action_rooted(root_quota, generation, envelope)
        } else {
            self.queue.push_rooted(root_quota, envelope)
        };
        pushed.map_err(|_| {
            ActionCommitError::Integrity(action.take().unwrap_or_else(|| unreachable!()))
        })
    }

    pub(in crate::runtime) fn commit_preflighted_edit_action(
        &mut self,
        action: Action,
        origin: crate::editing::EditActionOrigin,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        instant: MonotonicInstant,
    ) -> Result<WorkSequence, ActionCommitError<Action>> {
        let budget = self.action_producer_budget.map_or_else(
            || ProducerBudget::new(1, 0),
            |profile| profile(self.state(), &action),
        );
        if !self.application_output_budget_fits(budget) {
            return Err(ActionCommitError::Integrity(action));
        }
        let reservation = match self.reserve_producer(budget) {
            Ok(reservation) => reservation,
            Err(QueueReservationError::WorkCapacity(family)) => {
                return Err(ActionCommitError::WorkCapacity(action, family));
            }
            Err(QueueReservationError::Capacity | QueueReservationError::Closed) => {
                return Err(ActionCommitError::QueueFull(action));
            }
            Err(QueueReservationError::SequenceExhausted) => {
                return Err(ActionCommitError::WorkSequenceExhausted(action));
            }
            Err(
                QueueReservationError::Foreign
                | QueueReservationError::Stale
                | QueueReservationError::Exhausted,
            ) => {
                return Err(ActionCommitError::Integrity(action));
            }
        };
        if !self
            .trace
            .can_admit(MandatoryTracePlan::action_acceptance())
        {
            self.release_unused_reservation(&reservation);
            return Err(ActionCommitError::TraceSequenceExhausted(action));
        }
        let Some(sequence) = self.queue.next_sequence() else {
            self.release_unused_reservation(&reservation);
            return Err(ActionCommitError::Integrity(action));
        };
        let trace_enabled = self.trace.is_enabled();
        let action_label = if trace_enabled {
            self.trace_action_labeler
                .and_then(|labeler| labeler(&action))
        } else {
            None
        };
        let accepted = self.trace.record_draft(
            TraceRecordDraft::action_fact(
                TraceRecordKind::ActionSubmissionAccepted,
                instant,
                TraceContext::action_record(TraceActionIdentity::editing::<Action>(
                    action_label,
                    &origin,
                )),
            )
            .with_work_sequence(Some(sequence))
            .with_causal_parent(causal_parent)
            .with_target(target.clone()),
        );
        if trace_enabled && accepted.is_none() {
            self.release_unused_reservation(&reservation);
            return Err(ActionCommitError::TraceSequenceExhausted(action));
        }
        let result = self
            .queue
            .push_preflighted_with_reservation(
                action,
                accepted,
                target,
                ApplicationActionOrigin::Edit(origin),
                budget,
                &reservation,
            )
            .map_err(ActionCommitError::Integrity);
        if result.is_err() {
            self.release_unused_reservation(&reservation);
        }
        result
    }

    pub(in crate::runtime) fn commit_preflighted_edit_action_from_root(
        &mut self,
        action: Action,
        origin: crate::editing::EditActionOrigin,
        root_quota: RootQuotaId,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        instant: MonotonicInstant,
    ) -> Result<WorkSequence, ActionCommitError<Action>> {
        let budget = self.action_producer_budget.map_or_else(
            || ProducerBudget::new(1, 0),
            |profile| profile(self.state(), &action),
        );
        if !self.application_output_budget_fits(budget)
            || !self.queue.root_covers_action_budget(root_quota, budget)
        {
            return Err(ActionCommitError::Integrity(action));
        }
        if !self
            .trace
            .can_admit(MandatoryTracePlan::action_acceptance())
        {
            return Err(ActionCommitError::TraceSequenceExhausted(action));
        }
        let Some(sequence) = self.queue.next_sequence() else {
            return Err(ActionCommitError::Integrity(action));
        };
        let trace_enabled = self.trace.is_enabled();
        let action_label = if trace_enabled {
            self.trace_action_labeler
                .and_then(|labeler| labeler(&action))
        } else {
            None
        };
        let accepted = self.trace.record_draft(
            TraceRecordDraft::action_fact(
                TraceRecordKind::ActionSubmissionAccepted,
                instant,
                TraceContext::action_record(TraceActionIdentity::editing::<Action>(
                    action_label,
                    &origin,
                )),
            )
            .with_work_sequence(Some(sequence))
            .with_causal_parent(causal_parent)
            .with_target(target.clone()),
        );
        if trace_enabled && accepted.is_none() {
            return Err(ActionCommitError::TraceSequenceExhausted(action));
        }
        let mut action = Some(action);
        self.queue
            .push_rooted(root_quota, |sequence, root_quota| {
                WorkEnvelope::ApplicationAction(crate::queue::ApplicationActionEnvelope {
                    sequence,
                    action: action.take().unwrap_or_else(|| unreachable!()),
                    producer_budget: budget,
                    root_quota,
                    causal_parent: accepted,
                    target,
                    origin: ApplicationActionOrigin::Edit(origin),
                })
            })
            .map_err(|_| {
                ActionCommitError::Integrity(action.take().unwrap_or_else(|| unreachable!()))
            })
    }

    pub(crate) fn external_queue_commit_accepted(&self) {
        let _ = self.wake.handle().request();
    }

    pub(crate) fn submit_command(
        &mut self,
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
    ) -> Result<CommandSubmission, SubmitCommandError> {
        match self.submit_command_inner(
            &target,
            command,
            origin,
            CommandTrace::Direct { parent: None },
            None,
        ) {
            Ok(submission) => Ok(submission),
            Err(kind) => {
                let error = Self::reject_command_submission(kind, target, command, origin);
                self.terminalize_command_failure(kind);
                Err(error)
            }
        }
    }

    pub(crate) fn submit_command_with_parent(
        &mut self,
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
        parent: Option<TraceSequence>,
        root_quota: Option<RootQuotaId>,
    ) -> Result<CommandSubmission, SubmitCommandError> {
        match self.submit_command_inner(
            &target,
            command,
            origin,
            CommandTrace::Direct { parent },
            root_quota,
        ) {
            Ok(submission) => Ok(submission),
            Err(kind) => {
                let error = Self::reject_command_submission(kind, target, command, origin);
                self.terminalize_command_failure(kind);
                Err(error)
            }
        }
    }

    pub(super) const fn command_status_preflight(&self) -> Result<(), SubmitCommandErrorKind> {
        match self.status {
            RuntimeStatus::Running => Ok(()),
            RuntimeStatus::Closed => Err(SubmitCommandErrorKind::Closed),
            RuntimeStatus::Terminal(reason) => Err(SubmitCommandErrorKind::Terminal(reason)),
        }
    }

    pub(super) fn submit_surface_bound_command(
        &mut self,
        target: &MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
        trace: SurfaceCommandTrace,
    ) -> Result<CommandSubmission, SubmitCommandErrorKind> {
        self.submit_command_inner(target, command, origin, CommandTrace::Surface(trace), None)
    }

    pub(in crate::runtime) fn submit_semantic_action_command(
        &mut self,
        target: &MountedNodeId,
        command: SemanticCommand,
        semantic_target: SemanticActionTarget,
    ) -> Result<CommandSubmission, SubmitCommandErrorKind> {
        self.command_preflight(target)?;
        self.commit_preflighted_command(
            PreflightedCommand {
                target,
                command,
                origin: CommandOrigin::accessibility(),
                semantic_target: Some(semantic_target),
                focus_request_override: None,
                presentation_cancel_origin: None,
                instant: self.now(),
                trace: CommandTrace::Direct { parent: None },
            },
            None,
        )
    }

    fn submit_command_inner(
        &mut self,
        target: &MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
        trace: CommandTrace,
        root_quota: Option<RootQuotaId>,
    ) -> Result<CommandSubmission, SubmitCommandErrorKind> {
        self.command_preflight(target)?;
        self.commit_preflighted_command(
            PreflightedCommand {
                target,
                command,
                origin,
                semantic_target: None,
                focus_request_override: None,
                presentation_cancel_origin: None,
                instant: self.now(),
                trace,
            },
            root_quota,
        )
    }

    fn command_preflight(&self, target: &MountedNodeId) -> Result<(), SubmitCommandErrorKind> {
        self.command_status_preflight()?;
        match self.tree.target_status(target) {
            TargetStatus::Live => {}
            TargetStatus::Foreign => return Err(SubmitCommandErrorKind::ForeignTarget),
            TargetStatus::Stale => return Err(SubmitCommandErrorKind::StaleTarget),
            TargetStatus::Missing => return Err(SubmitCommandErrorKind::MissingTarget),
        }
        Ok(())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "Keep the target, command provenance, time, cancellation cause, and inherited root explicit for atomic routing"
    )]
    pub(in crate::runtime) fn commit_preflighted_routed_command(
        &mut self,
        target: &MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
        causal_parent: Option<TraceSequence>,
        instant: MonotonicInstant,
        presentation_cancel_origin: Option<super::super::presentation::PresentationCancelOrigin>,
        root_quota: Option<RootQuotaId>,
    ) -> Result<CommandSubmission, SubmitCommandErrorKind> {
        self.command_preflight(target)?;
        self.commit_preflighted_command(
            PreflightedCommand {
                target,
                command,
                origin,
                semantic_target: None,
                focus_request_override: None,
                presentation_cancel_origin,
                instant,
                trace: CommandTrace::Direct {
                    parent: causal_parent,
                },
            },
            root_quota,
        )
    }

    pub(in crate::runtime) fn submit_presentation_focus_request(
        &mut self,
        routing_target: &MountedNodeId,
        focus_target: Option<MountedNodeId>,
        reason: FocusReason,
        causal_parent: Option<TraceSequence>,
        entry: Option<crate::queue::PresentationFocusEntry>,
        root_quota: Option<RootQuotaId>,
    ) -> Result<CommandSubmission, SubmitCommandErrorKind> {
        let result = self.command_preflight(routing_target).and_then(|()| {
            self.commit_preflighted_command(
                PreflightedCommand {
                    target: routing_target,
                    command: SemanticCommand::RequestFocus,
                    origin: CommandOrigin::__runtime_semantic_default(EventSource::Programmatic),
                    semantic_target: None,
                    focus_request_override: Some(FocusRequestOverride {
                        target: focus_target,
                        reason,
                        presentation_entry: entry,
                    }),
                    presentation_cancel_origin: None,
                    instant: self.now(),
                    trace: CommandTrace::Direct {
                        parent: causal_parent,
                    },
                },
                root_quota,
            )
        });
        if let Err(kind) = result {
            self.handle_mandatory_presentation_command_failure(kind);
        }
        result
    }

    pub(in crate::runtime) fn submit_presentation_dismiss_request(
        &mut self,
        target: &MountedNodeId,
        reason: runenui_core::PresentationDismissReason,
        causal_parent: Option<TraceSequence>,
        root_quota: Option<RootQuotaId>,
    ) -> Result<CommandSubmission, SubmitCommandErrorKind> {
        let result = self.command_preflight(target).and_then(|()| {
            self.commit_preflighted_command(
                PreflightedCommand {
                    target,
                    command: SemanticCommand::PresentationDismiss(reason),
                    origin: CommandOrigin::__runtime_semantic_default(EventSource::Programmatic),
                    semantic_target: None,
                    focus_request_override: None,
                    presentation_cancel_origin: None,
                    instant: self.now(),
                    trace: CommandTrace::Direct {
                        parent: causal_parent,
                    },
                },
                root_quota,
            )
        });
        if let Err(kind) = result {
            self.handle_mandatory_presentation_command_failure(kind);
        }
        result
    }

    fn handle_mandatory_presentation_command_failure(&mut self, kind: SubmitCommandErrorKind) {
        match kind {
            SubmitCommandErrorKind::WorkSequenceExhausted
            | SubmitCommandErrorKind::TraceSequenceExhausted => {
                self.terminalize_command_failure(kind);
            }
            SubmitCommandErrorKind::Closed | SubmitCommandErrorKind::Terminal(_) => {}
            SubmitCommandErrorKind::Full
            | SubmitCommandErrorKind::WorkCapacity(_)
            | SubmitCommandErrorKind::InvalidProducerProfile
            | SubmitCommandErrorKind::ForeignTarget
            | SubmitCommandErrorKind::StaleTarget
            | SubmitCommandErrorKind::MissingTarget => {
                if matches!(self.status, RuntimeStatus::Running) {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                }
            }
        }
    }

    pub(in crate::runtime) fn commit_preflighted_routed_application_command(
        &mut self,
        target: &MountedNodeId,
        command: ApplicationCommandId,
        origin: CommandOrigin,
        causal_parent: Option<TraceSequence>,
        instant: MonotonicInstant,
        root_quota: Option<RootQuotaId>,
    ) -> Result<CommandSubmission, SubmitCommandErrorKind> {
        self.command_preflight(target)?;
        let mut reservation = if let Some(root) = root_quota {
            self.queue
                .preflight_root_commit(root, 1)
                .map_err(|error| match error {
                    QueueReservationError::SequenceExhausted => {
                        SubmitCommandErrorKind::WorkSequenceExhausted
                    }
                    QueueReservationError::Closed => SubmitCommandErrorKind::Closed,
                    _ => SubmitCommandErrorKind::InvalidProducerProfile,
                })?;
            None
        } else {
            Some(self.queue.reserve(1).map_err(|error| match error {
                QueueReservationError::Capacity => SubmitCommandErrorKind::Full,
                QueueReservationError::SequenceExhausted => {
                    SubmitCommandErrorKind::WorkSequenceExhausted
                }
                QueueReservationError::Closed => SubmitCommandErrorKind::Closed,
                _ => SubmitCommandErrorKind::InvalidProducerProfile,
            })?)
        };
        let trace_reservation = self
            .trace
            .reserve_command_outcome()
            .ok_or(SubmitCommandErrorKind::TraceSequenceExhausted)?;
        let Some(sequence) = self.queue.next_sequence() else {
            self.trace.release_reservation(trace_reservation);
            if let Some(reservation) = reservation.as_mut() {
                self.release_unused_reservation(reservation);
            }
            return Err(SubmitCommandErrorKind::WorkSequenceExhausted);
        };
        let accepted = self.trace.record_event(
            TraceRecordKind::ApplicationCommandSubmissionAccepted {
                command: command.clone(),
            },
            sequence,
            causal_parent,
            Some(self.tree.trace_target(target)),
            instant,
            target,
            None,
            origin,
        );
        let causal_parent = match self.require_reserved_trace_record(accepted, trace_reservation) {
            Ok(parent) => parent,
            Err(kind) => {
                if let Some(reservation) = reservation.as_mut() {
                    self.release_unused_reservation(reservation);
                }
                return Err(kind);
            }
        };
        let pushed = if let Some(root) = root_quota {
            self.queue.push_application_command_rooted(
                root,
                ApplicationCommandQueuePayload {
                    target: target.clone(),
                    command,
                    origin,
                    instant,
                    causal_parent,
                    trace_reservation,
                },
            )
        } else {
            self.queue.push_application_command_preflighted(
                reservation.as_ref().unwrap_or_else(|| unreachable!()),
                ApplicationCommandQueuePayload {
                    target: target.clone(),
                    command,
                    origin,
                    instant,
                    causal_parent,
                    trace_reservation,
                },
            )
        };
        if pushed.is_err() {
            self.trace.release_reservation(trace_reservation);
            if let Some(reservation) = reservation.as_mut() {
                self.release_unused_reservation(reservation);
            }
            return Err(SubmitCommandErrorKind::InvalidProducerProfile);
        }
        self.external_queue_commit_accepted();
        Ok(CommandSubmission::new(sequence))
    }

    fn commit_preflighted_command(
        &mut self,
        command: PreflightedCommand<'_>,
        inherited_root: Option<RootQuotaId>,
    ) -> Result<CommandSubmission, SubmitCommandErrorKind> {
        let mut reservation = if let Some(root) = inherited_root {
            self.queue
                .preflight_root_commit(root, 1)
                .map_err(|error| match error {
                    crate::queue::QueueReservationError::SequenceExhausted => {
                        SubmitCommandErrorKind::WorkSequenceExhausted
                    }
                    crate::queue::QueueReservationError::Closed => SubmitCommandErrorKind::Closed,
                    _ => SubmitCommandErrorKind::InvalidProducerProfile,
                })?;
            None
        } else {
            let Some(profile) = self
                .interaction_producer_budget
                .map(|budget| budget(self.state()))
            else {
                return Err(SubmitCommandErrorKind::InvalidProducerProfile);
            };
            self.queue
                .preflight_reservation(profile.queue_credits())
                .map_err(Self::command_reservation_error)?;
            if !self.interaction_profile_covers_command(command.target, command.command, profile) {
                return Err(SubmitCommandErrorKind::InvalidProducerProfile);
            }
            Some(
                self.reserve_producer(profile)
                    .map_err(Self::command_reservation_error)?,
            )
        };
        let root_quota =
            inherited_root.or_else(|| reservation.as_ref().map(QueueReservation::quota));
        let Some(root_quota) = root_quota else {
            return Err(SubmitCommandErrorKind::InvalidProducerProfile);
        };
        let trace_reservation = match self.reserve_command_processing_outcome(&command) {
            Ok(reservation) => reservation,
            Err(kind) => {
                if let Some(reservation) = reservation.as_mut() {
                    self.release_unused_reservation(reservation);
                }
                return Err(kind);
            }
        };
        let Some(sequence) = self.queue.next_sequence() else {
            self.trace.release_reservation(trace_reservation);
            if let Some(reservation) = reservation.as_mut() {
                self.release_unused_reservation(reservation);
            }
            return Err(SubmitCommandErrorKind::WorkSequenceExhausted);
        };
        let causal_parent =
            match self.record_command_acceptance(&command, sequence, trace_reservation) {
                Ok(parent) => parent,
                Err(kind) => {
                    self.trace.release_reservation(trace_reservation);
                    if let Some(reservation) = reservation.as_mut() {
                        self.release_unused_reservation(reservation);
                    }
                    return Err(kind);
                }
            };
        let PreflightedCommand {
            target,
            command,
            origin,
            semantic_target,
            focus_request_override,
            presentation_cancel_origin,
            instant,
            trace: _,
        } = command;
        let queued_target = semantic_target.map_or_else(
            || SemanticCommandQueueTarget::mounted(target.clone()),
            |semantic_target| SemanticCommandQueueTarget::semantic(target.clone(), semantic_target),
        );
        let payload = SemanticCommandQueuePayload {
            queued_target,
            command,
            origin,
            focus_request_override,
            presentation_cancel_origin,
            instant,
            causal_parent,
            trace_reservation,
            root_quota,
        };
        let pushed = if let Some(reservation) = reservation.as_ref() {
            self.queue.push_command_preflighted(payload, reservation)
        } else {
            self.queue.push_command_rooted(payload)
        };
        if pushed.is_err() {
            self.trace.release_reservation(trace_reservation);
            if let Some(reservation) = reservation.as_mut() {
                self.release_unused_reservation(reservation);
            }
            return Err(SubmitCommandErrorKind::InvalidProducerProfile);
        }
        self.external_queue_commit_accepted();
        Ok(CommandSubmission::new(sequence))
    }

    fn interaction_profile_covers_command(
        &self,
        target: &MountedNodeId,
        command: SemanticCommand,
        profile: ProducerBudget,
    ) -> bool {
        if !self.application_output_budget_fits(profile) {
            return false;
        }
        // Current mounted callback budgets cannot certify the successor of an
        // earlier mutable producer. In that case use the app's declared bound
        // across reachable predecessor states for this new routed command.
        // This gate does not constrain independently funded cleanup passes of
        // an already admitted application turn.
        if self.queue.has_unfinished_producers()
            && profile.outputs() > self.limits.transaction_outputs()
        {
            return false;
        }
        let Ok(route) = self.tree.event_route(target) else {
            return false;
        };
        let Some(route_budget) = self.interaction_route_budget(&route, command) else {
            return false;
        };
        self.interaction_profile_covers_route(&route, command, profile, route_budget)
    }

    fn interaction_route_budget(
        &self,
        route: &[MountedNodeId],
        command: SemanticCommand,
    ) -> Option<InteractionRouteBudget> {
        let mut required = InteractionRouteBudget::default();
        if let Some((target, ancestors)) = route.split_last() {
            for ancestor in ancestors {
                if !required.include_widget_budget(self.tree.event_budget(ancestor).ok()?, 2) {
                    return None;
                }
            }
            if !required.include_widget_budget(self.tree.event_budget(target).ok()?, 1) {
                return None;
            }
            if command == SemanticCommand::Activate
                && !required.include_widget_budget(self.tree.activation_budget(target).ok()?, 1)
            {
                return None;
            }
        }
        if is_focus_command(command) {
            for owner in self.tree.publication_preorder_ids() {
                // Focus transfer can notify both the leaving and entering
                // routes, each through capture and bubble phases.
                if !required.include_widget_budget(self.tree.event_budget(&owner).ok()?, 4) {
                    return None;
                }
            }
        }
        Some(required)
    }

    fn interaction_profile_covers_route(
        &self,
        route: &[MountedNodeId],
        command: SemanticCommand,
        profile: ProducerBudget,
        route_budget: InteractionRouteBudget,
    ) -> bool {
        let focus_scroll_output = usize::from(
            is_focus_navigation(command)
                && route.iter().any(|owner| {
                    self.surface_publication
                        .current_scroll_metrics(owner)
                        .is_some()
                }),
        );

        // This is one command's routed transaction. Its callback/default
        // allowance must fit independently of the larger causal-root quota,
        // which can also fund separate later cleanup transactions.
        if route_budget
            .outputs
            .checked_add(focus_scroll_output)
            .is_none_or(|required| required > self.limits.transaction_outputs())
        {
            return false;
        }

        let clipboard_start = matches!(
            command,
            SemanticCommand::Copy | SemanticCommand::Cut | SemanticCommand::Paste
        );
        let cancel_output = command == SemanticCommand::CancelOrBack;
        let input_method_start = usize::from(self.input_method_service_may_be_staged());
        let extra_outputs = usize::from(clipboard_start || cancel_output)
            .checked_add(focus_scroll_output)
            .and_then(|outputs| outputs.checked_add(input_method_start))
            .unwrap_or(usize::MAX);
        let default_framework_starts =
            usize::from(clipboard_start).saturating_add(input_method_start);
        let required_start_families = [
            runenui_core::WorkFamily::LocalTask,
            runenui_core::WorkFamily::SendTask,
            runenui_core::WorkFamily::Timer,
            runenui_core::WorkFamily::HostRequest,
            runenui_core::WorkFamily::FrameworkService,
        ];
        let start_exceeds =
            required_start_families
                .into_iter()
                .enumerate()
                .any(|(index, family)| {
                    route_budget.starts[index]
                        .checked_add(
                            usize::from(family == runenui_core::WorkFamily::FrameworkService)
                                .saturating_mul(default_framework_starts),
                        )
                        .is_none_or(|required| required > profile.work_starts().for_family(family))
                });
        let output_requirement =
            route_budget
                .outputs
                .checked_add(extra_outputs)
                .and_then(|required| {
                    route_budget
                        .starts
                        .iter()
                        .try_fold(required, |sum, count| sum.checked_add(*count))
                });
        if start_exceeds || output_requirement.is_none_or(|required| required > profile.outputs()) {
            return false;
        }
        let Some(future) = route_budget
            .future_credits
            .checked_add(2 * default_framework_starts)
        else {
            return false;
        };
        let Some(total_starts) = route_budget.total_starts() else {
            return false;
        };
        let Some(queue_minimum) = route_budget
            .outputs
            .checked_add(extra_outputs)
            .and_then(|count| count.checked_add(total_starts))
            .and_then(|count| count.checked_add(future))
        else {
            return false;
        };
        // Even an output-free command needs its own canonical envelope. Its
        // consumed head credit may then be reused by bounded descendants.
        profile.queue_credits() >= queue_minimum.max(1)
    }

    const fn command_reservation_error(error: QueueReservationError) -> SubmitCommandErrorKind {
        match error {
            QueueReservationError::Capacity => SubmitCommandErrorKind::Full,
            QueueReservationError::WorkCapacity(family) => {
                SubmitCommandErrorKind::WorkCapacity(family)
            }
            QueueReservationError::SequenceExhausted => {
                SubmitCommandErrorKind::WorkSequenceExhausted
            }
            QueueReservationError::Closed => SubmitCommandErrorKind::Closed,
            QueueReservationError::Foreign
            | QueueReservationError::Stale
            | QueueReservationError::Exhausted => SubmitCommandErrorKind::InvalidProducerProfile,
        }
    }

    fn reserve_command_processing_outcome(
        &mut self,
        command: &PreflightedCommand<'_>,
    ) -> Result<TraceReservation, SubmitCommandErrorKind> {
        match (&command.trace, command.semantic_target.as_ref()) {
            (CommandTrace::Direct { .. }, Some(_)) => self.trace.reserve_semantic_action_outcome(),
            (CommandTrace::Direct { .. }, None) => self.trace.reserve_command_outcome(),
            (CommandTrace::Surface(_), _) => self.trace.reserve_surface_command_outcome(),
        }
        .ok_or(SubmitCommandErrorKind::TraceSequenceExhausted)
    }

    fn record_command_acceptance(
        &mut self,
        command: &PreflightedCommand<'_>,
        sequence: WorkSequence,
        reservation: TraceReservation,
    ) -> Result<Option<TraceSequence>, SubmitCommandErrorKind> {
        match &command.trace {
            CommandTrace::Direct { parent } => {
                self.record_direct_command_acceptance(command, sequence, *parent, reservation)
            }
            CommandTrace::Surface(surface) => {
                self.record_surface_command_acceptance(command, sequence, surface, reservation)
            }
        }
    }

    fn record_direct_command_acceptance(
        &mut self,
        command: &PreflightedCommand<'_>,
        sequence: WorkSequence,
        parent: Option<TraceSequence>,
        reservation: TraceReservation,
    ) -> Result<Option<TraceSequence>, SubmitCommandErrorKind> {
        let trace_target = self.tree.trace_target(command.target);
        let parent = if let Some(semantic_target) = command.semantic_target.as_ref() {
            let bound = self.trace.record_event(
                TraceRecordKind::SemanticActionBound {
                    target: semantic_target.clone(),
                    command: command.command,
                },
                sequence,
                parent,
                Some(trace_target.clone()),
                command.instant,
                command.target,
                None,
                command.origin,
            );
            self.require_reserved_trace_record(bound, reservation)?
        } else {
            parent
        };
        let accepted = self.trace.record_event(
            TraceRecordKind::CommandSubmissionAccepted,
            sequence,
            parent,
            Some(trace_target),
            command.instant,
            command.target,
            None,
            command.origin,
        );
        self.require_reserved_trace_record(accepted, reservation)
    }

    fn record_surface_command_acceptance(
        &mut self,
        command: &PreflightedCommand<'_>,
        sequence: WorkSequence,
        surface: &SurfaceCommandTrace,
        reservation: TraceReservation,
    ) -> Result<Option<TraceSequence>, SubmitCommandErrorKind> {
        let trace_target = self.tree.trace_target(command.target);
        let context_parent = if self.trace.is_enabled() {
            self.trace.record_draft(
                TraceRecordDraft::surface_fact(
                    TraceRecordKind::SurfaceContextAccepted {
                        ingress: surface.ingress,
                    },
                    command.instant,
                    surface.surface.clone(),
                )
                .with_work_sequence(Some(sequence)),
            )
        } else {
            None
        };
        let context_parent = self.require_reserved_trace_record(context_parent, reservation)?;
        let target_parent = self.trace.record_event(
            TraceRecordKind::SurfaceTargetBound,
            sequence,
            context_parent,
            Some(trace_target.clone()),
            command.instant,
            command.target,
            None,
            command.origin,
        );
        let target_parent = self.require_reserved_trace_record(target_parent, reservation)?;
        let accepted = self.trace.record_event(
            TraceRecordKind::CommandSubmissionAccepted,
            sequence,
            target_parent,
            Some(trace_target),
            command.instant,
            command.target,
            None,
            command.origin,
        );
        self.require_reserved_trace_record(accepted, reservation)
    }

    fn require_reserved_trace_record(
        &mut self,
        record: Option<TraceSequence>,
        reservation: TraceReservation,
    ) -> Result<Option<TraceSequence>, SubmitCommandErrorKind> {
        if self.trace.is_enabled() && record.is_none() {
            self.trace.release_reservation(reservation);
            Err(SubmitCommandErrorKind::TraceSequenceExhausted)
        } else {
            Ok(record)
        }
    }

    pub(super) fn terminalize_command_failure(&mut self, kind: SubmitCommandErrorKind) {
        let reason = match kind {
            SubmitCommandErrorKind::WorkSequenceExhausted => {
                Some(RuntimeTerminalReason::WorkSequenceExhausted)
            }
            SubmitCommandErrorKind::TraceSequenceExhausted => {
                Some(RuntimeTerminalReason::TraceSequenceExhausted)
            }
            SubmitCommandErrorKind::Full
            | SubmitCommandErrorKind::WorkCapacity(_)
            | SubmitCommandErrorKind::InvalidProducerProfile
            | SubmitCommandErrorKind::Closed
            | SubmitCommandErrorKind::Terminal(_)
            | SubmitCommandErrorKind::ForeignTarget
            | SubmitCommandErrorKind::StaleTarget
            | SubmitCommandErrorKind::MissingTarget => None,
        };
        if let Some(reason) = reason {
            self.enter_terminal(reason, 0);
        }
    }

    pub(super) const fn reject_command_submission(
        kind: SubmitCommandErrorKind,
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
    ) -> SubmitCommandError {
        SubmitCommandError::new(kind, UnacceptedCommand::new(target, command, origin))
    }

    pub(in crate::runtime) fn append_cancellation_envelopes(
        &mut self,
        invalidated: &[crate::work::WorkGeneration],
        lineage: &HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)>,
    ) {
        for generation in invalidated {
            let (identity, parent) = lineage
                .get(&generation.get())
                .cloned()
                .unwrap_or_else(|| unreachable!("cancelled work retains trace lineage"));
            match self
                .queue
                .push_local_task_cancellation(*generation, identity.clone(), parent)
            {
                Ok(Some(_)) => continue,
                Ok(None) => {}
                Err(_) => unreachable!("local task cancellation credit was preflighted"),
            }
            match self
                .queue
                .push_send_task_cancellation(*generation, identity.clone(), parent)
            {
                Ok(Some(_)) => continue,
                Ok(None) => {}
                Err(_) => unreachable!("send task cancellation credit was preflighted"),
            }
            match self
                .queue
                .push_timer_cancellation(*generation, identity.clone(), parent)
            {
                Ok(Some(_)) => continue,
                Ok(None) => {}
                Err(_) => unreachable!("timer cancellation credit was preflighted"),
            }
            match self
                .queue
                .push_host_request_cancellation(*generation, identity.clone(), parent)
            {
                Ok(Some(_)) => continue,
                Ok(None) => {}
                Err(_) => unreachable!("host cancellation credit was preflighted"),
            }
            match self.queue.push_framework_service_request_cancellation(
                *generation,
                identity.clone(),
                parent,
            ) {
                Ok(Some(_)) => continue,
                Ok(None) => {}
                Err(_) => unreachable!("framework service cancellation credit was preflighted"),
            }
            self.queue
                .push_cancellation(*generation, identity, parent)
                .unwrap_or_else(|_| unreachable!("transaction queue was preflighted"));
        }
    }

    pub(in crate::runtime) fn append_cancellation_envelopes_rooted(
        &mut self,
        invalidated: &[crate::work::WorkGeneration],
        lineage: &HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)>,
        root_quota: RootQuotaId,
    ) {
        for generation in invalidated {
            let (identity, parent) = lineage
                .get(&generation.get())
                .cloned()
                .unwrap_or_else(|| unreachable!("cancelled work retains trace lineage"));
            match self
                .queue
                .push_local_task_cancellation(*generation, identity.clone(), parent)
            {
                Ok(Some(_)) => continue,
                Ok(None) => {}
                Err(_) => unreachable!("local task cancellation credit was preflighted"),
            }
            match self
                .queue
                .push_send_task_cancellation(*generation, identity.clone(), parent)
            {
                Ok(Some(_)) => continue,
                Ok(None) => {}
                Err(_) => unreachable!("send task cancellation credit was preflighted"),
            }
            match self
                .queue
                .push_timer_cancellation(*generation, identity.clone(), parent)
            {
                Ok(Some(_)) => continue,
                Ok(None) => {}
                Err(_) => unreachable!("timer cancellation credit was preflighted"),
            }
            match self
                .queue
                .push_host_request_cancellation(*generation, identity.clone(), parent)
            {
                Ok(Some(_)) => continue,
                Ok(None) => {}
                Err(_) => unreachable!("host cancellation credit was preflighted"),
            }
            match self.queue.push_framework_service_request_cancellation(
                *generation,
                identity.clone(),
                parent,
            ) {
                Ok(Some(_)) => continue,
                Ok(None) => {}
                Err(_) => unreachable!("framework service cancellation credit was preflighted"),
            }
            self.queue
                .push_cancellation_rooted(*generation, identity, parent, root_quota)
                .unwrap_or_else(|_| unreachable!("application root quota was preflighted"));
        }
    }

    pub(crate) fn pop_work(&mut self) -> Option<WorkEnvelope<Action>> {
        self.queue.pop()
    }

    pub(crate) fn subscription_diagnostics(&self) -> &[SubscriptionDiagnostic] {
        &self.subscription_diagnostics
    }
}
