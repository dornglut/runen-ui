use super::{
    ActionCommitError, ApplicationActionOrigin, CommandOrigin, CommandSubmission, HashMap,
    HostProtocol, MandatoryTracePlan, MountedNodeId, QueueCommitError, Runtime, RuntimeStatus,
    RuntimeTerminalReason, SemanticCommand, SubmitActionError, SubmitActionResult,
    SubmitCommandError, SubmitCommandErrorKind, SubscriptionDiagnostic, TargetStatus,
    TraceRecordKind, TraceSequence, TraceTarget, TraceWorkIdentity, UnacceptedCommand,
    WorkEnvelope, WorkSequence,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn submit_action(
        &mut self,
        action: Action,
        origin: ApplicationActionOrigin,
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
        let sequence = match self.commit_preflighted_action(action, causal_parent, target, origin) {
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
        origin: ApplicationActionOrigin,
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
        let accepted = self.trace.record(
            TraceRecordKind::ActionSubmissionAccepted,
            Some(sequence),
            causal_parent,
            None,
            None,
            target.clone(),
        );
        if trace_enabled && accepted.is_none() {
            return Err(ActionCommitError::TraceSequenceExhausted(action));
        }
        self.queue
            .push_preflighted(action, accepted, target, origin)
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
        match self.status {
            RuntimeStatus::Running => {}
            RuntimeStatus::Closed => {
                return Err(Self::reject_command_submission(
                    SubmitCommandErrorKind::Closed,
                    target,
                    command,
                    origin,
                ));
            }
            RuntimeStatus::Terminal(reason) => {
                return Err(Self::reject_command_submission(
                    SubmitCommandErrorKind::Terminal(reason),
                    target,
                    command,
                    origin,
                ));
            }
        }
        let target_error = match self.tree.target_status(&target) {
            TargetStatus::Live => None,
            TargetStatus::Foreign => Some(SubmitCommandErrorKind::ForeignTarget),
            TargetStatus::Stale => Some(SubmitCommandErrorKind::StaleTarget),
            TargetStatus::Missing => Some(SubmitCommandErrorKind::MissingTarget),
        };
        if let Some(kind) = target_error {
            return Err(Self::reject_command_submission(
                kind, target, command, origin,
            ));
        }
        match self.queue.preflight_commit(1) {
            Ok(()) => {}
            Err(QueueCommitError::Full) => {
                return Err(Self::reject_command_submission(
                    SubmitCommandErrorKind::Full,
                    target,
                    command,
                    origin,
                ));
            }
            Err(QueueCommitError::SequenceExhausted) => {
                let error = Self::reject_command_submission(
                    SubmitCommandErrorKind::WorkSequenceExhausted,
                    target,
                    command,
                    origin,
                );
                self.enter_terminal(RuntimeTerminalReason::WorkSequenceExhausted, 0);
                return Err(error);
            }
        }
        let Some(trace_reservation) = self.trace.reserve_command_outcome() else {
            let error = Self::reject_command_submission(
                SubmitCommandErrorKind::TraceSequenceExhausted,
                target,
                command,
                origin,
            );
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return Err(error);
        };
        let sequence = self
            .queue
            .next_sequence()
            .unwrap_or_else(|| unreachable!("command sequence was preflighted"));
        let instant = self.now();
        let trace_enabled = self.trace.is_enabled();
        let causal_parent = self.trace.record_event(
            TraceRecordKind::CommandSubmissionAccepted,
            sequence,
            None,
            Some(self.tree.trace_target(&target)),
            instant,
            &target,
            None,
            origin,
        );
        if trace_enabled && causal_parent.is_none() {
            self.trace.release_reservation(trace_reservation);
            let error = Self::reject_command_submission(
                SubmitCommandErrorKind::TraceSequenceExhausted,
                target,
                command,
                origin,
            );
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return Err(error);
        }
        self.queue
            .push_command_preflighted(
                target,
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
