use runenui_core::{MonotonicInstant, SemanticActionTarget};

use crate::{
    TraceActionCategory, TraceActionIdentity, TraceContext, TraceSurfaceContext,
    TraceSurfaceIngressKind, queue::SemanticCommandQueueTarget, trace::TraceRecordDraft,
};

use super::{
    ActionCommitError, CommandOrigin, CommandSubmission, HashMap, HostProtocol, MandatoryTracePlan,
    MountedNodeId, QueueCommitError, Runtime, RuntimeStatus, RuntimeTerminalReason,
    SemanticCommand, SubmitActionError, SubmitActionResult, SubmitCommandError,
    SubmitCommandErrorKind, SubscriptionDiagnostic, TargetStatus, TraceRecordKind, TraceSequence,
    TraceTarget, TraceWorkIdentity, UnacceptedCommand, WorkEnvelope, WorkSequence,
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

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
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

    pub(in crate::runtime) fn commit_preflighted_action(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        category: TraceActionCategory,
        instant: MonotonicInstant,
    ) -> Result<WorkSequence, ActionCommitError<Action>> {
        match self.queue.preflight_commit(1) {
            Ok(()) => {}
            Err(QueueCommitError::Full) => return Err(ActionCommitError::QueueFull(action)),
            Err(QueueCommitError::SequenceExhausted) => {
                return Err(ActionCommitError::WorkSequenceExhausted(action));
            }
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
            .push_preflighted(action, accepted, target)
            .map_err(ActionCommitError::Integrity)
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
    ) -> Result<CommandSubmission, SubmitCommandError> {
        match self.submit_command_inner(&target, command, origin, CommandTrace::Direct { parent }) {
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
        self.submit_command_inner(target, command, origin, CommandTrace::Surface(trace))
    }

    pub(in crate::runtime) fn submit_semantic_action_command(
        &mut self,
        target: &MountedNodeId,
        command: SemanticCommand,
        semantic_target: SemanticActionTarget,
    ) -> Result<CommandSubmission, SubmitCommandErrorKind> {
        self.command_preflight(target)?;
        self.commit_preflighted_command(
            target,
            command,
            CommandOrigin::accessibility(),
            Some(semantic_target),
            self.now(),
            CommandTrace::Direct { parent: None },
        )
    }

    fn submit_command_inner(
        &mut self,
        target: &MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
        trace: CommandTrace,
    ) -> Result<CommandSubmission, SubmitCommandErrorKind> {
        self.command_preflight(target)?;
        self.commit_preflighted_command(target, command, origin, None, self.now(), trace)
    }

    fn command_preflight(&self, target: &MountedNodeId) -> Result<(), SubmitCommandErrorKind> {
        self.command_status_preflight()?;
        match self.tree.target_status(target) {
            TargetStatus::Live => {}
            TargetStatus::Foreign => return Err(SubmitCommandErrorKind::ForeignTarget),
            TargetStatus::Stale => return Err(SubmitCommandErrorKind::StaleTarget),
            TargetStatus::Missing => return Err(SubmitCommandErrorKind::MissingTarget),
        }
        match self.queue.preflight_commit(1) {
            Ok(()) => Ok(()),
            Err(QueueCommitError::Full) => Err(SubmitCommandErrorKind::Full),
            Err(QueueCommitError::SequenceExhausted) => {
                Err(SubmitCommandErrorKind::WorkSequenceExhausted)
            }
        }
    }

    pub(in crate::runtime) fn commit_preflighted_routed_command(
        &mut self,
        target: &MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
        causal_parent: Option<TraceSequence>,
        instant: MonotonicInstant,
    ) -> Result<CommandSubmission, SubmitCommandErrorKind> {
        self.command_preflight(target)?;
        self.commit_preflighted_command(
            target,
            command,
            origin,
            None,
            instant,
            CommandTrace::Direct {
                parent: causal_parent,
            },
        )
    }

    fn commit_preflighted_command(
        &mut self,
        target: &MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
        semantic_target: Option<SemanticActionTarget>,
        instant: MonotonicInstant,
        trace: CommandTrace,
    ) -> Result<CommandSubmission, SubmitCommandErrorKind> {
        let trace_reservation = match (&trace, semantic_target.as_ref()) {
            (CommandTrace::Direct { .. }, Some(_)) => self.trace.reserve_semantic_action_outcome(),
            (CommandTrace::Direct { .. }, None) => self.trace.reserve_command_outcome(),
            (CommandTrace::Surface(_), _) => self.trace.reserve_surface_command_outcome(),
        }
        .ok_or(SubmitCommandErrorKind::TraceSequenceExhausted)?;
        let sequence = self
            .queue
            .next_sequence()
            .unwrap_or_else(|| unreachable!("command sequence was preflighted"));
        let trace_enabled = self.trace.is_enabled();
        let trace_target = self.tree.trace_target(target);
        let causal_parent = match trace {
            CommandTrace::Direct { parent } => {
                let bound_parent = if let Some(semantic_target) = semantic_target.as_ref() {
                    let bound = self.trace.record_event(
                        TraceRecordKind::SemanticActionBound {
                            target: semantic_target.clone(),
                            command,
                        },
                        sequence,
                        parent,
                        Some(trace_target.clone()),
                        instant,
                        target,
                        None,
                        origin,
                    );
                    if trace_enabled && bound.is_none() {
                        self.trace.release_reservation(trace_reservation);
                        return Err(SubmitCommandErrorKind::TraceSequenceExhausted);
                    }
                    bound
                } else {
                    parent
                };
                self.trace.record_event(
                    TraceRecordKind::CommandSubmissionAccepted,
                    sequence,
                    bound_parent,
                    Some(trace_target),
                    instant,
                    target,
                    None,
                    origin,
                )
            }
            CommandTrace::Surface(surface) => {
                let context_parent = if trace_enabled {
                    self.trace.record_draft(
                        TraceRecordDraft::surface_fact(
                            TraceRecordKind::SurfaceContextAccepted {
                                ingress: surface.ingress,
                            },
                            instant,
                            surface.surface,
                        )
                        .with_work_sequence(Some(sequence)),
                    )
                } else {
                    None
                };
                if trace_enabled && context_parent.is_none() {
                    self.trace.release_reservation(trace_reservation);
                    return Err(SubmitCommandErrorKind::TraceSequenceExhausted);
                }
                let target_parent = self.trace.record_event(
                    TraceRecordKind::SurfaceTargetBound,
                    sequence,
                    context_parent,
                    Some(trace_target.clone()),
                    instant,
                    target,
                    None,
                    origin,
                );
                if trace_enabled && target_parent.is_none() {
                    self.trace.release_reservation(trace_reservation);
                    return Err(SubmitCommandErrorKind::TraceSequenceExhausted);
                }
                self.trace.record_event(
                    TraceRecordKind::CommandSubmissionAccepted,
                    sequence,
                    target_parent,
                    Some(trace_target),
                    instant,
                    target,
                    None,
                    origin,
                )
            }
        };
        if trace_enabled && causal_parent.is_none() {
            self.trace.release_reservation(trace_reservation);
            return Err(SubmitCommandErrorKind::TraceSequenceExhausted);
        }
        let queued_target = semantic_target.map_or_else(
            || SemanticCommandQueueTarget::mounted(target.clone()),
            |semantic_target| SemanticCommandQueueTarget::semantic(target.clone(), semantic_target),
        );
        self.queue
            .push_command_preflighted(
                queued_target,
                command,
                origin,
                instant,
                causal_parent,
                trace_reservation,
            )
            .unwrap_or_else(|_| unreachable!("command queue was preflighted"));
        self.external_queue_commit_accepted();
        Ok(CommandSubmission::new(sequence))
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
            self.queue
                .push_cancellation(*generation, identity, parent)
                .unwrap_or_else(|_| unreachable!("transaction queue was preflighted"));
        }
    }

    pub(crate) fn pop_work(&mut self) -> Option<WorkEnvelope<Action>> {
        self.queue.pop()
    }

    pub(crate) fn subscription_diagnostics(&self) -> &[SubscriptionDiagnostic] {
        &self.subscription_diagnostics
    }
}
