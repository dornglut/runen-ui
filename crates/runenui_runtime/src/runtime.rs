//! Persistent runtime state, action transactions, and terminal authority.

#![allow(clippy::redundant_pub_crate)]

use core::fmt;

use runenui_core::{Element, ElementKey};

use crate::{
    FocusState, MountedNodeId, RuntimeConfig, SubmitActionError, SubmitActionResult, Trace,
    TraceRecordKind, TraceSequence, TraceTarget, WorkSequence,
    app::UiApp,
    mounted::{MountedTree, TargetStatus},
    queue::{ApplicationActionEnvelope, ApplicationActionOrigin, WorkQueue},
};

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeTerminalReason {
    WorkSequenceExhausted,
    ReconciliationGenerationExhausted,
    TraceSequenceExhausted,
}

impl fmt::Display for RuntimeTerminalReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkSequenceExhausted => formatter.write_str("work sequence exhausted"),
            Self::ReconciliationGenerationExhausted => {
                formatter.write_str("reconciliation generation exhausted")
            }
            Self::TraceSequenceExhausted => formatter.write_str("trace sequence exhausted"),
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeStatus {
    Running,
    Terminal(RuntimeTerminalReason),
    Closed,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    WidgetStatePayloadMismatch,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WidgetStatePayloadMismatch => {
                formatter.write_str("mounted widget state payload mismatch")
            }
        }
    }
}

impl std::error::Error for RuntimeError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ReconciliationGeneration(u64);

impl ReconciliationGeneration {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationReport {
    generation: ReconciliationGeneration,
    live_node_count: usize,
    mounted_count: usize,
    updated_count: usize,
    unmounted_count: usize,
    moved_count: usize,
    retained_focus: bool,
    diagnostics: Vec<ReconciliationDiagnostic>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReconciliationDiagnostic {
    DuplicateSiblingKey {
        key: ElementKey,
        parent_path: String,
        old_occurrence_paths: Vec<String>,
        new_occurrence_paths: Vec<String>,
    },
    StatePayloadMismatch {
        path: String,
    },
}

impl fmt::Display for ReconciliationDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSiblingKey {
                key,
                parent_path,
                old_occurrence_paths,
                new_occurrence_paths,
            } => write!(
                formatter,
                "duplicate sibling key {:?} under {parent_path}; old=[{}], new=[{}]",
                key.as_str(),
                old_occurrence_paths.join(", "),
                new_occurrence_paths.join(", ")
            ),
            Self::StatePayloadMismatch { path } => {
                write!(formatter, "mounted widget state payload mismatch at {path}")
            }
        }
    }
}

impl ReconciliationReport {
    #[must_use]
    pub const fn generation(&self) -> ReconciliationGeneration {
        self.generation
    }
    #[must_use]
    pub const fn live_node_count(&self) -> usize {
        self.live_node_count
    }
    #[must_use]
    pub const fn mounted_count(&self) -> usize {
        self.mounted_count
    }
    #[must_use]
    pub const fn updated_count(&self) -> usize {
        self.updated_count
    }
    #[must_use]
    pub const fn unmounted_count(&self) -> usize {
        self.unmounted_count
    }
    #[must_use]
    pub const fn moved_count(&self) -> usize {
        self.moved_count
    }
    #[must_use]
    pub const fn retained_focus(&self) -> bool {
        self.retained_focus
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[ReconciliationDiagnostic] {
        &self.diagnostics
    }
}

/// Result of one explicit, idempotent runtime shutdown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShutdownReport {
    already_complete: bool,
    cancelled_queued_envelopes: usize,
    unmounted_lifetimes: usize,
}

impl ShutdownReport {
    #[must_use]
    pub const fn already_complete(self) -> bool {
        self.already_complete
    }
    #[must_use]
    pub const fn cancelled_queued_envelopes(self) -> usize {
        self.cancelled_queued_envelopes
    }
    #[must_use]
    pub const fn unmounted_lifetimes(self) -> usize {
        self.unmounted_lifetimes
    }
}

pub(crate) struct Runtime<State, Action> {
    state: Option<State>,
    pub(crate) tree: MountedTree<Action>,
    queue: WorkQueue<Action>,
    trace: Trace,
    focus: FocusState,
    generation: u64,
    report: ReconciliationReport,
    status: RuntimeStatus,
    #[cfg(test)]
    readiness_checkpoint_count: usize,
}

pub(crate) enum ProcessApplicationActionOutcome {
    Completed,
    Terminal {
        reason: RuntimeTerminalReason,
        cancelled: usize,
    },
}

impl<State, Action> Runtime<State, Action> {
    pub(crate) fn mount(
        state: State,
        root: impl FnOnce(&State) -> Element<Action>,
        config: RuntimeConfig,
    ) -> Self {
        let transient = root(&state);
        let (tree, reconcile_stats) = MountedTree::mount(transient);
        let mut trace = Trace::new(config.trace_config());
        trace.record(
            TraceRecordKind::RuntimeMounted,
            None,
            None,
            None,
            None,
            None,
        );
        let report = ReconciliationReport {
            generation: ReconciliationGeneration(1),
            live_node_count: tree.live_count(),
            mounted_count: reconcile_stats.mounted,
            updated_count: 0,
            unmounted_count: 0,
            moved_count: 0,
            retained_focus: false,
            diagnostics: reconcile_stats.diagnostics,
        };
        Self {
            state: Some(state),
            tree,
            queue: WorkQueue::new(config.queue_capacity()),
            trace,
            focus: FocusState::new(),
            generation: 1,
            report,
            status: RuntimeStatus::Running,
            #[cfg(test)]
            readiness_checkpoint_count: 0,
        }
    }

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
        if !self.queue.has_sequence() {
            let reason = RuntimeTerminalReason::WorkSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(SubmitActionError::Terminal { action, reason });
        }
        if !self.trace.can_record_mandatory(1) {
            let reason = RuntimeTerminalReason::TraceSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(SubmitActionError::Terminal { action, reason });
        }
        let trace_target = target.clone();
        let sequence = match self
            .queue
            .push_preflighted(action, causal_parent, target, origin)
        {
            Ok(sequence) => sequence,
            Err(action) => {
                let reason = RuntimeTerminalReason::WorkSequenceExhausted;
                self.enter_terminal(reason, 0);
                return Err(SubmitActionError::Terminal { action, reason });
            }
        };
        self.trace.record(
            TraceRecordKind::ActionSubmissionAccepted,
            Some(sequence),
            causal_parent,
            None,
            None,
            trace_target,
        );
        Ok(sequence)
    }

    pub(crate) fn activation_preflight(
        &mut self,
        target: &TraceTarget,
    ) -> Result<(), RuntimeStatus> {
        match self.status {
            RuntimeStatus::Running => {}
            status => return Err(status),
        }
        if self.next_generation().is_none() {
            let reason = RuntimeTerminalReason::ReconciliationGenerationExhausted;
            self.enter_terminal(reason, 0);
            return Err(RuntimeStatus::Terminal(reason));
        }
        if self.queue.is_full() {
            self.record_optional(
                TraceRecordKind::ActivationRejectedFull,
                None,
                None,
                Some(target.clone()),
            );
            return Err(RuntimeStatus::Running);
        }
        if !self.queue.has_sequence() {
            let reason = RuntimeTerminalReason::WorkSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(RuntimeStatus::Terminal(reason));
        }
        if !self.trace.can_record_mandatory(2) {
            let reason = RuntimeTerminalReason::TraceSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(RuntimeStatus::Terminal(reason));
        }
        Ok(())
    }

    pub(crate) fn commit_activation(
        &mut self,
        action: Option<Action>,
        target: TraceTarget,
        origin: ApplicationActionOrigin,
    ) -> Option<WorkSequence> {
        let causal_parent = self.trace.record(
            TraceRecordKind::ActivationCommitted,
            None,
            None,
            None,
            None,
            Some(target.clone()),
        );
        let action = action?;
        let sequence =
            match self
                .queue
                .push_preflighted(action, causal_parent, Some(target.clone()), origin)
            {
                Ok(sequence) => sequence,
                Err(_action) => {
                    self.enter_terminal(RuntimeTerminalReason::WorkSequenceExhausted, 0);
                    return None;
                }
            };
        self.trace.record(
            TraceRecordKind::ActionSubmissionAccepted,
            Some(sequence),
            causal_parent,
            None,
            None,
            Some(target),
        );
        Some(sequence)
    }

    pub(crate) fn pop_application_action(&mut self) -> Option<ApplicationActionEnvelope<Action>> {
        self.queue.pop_application_action()
    }

    pub(crate) fn queued_len(&self) -> usize {
        self.queue.len()
    }

    pub(crate) fn queue_is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub(crate) fn enter_terminal(
        &mut self,
        reason: RuntimeTerminalReason,
        additional_cancelled: usize,
    ) -> usize {
        if !matches!(self.status, RuntimeStatus::Running) {
            return 0;
        }
        let cancelled = self.queue.cancel_all().saturating_add(additional_cancelled);
        self.status = RuntimeStatus::Terminal(reason);
        self.record_optional(
            TraceRecordKind::RuntimeTerminal { reason },
            None,
            None,
            None,
        );
        if cancelled > 0 {
            self.record_optional(
                TraceRecordKind::QueuedWorkCancelled { count: cancelled },
                None,
                None,
                None,
            );
        }
        cancelled
    }

    pub(crate) fn record_optional(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: Option<WorkSequence>,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
    ) {
        if self.trace.can_record_mandatory(1) {
            self.trace
                .record(kind, work_sequence, causal_parent, None, None, target);
        }
    }

    const fn next_generation(&self) -> Option<u64> {
        self.generation.checked_add(1)
    }

    fn validate_focus(&mut self, id: &MountedNodeId) -> bool {
        self.tree.target_status(id) == TargetStatus::Live
            && self
                .tree
                .activation(id)
                .is_ok_and(|activation| activation.enabled() && activation.is_actionable())
    }

    #[must_use]
    pub(crate) const fn state(&self) -> &State {
        match &self.state {
            Some(state) => state,
            None => unreachable!(),
        }
    }
    #[must_use]
    pub(crate) const fn trace(&self) -> &Trace {
        &self.trace
    }
    #[must_use]
    pub(crate) const fn focus(&self) -> &FocusState {
        &self.focus
    }
    pub(crate) fn set_focus(&mut self, id: MountedNodeId) {
        self.focus.set(id);
    }
    pub(crate) fn clear_focus(&mut self) {
        self.focus.clear();
    }
    #[must_use]
    pub(crate) const fn report(&self) -> &ReconciliationReport {
        &self.report
    }
    #[must_use]
    pub(crate) const fn status(&self) -> RuntimeStatus {
        self.status
    }

    #[cfg(test)]
    pub(crate) const fn note_readiness_checkpoint(&mut self) {
        self.readiness_checkpoint_count += 1;
    }

    #[cfg(test)]
    pub(crate) const fn readiness_checkpoint_count_for_test(&self) -> usize {
        self.readiness_checkpoint_count
    }

    pub(crate) fn shutdown(&mut self) -> ShutdownReport {
        if matches!(self.status, RuntimeStatus::Closed) {
            return ShutdownReport {
                already_complete: true,
                cancelled_queued_envelopes: 0,
                unmounted_lifetimes: 0,
            };
        }
        let cancelled_queued_envelopes = self.queue.cancel_all();
        let stats = self.tree.shutdown();
        self.focus.clear();
        self.record_optional(
            TraceRecordKind::RuntimeShutdown {
                cancelled_queued: cancelled_queued_envelopes,
                unmounted_lifetimes: stats.unmounted,
            },
            None,
            None,
            None,
        );
        self.status = RuntimeStatus::Closed;
        ShutdownReport {
            already_complete: false,
            cancelled_queued_envelopes,
            unmounted_lifetimes: stats.unmounted,
        }
    }

    pub(crate) fn into_state(mut self) -> State {
        self.shutdown();
        self.state
            .take()
            .unwrap_or_else(|| unreachable!("state is returned exactly once"))
    }

    #[cfg(any(test, feature = "internal-test-seams"))]
    pub(crate) const fn seed_generation_for_test(&mut self, generation: u64) {
        self.generation = generation;
    }

    #[cfg(any(test, feature = "internal-test-seams"))]
    pub(crate) const fn seed_next_work_sequence_for_test(&mut self, next: u64) {
        self.queue.seed_next_sequence_for_test(next);
    }

    #[cfg(any(test, feature = "internal-test-seams"))]
    pub(crate) const fn seed_next_trace_sequence_for_test(&mut self, next: u64) {
        self.trace.seed_next_sequence_for_test(next);
    }
}

pub(crate) fn process_application_action<App: UiApp>(
    runtime: &mut Runtime<App::State, App::Action>,
    envelope: ApplicationActionEnvelope<App::Action>,
) -> ProcessApplicationActionOutcome {
    let ApplicationActionEnvelope {
        sequence,
        action,
        causal_parent,
        target,
        origin: _origin,
    } = envelope;
    let before = ReconciliationGeneration(runtime.generation);
    let Some(next) = runtime.next_generation() else {
        let reason = RuntimeTerminalReason::ReconciliationGenerationExhausted;
        let cancelled = runtime.enter_terminal(reason, 1);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    };
    let trace_count = if runtime.focus.focused_node().is_some() {
        4
    } else {
        3
    };
    if !runtime.trace.can_record_mandatory(trace_count) {
        let reason = RuntimeTerminalReason::TraceSequenceExhausted;
        let cancelled = runtime.enter_terminal(reason, 1);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    }
    runtime.trace.record(
        TraceRecordKind::ApplicationActionTransactionStarted,
        Some(sequence),
        causal_parent,
        Some(before),
        None,
        target.clone(),
    );
    let app_state = runtime
        .state
        .as_mut()
        .unwrap_or_else(|| unreachable!("live runtime retains application state"));
    App::update(app_state, action);
    runtime.trace.record(
        TraceRecordKind::ApplicationStateUpdated,
        Some(sequence),
        causal_parent,
        Some(before),
        None,
        target.clone(),
    );
    let transient = App::root(app_state);
    let previous_focus = runtime.focus.focused_node().cloned();
    let reconcile_stats = runtime.tree.reconcile(transient);
    runtime.generation = next;
    let after = ReconciliationGeneration(next);
    let retained_focus = previous_focus
        .as_ref()
        .is_some_and(|id| runtime.validate_focus(id));
    if !retained_focus && previous_focus.is_some() {
        runtime.focus.clear();
        runtime.trace.record(
            TraceRecordKind::FocusCleared,
            Some(sequence),
            causal_parent,
            Some(before),
            Some(after),
            target.clone(),
        );
    } else if retained_focus {
        runtime.trace.record(
            TraceRecordKind::FocusRetained,
            Some(sequence),
            causal_parent,
            Some(before),
            Some(after),
            target.clone(),
        );
    }
    runtime.tree.finish_focus_validation();
    runtime.report = ReconciliationReport {
        generation: after,
        live_node_count: runtime.tree.live_count(),
        mounted_count: reconcile_stats.mounted,
        updated_count: reconcile_stats.updated,
        unmounted_count: reconcile_stats.unmounted,
        moved_count: reconcile_stats.moved,
        retained_focus,
        diagnostics: reconcile_stats.diagnostics,
    };
    runtime.trace.record(
        TraceRecordKind::TreeReconciled,
        Some(sequence),
        causal_parent,
        Some(before),
        Some(after),
        target,
    );
    ProcessApplicationActionOutcome::Completed
}

impl<State, Action> Drop for Runtime<State, Action> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
