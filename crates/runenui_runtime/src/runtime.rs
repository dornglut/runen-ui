//! Persistent runtime state, action transactions, and terminal authority.

#![allow(clippy::redundant_pub_crate)]

mod routed;

use core::fmt;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use runenui_core::{
    __runtime::{Effect, MountedEffect, SendOutput, Subscription},
    CommandOrigin, Element, ElementKey, HostProtocol, IntoEffects, NoHostProtocol, SemanticCommand,
    SendSubscriptionSink, SendSubscriptionStartOutcome, SendTaskStartFailure, SubscriptionSet,
    UiApp, View,
};

use crate::trace::MandatoryTracePlan;
use crate::{
    CommandSubmission, FocusState, ManualClock, MonotonicClock, MonotonicInstant, MountedNodeId,
    RuntimeConfig, SendTaskExecutor, SendTaskStartError, SendTaskStartOutcome, SubmitActionError,
    SubmitActionResult, SubmitCommandError, SubmitCommandErrorKind, Trace, TraceRecordKind,
    TraceSequence, TraceTarget, TraceTimerTerminalOutcome, TraceWorkIdentity, TraceWorkOwner,
    TraceWorkStartRefusal, UnacceptedCommand, WorkSequence,
    completion::{
        CompletionIngress, CompletionKind, HostResponseCompletion, SendTaskJob, UnavailableExecutor,
    },
    mounted::{MountedIdentityExhausted, MountedTree, TargetStatus},
    queue::{
        ApplicationActionEnvelope, ApplicationActionOrigin, QueueCommitError, WorkEnvelope,
        WorkQueue,
    },
    transaction::{
        ApplicationTransactionInput, OwnedTransactionLedger, PlannedApplicationTransaction,
        PlannedOutput, PlannedStartPayload, PlannedWorkSemanticEvent, TransactionLedger,
        TransactionPlanError,
    },
    wake::WakeState,
    work::{
        RegistryInsertError, WorkCancellationCounts, WorkFamily, WorkOwner, WorkRegistry,
        WorkTraceIdentity,
        host_request::{HostRequestRef, HostRequestToken, LiveHostRequest},
        subscription::{LiveSubscription, LiveSubscriptionSource, SubscriptionPoll},
        task::{LocalTask, TaskReady},
        timer::{Timer, TimerFireOutcome, TimerStartError},
    },
};

enum ActionCommitError<Action> {
    QueueFull(Action),
    WorkSequenceExhausted(Action),
    TraceSequenceExhausted(Action),
    Integrity(Action),
}

enum CollectedRoutedOutput<Action> {
    Action {
        action: Action,
        causal_parent: Option<TraceSequence>,
        current_target: MountedNodeId,
    },
    Command {
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
        causal_parent: Option<TraceSequence>,
    },
}

#[derive(Clone, Copy)]
enum MutationPhase {
    PreMutation,
    Mutated,
}

impl MutationPhase {
    const fn terminal_reason(
        self,
        pre_mutation_reason: RuntimeTerminalReason,
    ) -> RuntimeTerminalReason {
        match self {
            Self::PreMutation => pre_mutation_reason,
            Self::Mutated => RuntimeTerminalReason::Poisoned,
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeTerminalReason {
    WorkSequenceExhausted,
    WorkGenerationExhausted,
    ReconciliationGenerationExhausted,
    MountedIdentityExhausted,
    TraceSequenceExhausted,
    Poisoned,
}

impl fmt::Display for RuntimeTerminalReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkSequenceExhausted => formatter.write_str("work sequence exhausted"),
            Self::WorkGenerationExhausted => formatter.write_str("work generation exhausted"),
            Self::ReconciliationGenerationExhausted => {
                formatter.write_str("reconciliation generation exhausted")
            }
            Self::MountedIdentityExhausted => {
                formatter.write_str("mounted identity capacity exhausted")
            }
            Self::TraceSequenceExhausted => formatter.write_str("trace sequence exhausted"),
            Self::Poisoned => formatter.write_str("runtime integrity poisoned after mutation"),
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
pub enum TimerStartOutcome {
    Started,
    ZeroInterval,
    DeadlineOverflow,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerFiringOutcome {
    Completed,
    Rescheduled,
    RepeatDeadlineOverflow,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubscriptionOwnerKind {
    Application,
    Mounted,
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubscriptionDiagnostic {
    DuplicateKey {
        owner: SubscriptionOwnerKind,
        key: runenui_core::WorkKey,
    },
}

pub enum HostResponseError<Response> {
    ForeignRuntime(Response),
    Stale(Response),
    MismatchedKind(Response),
    Full(Response),
    Closed(Response),
    Terminal {
        response: Response,
        reason: RuntimeTerminalReason,
    },
}

impl<Response> fmt::Debug for HostResponseError<Response> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ForeignRuntime(_) => "HostResponseError::ForeignRuntime(..)",
            Self::Stale(_) => "HostResponseError::Stale(..)",
            Self::MismatchedKind(_) => "HostResponseError::MismatchedKind(..)",
            Self::Full(_) => "HostResponseError::Full(..)",
            Self::Closed(_) => "HostResponseError::Closed(..)",
            Self::Terminal { .. } => "HostResponseError::Terminal { .. }",
        })
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostRequestCancelError {
    ForeignRuntime,
    Stale,
    Full,
    Closed,
    Terminal(RuntimeTerminalReason),
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
    cancelled_live_work: WorkCancellationCounts,
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
    #[must_use]
    pub const fn cancelled_local_tasks(self) -> usize {
        self.cancelled_live_work.local_tasks
    }
    #[must_use]
    pub const fn cancelled_send_tasks(self) -> usize {
        self.cancelled_live_work.send_tasks
    }
    #[must_use]
    pub const fn cancelled_timers(self) -> usize {
        self.cancelled_live_work.timers
    }
    #[must_use]
    pub const fn cancelled_subscriptions(self) -> usize {
        self.cancelled_live_work.subscriptions
    }
    #[must_use]
    pub const fn cancelled_host_requests(self) -> usize {
        self.cancelled_live_work.host_requests
    }
}

pub(crate) struct Runtime<State, Action, Protocol: HostProtocol = NoHostProtocol> {
    state: Option<State>,
    pub(crate) tree: MountedTree<Action>,
    queue: WorkQueue<Action>,
    trace: Trace,
    focus: FocusState,
    generation: u64,
    report: ReconciliationReport,
    status: RuntimeStatus,
    limits: crate::RuntimeLimits,
    mounted_public_slot_limit: u64,
    work: WorkRegistry<Action, Protocol>,
    mounted_subscription_reconcile_pending: Vec<MountedNodeId>,
    initial_mounted_subscription_owners: Vec<MountedNodeId>,
    initial_mounted_outputs: Vec<(MountedNodeId, Vec<MountedEffect<Action>>)>,
    subscriptions: Vec<LiveSubscription<Action>>,
    subscription_diagnostics: Vec<SubscriptionDiagnostic>,
    clock: ManualClock,
    local_tasks: Vec<LocalTask<Action>>,
    timers: Vec<Timer<Action>>,
    completion_ingress: CompletionIngress,
    send_executor: Box<dyn SendTaskExecutor>,
    send_task_mappers: Vec<SendTaskMapper<Action>>,
    last_send_task_start_outcome: Option<SendTaskStartOutcome>,
    last_timer_start_outcome: Option<TimerStartOutcome>,
    last_timer_firing_outcome: Option<TimerFiringOutcome>,
    host_clock: Option<Box<dyn MonotonicClock>>,
    host_namespace: Arc<()>,
    host_requests: Vec<LiveHostRequest<Action, Protocol>>,
    redraw_namespace: Arc<()>,
    redraw_revision: u64,
    redraw_acknowledged: u64,
    wake: WakeState,
    #[cfg(test)]
    readiness_checkpoint_count: usize,
    #[cfg(feature = "internal-test-seams")]
    routed_callback_bridge_failure_for_test: bool,
    #[cfg(feature = "internal-test-seams")]
    routed_semantic_default_failure_for_test: bool,
    #[cfg(feature = "internal-test-seams")]
    routed_commit_failure_for_test: bool,
}

pub(crate) enum ProcessApplicationActionOutcome {
    Completed,
    Terminal {
        reason: RuntimeTerminalReason,
        cancelled: usize,
    },
}

pub(crate) struct ReadinessCheckpointReport {
    pub(crate) imported_completions: usize,
    pub(crate) polled_local_work: usize,
    pub(crate) promoted_timers: usize,
}

#[allow(clippy::struct_excessive_bools)]
pub(crate) struct SchedulerObservation {
    pub(crate) completion_imports_pending: bool,
    pub(crate) due_timers_pending: bool,
    pub(crate) local_polls_pending: bool,
    pub(crate) mandatory_derived_work_pending: bool,
    pub(crate) next_deadline: Option<MonotonicInstant>,
    pub(crate) publication_dirty: bool,
}

struct SendTaskMapper<Action> {
    generation: crate::work::WorkGeneration,
    map: Box<dyn FnOnce(SendOutput) -> Action>,
}

struct SubscriptionDiff<Action> {
    invalidated: Vec<crate::work::WorkGeneration>,
    starts: Vec<Subscription<Action>>,
    duplicate_keys: HashSet<runenui_core::WorkKey>,
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn mount(
        state: State,
        root: impl FnOnce(&State) -> Element<Action>,
        config: RuntimeConfig,
    ) -> Self {
        let transient = root(&state);
        let mounted_public_slot_limit = config.mounted_public_slot_limit();
        let mounted =
            MountedTree::mount_with_public_slot_limit(transient, mounted_public_slot_limit);
        let mount_failed = mounted.is_err();
        let (tree, reconcile_stats, generation) = match mounted {
            Ok((tree, reconcile_stats)) => (tree, reconcile_stats, 1),
            Err(MountedIdentityExhausted) => (
                MountedTree::empty(),
                crate::mounted::ReconcileStats::default(),
                0,
            ),
        };
        let mut trace = Trace::new(config.trace_config());
        if !mount_failed {
            trace.record(
                TraceRecordKind::RuntimeMounted,
                None,
                None,
                None,
                None,
                None,
            );
        }
        let report = ReconciliationReport {
            generation: ReconciliationGeneration(generation),
            live_node_count: tree.live_count(),
            mounted_count: reconcile_stats.mounted,
            updated_count: 0,
            unmounted_count: 0,
            moved_count: 0,
            retained_focus: false,
            diagnostics: reconcile_stats.diagnostics,
        };
        let limits = config.limits();
        let wake = WakeState::new();
        let mounted_owners = reconcile_stats.mounted_owners;
        let queue = WorkQueue::new(config.queue_capacity());
        let work = WorkRegistry::new(limits);
        #[cfg(feature = "internal-test-seams")]
        let (queue, work) = {
            let mut queue = queue;
            let mut work = work;
            queue.seed_next_sequence_for_test(config.initial_next_work_sequence());
            work.seed_next_generation_for_test(config.initial_next_work_generation());
            (queue, work)
        };
        let mut runtime = Self {
            state: Some(state),
            tree,
            queue,
            trace,
            focus: FocusState::new(),
            generation,
            report,
            status: RuntimeStatus::Running,
            limits,
            mounted_public_slot_limit,
            work,
            mounted_subscription_reconcile_pending: Vec::new(),
            initial_mounted_subscription_owners: mounted_owners,
            initial_mounted_outputs: reconcile_stats.mounted_outputs,
            subscriptions: Vec::new(),
            subscription_diagnostics: Vec::new(),
            clock: ManualClock::new(),
            local_tasks: Vec::new(),
            timers: Vec::new(),
            completion_ingress: CompletionIngress::new(limits.completion_ingress(), wake.handle()),
            send_executor: Box::new(UnavailableExecutor),
            send_task_mappers: Vec::new(),
            last_send_task_start_outcome: None,
            last_timer_start_outcome: None,
            last_timer_firing_outcome: None,
            host_clock: None,
            host_namespace: Arc::new(()),
            host_requests: Vec::new(),
            redraw_namespace: Arc::new(()),
            redraw_revision: 1,
            redraw_acknowledged: 0,
            wake,
            #[cfg(test)]
            readiness_checkpoint_count: 0,
            #[cfg(feature = "internal-test-seams")]
            routed_callback_bridge_failure_for_test: false,
            #[cfg(feature = "internal-test-seams")]
            routed_semantic_default_failure_for_test: false,
            #[cfg(feature = "internal-test-seams")]
            routed_commit_failure_for_test: false,
        };
        if mount_failed {
            runtime.enter_terminal(RuntimeTerminalReason::MountedIdentityExhausted, 0);
        }
        runtime
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

    fn commit_preflighted_action(
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

    const fn reject_command_submission(
        kind: SubmitCommandErrorKind,
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
    ) -> SubmitCommandError {
        SubmitCommandError::new(kind, UnacceptedCommand::new(target, command, origin))
    }

    fn append_cancellation_envelopes(
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

    #[allow(clippy::too_many_lines)]
    pub(crate) fn process_effect_start(
        &mut self,
        sequence: WorkSequence,
        generation: crate::work::WorkGeneration,
    ) {
        let Some(family) = self.work.pending_family(generation) else {
            return;
        };
        let Some(identity) = self.trace_work_identity(generation) else {
            return;
        };
        if !self.trace.can_admit(MandatoryTracePlan::work_start(
            family == WorkFamily::HostRequest,
        )) {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        }
        self.record_work_fact_from_envelope(
            TraceRecordKind::WorkStartAttempted,
            sequence,
            identity.clone(),
        );
        match family {
            WorkFamily::LocalTask => {
                let Some(Effect::LocalTask(task)) = self.work.take_pending_effect(generation)
                else {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return;
                };
                if self.work.mark_running(generation).is_none() {
                    return;
                }
                self.local_tasks
                    .push(LocalTask::new(generation, task.future, self.wake.handle()));
                self.record_work_fact_from_envelope(
                    TraceRecordKind::WorkStartAccepted,
                    sequence,
                    identity,
                );
            }
            WorkFamily::Timer => {
                let Some(Effect::Timer(timer)) = self.work.take_pending_effect(generation) else {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return;
                };
                match Timer::new(generation, self.now(), timer) {
                    Ok(timer) => {
                        if self.work.mark_running(generation).is_none() {
                            return;
                        }
                        self.timers.push(timer);
                        self.last_timer_start_outcome = Some(TimerStartOutcome::Started);
                        self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartAccepted,
                            sequence,
                            identity,
                        );
                    }
                    Err(error) => {
                        let (outcome, refusal) = match error {
                            TimerStartError::ZeroInterval => (
                                TimerStartOutcome::ZeroInterval,
                                TraceWorkStartRefusal::TimerZeroInterval,
                            ),
                            TimerStartError::DeadlineOverflow => (
                                TimerStartOutcome::DeadlineOverflow,
                                TraceWorkStartRefusal::TimerDeadlineOverflow,
                            ),
                        };
                        self.last_timer_start_outcome = Some(outcome);
                        self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartRefused { outcome: refusal },
                            sequence,
                            identity,
                        );
                        self.revoke_generation(generation);
                    }
                }
            }
            WorkFamily::SendTask => {
                let Some(Effect::SendTask(task)) = self.work.take_pending_effect(generation) else {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return;
                };
                let job = SendTaskJob::new(
                    generation,
                    task.future,
                    self.completion_ingress.sender(),
                    identity.clone(),
                    self.work.trace_parent(generation),
                );
                self.completion_ingress
                    .register_send_task_starting(generation);
                match self.send_executor.start(job) {
                    Ok(()) => {
                        if !self
                            .completion_ingress
                            .promote_send_task_running(generation)
                            || self.work.mark_running(generation).is_none()
                        {
                            self.revoke_generation(generation);
                            return;
                        }
                        self.send_task_mappers.push(SendTaskMapper {
                            generation,
                            map: task.map,
                        });
                        self.last_send_task_start_outcome = Some(SendTaskStartOutcome::Started);
                        self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartAccepted,
                            sequence,
                            identity,
                        );
                    }
                    Err(error) => {
                        let (outcome, failure, refusal) = match error {
                            SendTaskStartError::Unavailable(_job) => (
                                SendTaskStartOutcome::Unavailable,
                                SendTaskStartFailure::Unavailable,
                                TraceWorkStartRefusal::ExecutorUnavailable,
                            ),
                            SendTaskStartError::Full(_job) => (
                                SendTaskStartOutcome::Full,
                                SendTaskStartFailure::Full,
                                TraceWorkStartRefusal::ExecutorFull,
                            ),
                            SendTaskStartError::Closed(_job) => (
                                SendTaskStartOutcome::Closed,
                                SendTaskStartFailure::Closed,
                                TraceWorkStartRefusal::ExecutorClosed,
                            ),
                            SendTaskStartError::Rejected(_job) => (
                                SendTaskStartOutcome::Rejected,
                                SendTaskStartFailure::Rejected,
                                TraceWorkStartRefusal::ExecutorRejected,
                            ),
                        };
                        self.last_send_task_start_outcome = Some(outcome);
                        let refusal_parent = self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartRefused { outcome: refusal },
                            sequence,
                            identity,
                        );
                        self.revoke_generation(generation);
                        if let Some(map) = task.start_failure
                            && self.callback_output_preflight(
                                None,
                                MandatoryTracePlan::typed_start_refusal_with_action(),
                            )
                        {
                            let action = map(failure);
                            self.queue_callback_action(action, refusal_parent);
                        }
                    }
                }
            }
            WorkFamily::Subscription => {
                let Some(subscription_index) = self
                    .subscriptions
                    .iter()
                    .position(|subscription| subscription.generation == generation)
                else {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return;
                };
                self.completion_ingress
                    .register_subscription_starting(generation);
                let sender = self.completion_ingress.sender();
                let completion_identity = identity.clone();
                let completion_parent = self.work.trace_parent(generation);
                let sink = SendSubscriptionSink::__runtime_new(move |output| {
                    sender.submit_subscription(
                        generation,
                        output,
                        completion_identity.clone(),
                        completion_parent,
                    )
                });
                let outcome = self.subscriptions[subscription_index].start_send(sink);
                match outcome {
                    None => {
                        self.completion_ingress.release_subscription(generation);
                        if self.work.mark_running(generation).is_none() {
                            return;
                        }
                        self.subscriptions[subscription_index].started = true;
                        self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartAccepted,
                            sequence,
                            identity,
                        );
                    }
                    Some(SendSubscriptionStartOutcome::Started) => {
                        if !self
                            .completion_ingress
                            .promote_subscription_running(generation)
                            || self.work.mark_running(generation).is_none()
                        {
                            self.completion_ingress.release_subscription(generation);
                            return;
                        }
                        self.subscriptions[subscription_index].started = true;
                        self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartAccepted,
                            sequence,
                            identity,
                        );
                    }
                    Some(outcome) => {
                        self.completion_ingress.release_subscription(generation);
                        let outcome = match outcome {
                            SendSubscriptionStartOutcome::Unavailable => {
                                TraceWorkStartRefusal::SubscriptionUnavailable
                            }
                            SendSubscriptionStartOutcome::Full => {
                                TraceWorkStartRefusal::SubscriptionFull
                            }
                            SendSubscriptionStartOutcome::Closed => {
                                TraceWorkStartRefusal::SubscriptionClosed
                            }
                            SendSubscriptionStartOutcome::Rejected => {
                                TraceWorkStartRefusal::SubscriptionRejected
                            }
                            SendSubscriptionStartOutcome::Started => unreachable!(),
                            _ => TraceWorkStartRefusal::SubscriptionRejected,
                        };
                        self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartRefused { outcome },
                            sequence,
                            identity,
                        );
                        self.revoke_generation(generation);
                    }
                }
            }
            WorkFamily::HostRequest => {
                let Some(Effect::HostRequest(request)) = self.work.take_pending_effect(generation)
                else {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return;
                };
                if self.work.mark_running(generation).is_none() {
                    return;
                }
                self.host_requests
                    .push(LiveHostRequest::new(generation, request));
                self.completion_ingress.register_host_response(generation);
                self.record_work_fact_from_envelope(
                    TraceRecordKind::WorkStartAccepted,
                    sequence,
                    identity.clone(),
                );
                self.record_work_fact_from_envelope(
                    TraceRecordKind::HostRequestExposed,
                    sequence,
                    identity,
                );
            }
        }
    }

    pub(crate) fn process_work_cancellation(
        &mut self,
        sequence: WorkSequence,
        generation: crate::work::WorkGeneration,
        identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) {
        if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        }
        self.trace.record_work(
            TraceRecordKind::WorkCleanupProcessed,
            Some(sequence),
            causal_parent,
            None,
            None,
            None,
            identity,
        );
        self.revoke_generation(generation);
    }

    pub(crate) fn process_mounted_subscription_reconcile(
        &mut self,
        sequence: WorkSequence,
        owner: &MountedNodeId,
        causal_parent: Option<TraceSequence>,
    ) {
        self.mounted_subscription_reconcile_pending
            .retain(|pending| pending != owner);
        if self.tree.target_status(owner) != TargetStatus::Live {
            if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
                self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
                return;
            }
            self.trace.record(
                TraceRecordKind::MountedSubscriptionReconciliationSuppressedStale,
                Some(sequence),
                causal_parent,
                None,
                None,
                Some(TraceTarget::new(owner.clone(), None)),
            );
            self.cancel_owner_work(&WorkOwner::Mounted(owner.clone()));
            return;
        }
        let mut subscriptions = SubscriptionSet::new();
        if self
            .tree
            .declare_subscriptions(owner, &mut subscriptions)
            .is_err()
        {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        }
        let declarations = subscriptions.__runtime_into_declarations();
        self.reconcile_subscriptions(
            &WorkOwner::Mounted(owner.clone()),
            declarations,
            Some(sequence),
            causal_parent,
        );
    }

    #[allow(clippy::too_many_lines)]
    fn reconcile_subscriptions(
        &mut self,
        owner: &WorkOwner,
        declarations: Vec<Subscription<Action>>,
        work_sequence: Option<WorkSequence>,
        transaction_parent: Option<TraceSequence>,
    ) {
        let SubscriptionDiff {
            invalidated,
            starts,
            duplicate_keys,
        } = self.derive_subscription_diff(owner, declarations);
        let invalidated_set: HashSet<_> = invalidated.iter().copied().collect();
        let Ok((generations, next_generation)) = self.work.preview_generations(starts.len()) else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        };
        let sequenced = invalidated.len().checked_add(starts.len());
        if sequenced.is_none_or(|count| self.queue.preflight_commit(count).is_err())
            || self
                .work
                .preflight_subscriptions(&invalidated_set, starts.len())
                .is_err()
        {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        }
        let required_trace_records = invalidated
            .len()
            .saturating_mul(2)
            .saturating_add(starts.len().saturating_mul(3))
            .saturating_add(1);
        if !self
            .trace
            .can_admit(MandatoryTracePlan::planned_scheduler_transaction(
                required_trace_records,
            ))
        {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        }

        self.record_subscription_duplicates(owner, &duplicate_keys);
        let cancelled_count = invalidated.len();
        let started_count = starts.len();
        let transaction_parent = self.trace.record(
            TraceRecordKind::SubscriptionDiffCommitted {
                started: started_count,
                cancelled: cancelled_count,
                duplicate_keys: duplicate_keys.len(),
            },
            work_sequence,
            transaction_parent,
            None,
            None,
            None,
        );
        let invalidated_identities: Vec<_> = invalidated
            .iter()
            .filter_map(|generation| self.trace_work_identity(*generation))
            .collect();
        self.work.commit_generation_reservation(next_generation);
        let cancellation_lineage =
            self.record_invalidation_facts(&invalidated_identities, transaction_parent);
        for generation in &invalidated {
            self.invalidate_generation_now(*generation);
        }
        let start_generations = generations.clone();
        for (generation, declaration) in generations.into_iter().zip(starts) {
            let key = declaration.key.clone();
            self.work
                .commit_subscription_record(generation, owner.clone(), key);
            self.subscriptions.push(LiveSubscription::new(
                generation,
                owner.clone(),
                declaration,
                self.wake.handle(),
            ));
            let identity = self
                .trace_work_identity(generation)
                .unwrap_or_else(|| unreachable!("committed subscription has trace identity"));
            self.record_work_fact_with_parent(
                TraceRecordKind::WorkRequested,
                transaction_parent,
                identity.clone(),
            );
            self.record_work_fact(TraceRecordKind::SubscriptionDeclared, identity.clone());
            self.record_work_fact(TraceRecordKind::WorkGenerationCommitted, identity);
        }
        for generation in invalidated {
            let (identity, parent) = cancellation_lineage
                .get(&generation.get())
                .cloned()
                .unwrap_or_else(|| unreachable!("cancelled subscription retains trace lineage"));
            self.queue
                .push_cancellation(generation, identity, parent)
                .unwrap_or_else(|_| unreachable!("subscription diff was preflighted"));
        }
        for generation in start_generations {
            self.queue
                .push_effect_start(generation)
                .unwrap_or_else(|_| unreachable!("subscription diff was preflighted"));
        }
    }

    fn derive_subscription_diff(
        &self,
        owner: &WorkOwner,
        declarations: Vec<Subscription<Action>>,
    ) -> SubscriptionDiff<Action> {
        let mut counts = HashMap::new();
        for declaration in &declarations {
            *counts.entry(declaration.key.clone()).or_insert(0usize) += 1;
        }
        let duplicate_keys: HashSet<_> = counts
            .into_iter()
            .filter_map(|(key, count)| (count > 1).then_some(key))
            .collect();
        let desired: HashMap<_, _> = declarations
            .iter()
            .filter(|declaration| !duplicate_keys.contains(&declaration.key))
            .map(|declaration| (declaration.key.clone(), declaration))
            .collect();
        let invalidated: Vec<_> = self
            .subscriptions
            .iter()
            .filter(|subscription| {
                if &subscription.owner != owner {
                    return false;
                }
                desired.get(&subscription.key).is_none_or(|declaration| {
                    subscription.source_type != declaration.source_type
                        || subscription.revision != declaration.revision
                })
            })
            .map(|subscription| subscription.generation)
            .collect();
        let starts: Vec<_> = declarations
            .into_iter()
            .filter(|declaration| !duplicate_keys.contains(&declaration.key))
            .filter(|declaration| {
                !self.subscriptions.iter().any(|subscription| {
                    &subscription.owner == owner
                        && subscription.key == declaration.key
                        && subscription.source_type == declaration.source_type
                        && subscription.revision == declaration.revision
                })
            })
            .collect();
        SubscriptionDiff {
            invalidated,
            starts,
            duplicate_keys,
        }
    }

    fn record_subscription_duplicates(
        &mut self,
        owner: &WorkOwner,
        duplicate_keys: &HashSet<runenui_core::WorkKey>,
    ) {
        let owner = match owner {
            WorkOwner::Application => SubscriptionOwnerKind::Application,
            WorkOwner::Mounted(_) => SubscriptionOwnerKind::Mounted,
        };
        let mut duplicate_keys: Vec<_> = duplicate_keys.iter().cloned().collect();
        duplicate_keys.sort_unstable();
        let limit = self.limits.subscription_diagnostics();
        if limit == 0 {
            return;
        }
        for key in duplicate_keys {
            if self.subscription_diagnostics.len() == limit {
                self.subscription_diagnostics.remove(0);
            }
            self.subscription_diagnostics
                .push(SubscriptionDiagnostic::DuplicateKey { owner, key });
        }
    }

    fn cancel_owner_work(&mut self, owner: &WorkOwner) {
        let generations = self.work.generations_for_owner(owner);
        for generation in generations {
            self.revoke_generation(generation);
        }
        if let WorkOwner::Mounted(owner) = owner {
            self.mounted_subscription_reconcile_pending
                .retain(|pending| pending != owner);
        }
    }

    pub(crate) fn queued_len(&self) -> usize {
        self.queue.len()
    }

    pub(crate) fn readiness_checkpoint(
        &mut self,
        max_completion_imports: usize,
        max_local_polls: usize,
        max_timer_promotions: usize,
    ) -> ReadinessCheckpointReport {
        let imported_completions = self.import_send_completions(max_completion_imports);
        let promoted_timers = self.promote_due_timers(max_timer_promotions);
        let polled_local_work = self.poll_local_work(max_local_polls);
        let report = ReadinessCheckpointReport {
            imported_completions,
            polled_local_work,
            promoted_timers,
        };
        self.record_optional(
            TraceRecordKind::ReadinessCheckpoint {
                imported_completions,
                polled_local_work,
                promoted_timers,
            },
            None,
            None,
            None,
        );
        report
    }

    #[allow(clippy::too_many_lines)]
    fn import_send_completions(&mut self, limit: usize) -> usize {
        let mut imported = 0;
        while imported < limit {
            let Some((generation, kind, identity, completion_parent)) =
                self.completion_ingress.front()
            else {
                break;
            };
            let family = match kind {
                CompletionKind::SendTask => WorkFamily::SendTask,
                CompletionKind::Subscription => WorkFamily::Subscription,
                CompletionKind::HostResponse => WorkFamily::HostRequest,
            };
            if self.work.is_running_family(generation, family) {
                let trace_plan = match family {
                    WorkFamily::HostRequest => MandatoryTracePlan::host_completion(),
                    WorkFamily::SendTask | WorkFamily::Subscription => {
                        MandatoryTracePlan::send_completion()
                    }
                    WorkFamily::LocalTask | WorkFamily::Timer => unreachable!(),
                };
                if !self.callback_output_preflight(Some((generation, family)), trace_plan) {
                    break;
                }
            } else if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
                self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
                break;
            }
            let Some(completion) = self.completion_ingress.pop() else {
                unreachable!("completion ingress front remains owned until UI-thread pop")
            };
            imported += 1;
            if !self.work.is_running(completion.generation) {
                let stale_parent = self
                    .trace
                    .records()
                    .filter(|record| record.work() == Some(&completion.trace_identity))
                    .map(crate::TraceRecord::sequence)
                    .last()
                    .or(completion_parent);
                self.record_work_fact_with_parent(
                    TraceRecordKind::WorkCompletionRejectedStale,
                    stale_parent,
                    completion.trace_identity,
                );
                if completion.kind == CompletionKind::HostResponse {
                    self.completion_ingress
                        .release_host_response(completion.generation);
                }
                continue;
            }
            self.record_work_fact(TraceRecordKind::WorkCompletionImported, identity.clone());
            if completion.kind == CompletionKind::Subscription {
                let Some(subscription) = self.subscriptions.iter_mut().find(|subscription| {
                    subscription.generation == completion.generation && subscription.started
                }) else {
                    continue;
                };
                let LiveSubscriptionSource::Send { map, .. } = &mut subscription.source else {
                    continue;
                };
                let action = map(completion.output);
                let mapped = self.record_work_fact(TraceRecordKind::WorkCompletionMapped, identity);
                self.queue_callback_action(action, mapped);
                continue;
            }
            if completion.kind == CompletionKind::HostResponse {
                let Ok(response) = completion.output.downcast::<Protocol::Response>() else {
                    self.completion_ingress
                        .release_host_response(completion.generation);
                    continue;
                };
                let response = *response;
                let Some(request) = self.host_requests.iter().find(|request| {
                    request.generation == completion.generation
                        && self
                            .work
                            .is_running_family(request.generation, WorkFamily::HostRequest)
                }) else {
                    self.completion_ingress
                        .release_host_response(completion.generation);
                    continue;
                };
                if request.expected != Protocol::response_kind(&response) {
                    self.completion_ingress
                        .release_host_response(completion.generation);
                    continue;
                }
                self.record_work_fact(TraceRecordKind::HostResponseAccepted, identity.clone());
                let Some(request_index) = self
                    .host_requests
                    .iter()
                    .position(|request| request.generation == completion.generation)
                else {
                    self.completion_ingress
                        .release_host_response(completion.generation);
                    continue;
                };
                let request = self.host_requests.remove(request_index);
                let action = (request.map)(response);
                let mapped = self.record_work_fact(TraceRecordKind::WorkCompletionMapped, identity);
                self.revoke_generation(completion.generation);
                self.queue_callback_action(action, mapped);
                continue;
            }
            let Some(index) = self
                .send_task_mappers
                .iter()
                .position(|mapper| mapper.generation == completion.generation)
            else {
                continue;
            };
            let mapper = self.send_task_mappers.remove(index);
            let action = (mapper.map)(completion.output);
            let mapping_trace =
                self.record_work_fact(TraceRecordKind::WorkCompletionMapped, identity);
            self.revoke_generation(completion.generation);
            self.queue_callback_action(action, mapping_trace);
        }
        imported
    }

    #[allow(clippy::too_many_lines)]
    fn poll_local_work(&mut self, limit: usize) -> usize {
        self.local_tasks
            .retain(|task| self.work.is_running(task.generation));
        self.subscriptions.retain(|subscription| {
            self.work
                .is_live_family(subscription.generation, WorkFamily::Subscription)
        });

        let mut visited = HashSet::new();
        let mut polled = 0;
        while polled < limit && !self.queue.is_full() {
            let next_task = self
                .local_tasks
                .iter()
                .filter(|task| task.is_eligible() && !visited.contains(&task.generation))
                .map(|task| task.generation)
                .min();
            let next_subscription = self
                .subscriptions
                .iter()
                .filter(|subscription| {
                    subscription.is_local_eligible() && !visited.contains(&subscription.generation)
                })
                .map(|subscription| subscription.generation)
                .min();
            let next = match (next_task, next_subscription) {
                (Some(task), Some(subscription)) => Some(task.min(subscription)),
                (Some(task), None) => Some(task),
                (None, Some(subscription)) => Some(subscription),
                (None, None) => None,
            };
            let Some(generation) = next else {
                break;
            };
            visited.insert(generation);

            let family = if self
                .local_tasks
                .iter()
                .any(|task| task.generation == generation)
            {
                WorkFamily::LocalTask
            } else {
                WorkFamily::Subscription
            };
            if !self.callback_output_preflight(
                Some((generation, family)),
                MandatoryTracePlan::callback_with_action(),
            ) {
                break;
            }
            let identity = self
                .trace_work_identity(generation)
                .unwrap_or_else(|| unreachable!("live local work has trace identity"));
            self.record_work_fact(TraceRecordKind::LocalWorkPolled, identity.clone());

            if let Some(task_index) = self
                .local_tasks
                .iter()
                .position(|task| task.generation == generation)
            {
                let ready = {
                    let task = &mut self.local_tasks[task_index];
                    task.poll_once().then(|| task.take_ready())
                };
                let Some(ready) = ready else {
                    continue;
                };
                polled += 1;
                match ready {
                    TaskReady::Complete(Some(action)) => {
                        let ready =
                            self.record_work_fact(TraceRecordKind::LocalWorkReady, identity);
                        self.revoke_generation(generation);
                        self.queue_callback_action(action, ready);
                    }
                    TaskReady::Complete(None) => {
                        self.record_work_fact(TraceRecordKind::LocalWorkReady, identity);
                        self.revoke_generation(generation);
                    }
                    TaskReady::NotReady => {}
                }
                continue;
            }

            let poll = self
                .subscriptions
                .iter_mut()
                .find(|subscription| subscription.generation == generation)
                .map_or(
                    SubscriptionPoll::NotEligible,
                    LiveSubscription::poll_local_once,
                );
            match poll {
                SubscriptionPoll::Item(action) => {
                    polled += 1;
                    let ready = self.record_work_fact(TraceRecordKind::LocalWorkReady, identity);
                    self.queue_callback_action(action, ready);
                }
                SubscriptionPoll::Closed => {
                    polled += 1;
                    self.record_work_fact(TraceRecordKind::LocalWorkReady, identity);
                    self.revoke_generation(generation);
                }
                SubscriptionPoll::Pending => polled += 1,
                SubscriptionPoll::NotEligible => {}
            }
        }
        self.local_tasks
            .retain(|task| self.work.is_running(task.generation));
        self.subscriptions.retain(|subscription| {
            self.work
                .is_live_family(subscription.generation, WorkFamily::Subscription)
        });
        polled
    }

    fn promote_due_timers(&mut self, limit: usize) -> usize {
        let now = self.now();
        let mut due: Vec<_> = self
            .timers
            .iter()
            .filter(|timer| self.work.is_running(timer.generation) && timer.is_due(now))
            .map(|timer| (timer.deadline, timer.generation))
            .collect();
        due.sort_unstable();
        let mut promoted = 0;
        for (_deadline, generation) in due {
            if promoted >= limit || self.queue.is_full() {
                break;
            }
            if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
                self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
                return promoted;
            }
            let Some(identity) = self.trace_work_identity(generation) else {
                continue;
            };
            if self.queue.push_timer_firing(generation).is_err() {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                return promoted;
            }
            if let Some(timer) = self
                .timers
                .iter_mut()
                .find(|timer| timer.generation == generation)
            {
                timer.mark_promoted();
                promoted += 1;
            }
            self.record_work_fact(TraceRecordKind::TimerPromoted, identity);
        }
        self.timers
            .retain(|timer| self.work.is_running(timer.generation));
        promoted
    }

    pub(crate) fn scheduler_observation(&self) -> SchedulerObservation {
        let now = self.now();
        SchedulerObservation {
            completion_imports_pending: self.completion_ingress.len() > 0,
            due_timers_pending: self
                .timers
                .iter()
                .any(|timer| self.work.is_running(timer.generation) && timer.is_due(now)),
            local_polls_pending: self
                .local_tasks
                .iter()
                .any(|task| self.work.is_running(task.generation) && task.is_eligible())
                || self.subscriptions.iter().any(|subscription| {
                    self.work
                        .is_running_family(subscription.generation, WorkFamily::Subscription)
                        && subscription.is_local_eligible()
                }),
            mandatory_derived_work_pending: !self.mounted_subscription_reconcile_pending.is_empty(),
            next_deadline: self
                .timers
                .iter()
                .filter(|timer| self.work.is_running(timer.generation))
                .map(|timer| timer.deadline)
                .min(),
            publication_dirty: self.redraw_revision > self.redraw_acknowledged,
        }
    }

    pub(crate) fn process_timer_firing(
        &mut self,
        sequence: WorkSequence,
        generation: crate::work::WorkGeneration,
    ) {
        if !self.callback_output_preflight(
            Some((generation, WorkFamily::Timer)),
            MandatoryTracePlan::callback_with_action(),
        ) {
            return;
        }
        let Some(identity) = self.trace_work_identity(generation) else {
            return;
        };
        let firing_parent = self.record_work_fact_from_envelope(
            TraceRecordKind::TimerFired,
            sequence,
            identity.clone(),
        );
        let now = self.now();
        let Some(index) = self
            .timers
            .iter()
            .position(|timer| timer.generation == generation)
        else {
            return;
        };
        let (action, outcome) = self.timers[index].fire(now);
        self.last_timer_firing_outcome = Some(match outcome {
            TimerFireOutcome::Completed => TimerFiringOutcome::Completed,
            TimerFireOutcome::Rescheduled => TimerFiringOutcome::Rescheduled,
            TimerFireOutcome::RepeatDeadlineOverflow => TimerFiringOutcome::RepeatDeadlineOverflow,
        });
        let mut action_parent = firing_parent;
        if outcome != TimerFireOutcome::Rescheduled {
            let outcome = match outcome {
                TimerFireOutcome::Completed => TraceTimerTerminalOutcome::Completed,
                TimerFireOutcome::RepeatDeadlineOverflow => {
                    TraceTimerTerminalOutcome::RepeatDeadlineOverflow
                }
                TimerFireOutcome::Rescheduled => unreachable!(),
            };
            action_parent = self.record_work_fact_from_envelope(
                TraceRecordKind::TimerTerminated { outcome },
                sequence,
                identity,
            );
            self.timers.remove(index);
            self.revoke_generation(generation);
        }
        if self.queue.is_full() {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 1);
            return;
        }
        self.queue_callback_action(action, action_parent);
    }

    pub(crate) fn pending_host_requests(&self) -> Vec<HostRequestRef<'_, Protocol>> {
        self.host_requests
            .iter()
            .filter(|request| {
                self.work
                    .is_running_family(request.generation, WorkFamily::HostRequest)
            })
            .map(|request| HostRequestRef {
                token: HostRequestToken {
                    namespace: Arc::clone(&self.host_namespace),
                    generation: request.generation,
                },
                command: &request.command,
            })
            .collect()
    }

    pub(crate) fn complete_host_request(
        &mut self,
        token: &HostRequestToken,
        response: Protocol::Response,
    ) -> Result<WorkSequence, HostResponseError<Protocol::Response>> {
        match self.status {
            RuntimeStatus::Closed => return Err(HostResponseError::Closed(response)),
            RuntimeStatus::Terminal(reason) => {
                return Err(HostResponseError::Terminal { response, reason });
            }
            RuntimeStatus::Running => {}
        }
        if !Arc::ptr_eq(&self.host_namespace, &token.namespace) {
            return Err(HostResponseError::ForeignRuntime(response));
        }
        let Some(request) = self.host_requests.iter().find(|request| {
            request.generation == token.generation
                && self
                    .work
                    .is_running_family(request.generation, WorkFamily::HostRequest)
        }) else {
            return Err(HostResponseError::Stale(response));
        };
        if request.expected != Protocol::response_kind(&response) {
            let identity = self
                .trace_work_identity(token.generation)
                .unwrap_or_else(|| unreachable!("live host request has trace identity"));
            if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
                let reason = RuntimeTerminalReason::TraceSequenceExhausted;
                self.enter_terminal(reason, 0);
                return Err(HostResponseError::Terminal { response, reason });
            }
            self.record_work_fact(TraceRecordKind::HostResponseRejected, identity);
            return Err(HostResponseError::MismatchedKind(response));
        }
        if !self.callback_output_preflight(
            Some((token.generation, WorkFamily::HostRequest)),
            MandatoryTracePlan::callback_with_action(),
        ) {
            if matches!(self.status, RuntimeStatus::Running) {
                return Err(HostResponseError::Full(response));
            }
            let RuntimeStatus::Terminal(reason) = self.status else {
                unreachable!("callback preflight only closes through a terminal transition")
            };
            return Err(HostResponseError::Terminal { response, reason });
        }
        let Some(request_index) = self
            .host_requests
            .iter()
            .position(|request| request.generation == token.generation)
        else {
            return Err(HostResponseError::Stale(response));
        };
        if !self
            .completion_ingress
            .claim_direct_host_response(token.generation)
        {
            return Err(HostResponseError::Stale(response));
        }
        let identity = self
            .trace_work_identity(token.generation)
            .unwrap_or_else(|| unreachable!("live host request has trace identity"));
        self.record_work_fact(TraceRecordKind::HostResponseAccepted, identity.clone());
        let request = self.host_requests.remove(request_index);
        let action = (request.map)(response);
        let mapped = self.record_work_fact(TraceRecordKind::WorkCompletionMapped, identity);
        self.revoke_generation(token.generation);
        let Some(sequence) = self.queue_callback_action(action, mapped) else {
            unreachable!("host callback output was preflighted")
        };
        self.external_queue_commit_accepted();
        Ok(sequence)
    }

    pub(crate) fn host_response_completion(
        &mut self,
        token: &HostRequestToken,
        response: Protocol::Response,
    ) -> Result<HostResponseCompletion, HostResponseError<Protocol::Response>>
    where
        Protocol::Response: Send + 'static,
    {
        match self.status {
            RuntimeStatus::Closed => return Err(HostResponseError::Closed(response)),
            RuntimeStatus::Terminal(reason) => {
                return Err(HostResponseError::Terminal { response, reason });
            }
            RuntimeStatus::Running => {}
        }
        if !Arc::ptr_eq(&self.host_namespace, &token.namespace) {
            return Err(HostResponseError::ForeignRuntime(response));
        }
        if !self
            .work
            .is_running_family(token.generation, WorkFamily::HostRequest)
        {
            return Err(HostResponseError::Stale(response));
        }
        if !self
            .completion_ingress
            .host_response_is_open(token.generation)
        {
            return Err(HostResponseError::Stale(response));
        }
        let Some(request) = self
            .host_requests
            .iter()
            .find(|request| request.generation == token.generation)
        else {
            return Err(HostResponseError::Stale(response));
        };
        if request.expected != Protocol::response_kind(&response) {
            let identity = self
                .trace_work_identity(token.generation)
                .unwrap_or_else(|| unreachable!("live host request has trace identity"));
            if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
                let reason = RuntimeTerminalReason::TraceSequenceExhausted;
                self.enter_terminal(reason, 0);
                return Err(HostResponseError::Terminal { response, reason });
            }
            self.record_work_fact(TraceRecordKind::HostResponseRejected, identity);
            return Err(HostResponseError::MismatchedKind(response));
        }
        Ok(HostResponseCompletion::new(
            token.generation,
            Box::new(response),
            self.completion_ingress.sender(),
            self.trace_work_identity(token.generation)
                .unwrap_or_else(|| unreachable!("live host request has trace identity")),
            self.work.trace_parent(token.generation),
        ))
    }

    pub(crate) fn cancel_host_request(
        &mut self,
        token: &HostRequestToken,
    ) -> Result<WorkSequence, HostRequestCancelError> {
        match self.status {
            RuntimeStatus::Closed => return Err(HostRequestCancelError::Closed),
            RuntimeStatus::Terminal(reason) => {
                return Err(HostRequestCancelError::Terminal(reason));
            }
            RuntimeStatus::Running => {}
        }
        if !Arc::ptr_eq(&self.host_namespace, &token.namespace) {
            return Err(HostRequestCancelError::ForeignRuntime);
        }
        if !self
            .work
            .is_running_family(token.generation, WorkFamily::HostRequest)
        {
            return Err(HostRequestCancelError::Stale);
        }
        match self.queue.preflight_commit(1) {
            Ok(()) => {}
            Err(QueueCommitError::Full) => return Err(HostRequestCancelError::Full),
            Err(QueueCommitError::SequenceExhausted) => {
                let reason = RuntimeTerminalReason::WorkSequenceExhausted;
                self.enter_terminal(reason, 0);
                return Err(HostRequestCancelError::Terminal(reason));
            }
        }
        if !self
            .trace
            .can_admit(MandatoryTracePlan::work_cancellation())
        {
            let reason = RuntimeTerminalReason::TraceSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(HostRequestCancelError::Terminal(reason));
        }
        let identity = self
            .trace_work_identity(token.generation)
            .unwrap_or_else(|| unreachable!("live host request has trace identity"));
        let lineage = self.record_invalidation_facts(
            core::slice::from_ref(&identity),
            self.work.trace_parent(token.generation),
        );
        let (_, parent) = lineage
            .get(&token.generation.get())
            .cloned()
            .unwrap_or_else(|| unreachable!("host cancellation retains trace lineage"));
        let sequence = self
            .queue
            .push_cancellation(token.generation, identity, parent)
            .unwrap_or_else(|_| unreachable!("host cancellation was preflighted"));
        self.invalidate_generation_now(token.generation);
        self.external_queue_commit_accepted();
        Ok(sequence)
    }

    pub(crate) fn subscription_diagnostics(&self) -> &[SubscriptionDiagnostic] {
        &self.subscription_diagnostics
    }

    pub(crate) fn request_redraw(&mut self) {
        let Some(next) = self.redraw_revision.checked_add(1) else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        };
        self.redraw_revision = next;
        self.record_optional(
            TraceRecordKind::RedrawRequested { revision: next },
            None,
            None,
            None,
        );
    }

    pub(crate) fn take_redraw_request(&mut self) -> Option<crate::RedrawRequest> {
        let request =
            (self.redraw_revision > self.redraw_acknowledged).then(|| crate::RedrawRequest {
                namespace: Arc::clone(&self.redraw_namespace),
                revision: self.redraw_revision,
            });
        if let Some(request) = &request {
            self.record_optional(
                TraceRecordKind::RedrawTaken {
                    revision: request.revision,
                },
                None,
                None,
                None,
            );
        }
        request
    }

    pub(crate) fn acknowledge_redraw(
        &mut self,
        request: &crate::RedrawRequest,
    ) -> Result<(), crate::RedrawAcknowledgeError> {
        if !Arc::ptr_eq(&self.redraw_namespace, &request.namespace) {
            return Err(crate::RedrawAcknowledgeError::ForeignRuntime);
        }
        if request.revision > self.redraw_revision {
            return Err(crate::RedrawAcknowledgeError::FutureRevision);
        }
        self.redraw_acknowledged = self.redraw_acknowledged.max(request.revision);
        self.record_optional(
            TraceRecordKind::RedrawAcknowledged {
                revision: request.revision,
            },
            None,
            None,
            None,
        );
        Ok(())
    }

    pub(crate) fn advance_time(
        &self,
        duration: std::time::Duration,
    ) -> Result<MonotonicInstant, crate::MonotonicTimeError> {
        let now = self.clock.advance(duration)?;
        if self
            .timers
            .iter()
            .any(|timer| self.work.is_running(timer.generation) && timer.is_due(now))
        {
            let _ = self.wake.handle().request();
        }
        Ok(now)
    }

    pub(crate) fn set_send_task_executor(&mut self, executor: impl SendTaskExecutor + 'static) {
        self.send_executor = Box::new(executor);
    }

    pub(crate) const fn last_send_task_start_outcome(&self) -> Option<SendTaskStartOutcome> {
        self.last_send_task_start_outcome
    }

    pub(crate) const fn last_timer_start_outcome(&self) -> Option<TimerStartOutcome> {
        self.last_timer_start_outcome
    }

    pub(crate) const fn last_timer_firing_outcome(&self) -> Option<TimerFiringOutcome> {
        self.last_timer_firing_outcome
    }

    pub(crate) fn set_monotonic_clock(&mut self, clock: impl MonotonicClock + 'static) {
        self.host_clock = Some(Box::new(clock));
    }

    pub(crate) fn set_wake_transport(&self, transport: impl crate::WakeTransport + 'static) {
        self.wake.set_transport(transport);
    }

    pub(crate) fn acknowledge_wake(&mut self) {
        self.wake.acknowledge();
        // Completion import owns an exact mandatory trace plan. An optional wake
        // acknowledgement must not consume one of that operation's sequences.
        if self.completion_ingress.len() == 0 && self.queue.is_empty() {
            self.record_optional(TraceRecordKind::WakeAcknowledged, None, None, None);
        }
    }

    pub(crate) fn rearm_wake_if_needed(&mut self) {
        let observation = self.scheduler_observation();
        let serviceable = !self.queue.is_empty()
            || observation.completion_imports_pending
            || observation.due_timers_pending
            || observation.local_polls_pending
            || observation.mandatory_derived_work_pending;
        if !serviceable {
            return;
        }
        if matches!(
            self.wake.handle().request(),
            crate::WakeRequestOutcome::Requested
        ) {
            self.record_optional(TraceRecordKind::WakeRequested, None, None, None);
        }
    }

    fn now(&self) -> MonotonicInstant {
        self.host_clock
            .as_ref()
            .map_or_else(|| self.clock.now(), |clock| clock.now())
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
        let (cancelled_queued, cancelled_live) = self.close_scheduling_authority();
        let cancelled = cancelled_queued
            .saturating_add(cancelled_live.total())
            .saturating_add(additional_cancelled);
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

    fn callback_output_preflight(
        &mut self,
        work: Option<(crate::work::WorkGeneration, WorkFamily)>,
        trace_plan: MandatoryTracePlan,
    ) -> bool {
        if !matches!(self.status, RuntimeStatus::Running) {
            return false;
        }
        if let Some((generation, family)) = work
            && !self.work.is_running_family(generation, family)
        {
            return false;
        }
        if self.queue.is_full() {
            return false;
        }
        if !self.queue.has_sequence() {
            self.enter_terminal(RuntimeTerminalReason::WorkSequenceExhausted, 0);
            return false;
        }
        if !self.trace.can_admit(trace_plan) {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return false;
        }
        true
    }

    fn queue_callback_action(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
    ) -> Option<WorkSequence> {
        self.commit_preflighted_action(
            action,
            causal_parent,
            None,
            ApplicationActionOrigin::ApplicationEffect,
        )
        .map_or_else(
            |_| {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                None
            },
            Some,
        )
    }

    fn trace_work_identity(
        &self,
        generation: crate::work::WorkGeneration,
    ) -> Option<TraceWorkIdentity> {
        self.work
            .trace_identity(generation)
            .map(public_trace_work_identity)
    }

    fn record_work_fact(
        &mut self,
        kind: TraceRecordKind,
        identity: TraceWorkIdentity,
    ) -> Option<TraceSequence> {
        let generation = self.work.generation_with_value(identity.generation());
        let causal_parent = generation.and_then(|generation| self.work.trace_parent(generation));
        let trace = self
            .trace
            .record_work(kind, None, causal_parent, None, None, None, identity);
        if let (Some(generation), Some(trace)) = (generation, trace) {
            self.work.set_trace(generation, trace);
        }
        trace
    }

    fn record_work_fact_from_envelope(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: WorkSequence,
        identity: TraceWorkIdentity,
    ) -> Option<TraceSequence> {
        let generation = self.work.generation_with_value(identity.generation());
        let causal_parent = generation.and_then(|generation| self.work.trace_parent(generation));
        let trace = self.trace.record_work(
            kind,
            Some(work_sequence),
            causal_parent,
            None,
            None,
            None,
            identity,
        );
        if let (Some(generation), Some(trace)) = (generation, trace) {
            self.work.set_trace(generation, trace);
        }
        trace
    }

    fn record_work_fact_with_parent(
        &mut self,
        kind: TraceRecordKind,
        causal_parent: Option<TraceSequence>,
        identity: TraceWorkIdentity,
    ) -> Option<TraceSequence> {
        let generation = self.work.generation_with_value(identity.generation());
        let trace = self
            .trace
            .record_work(kind, None, causal_parent, None, None, None, identity);
        if let (Some(generation), Some(trace)) = (generation, trace) {
            self.work.set_trace(generation, trace);
        }
        trace
    }

    fn record_invalidation_facts(
        &mut self,
        identities: &[TraceWorkIdentity],
        transaction_parent: Option<TraceSequence>,
    ) -> HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)> {
        identities
            .iter()
            .map(|identity| {
                let bound = self.record_work_fact_with_parent(
                    TraceRecordKind::WorkCancellationBound,
                    transaction_parent,
                    identity.clone(),
                );
                let invalidated = self.record_work_fact_with_parent(
                    TraceRecordKind::WorkLogicallyInvalidated,
                    bound,
                    identity.clone(),
                );
                (identity.generation(), (identity.clone(), invalidated))
            })
            .collect()
    }

    fn close_scheduling_authority(&mut self) -> (usize, WorkCancellationCounts) {
        self.completion_ingress.close();
        self.wake.close();
        let cancelled_queue = self.queue.cancel_all();
        self.trace
            .release_reservations(cancelled_queue.command_trace_reservations);
        let cancelled_queued = cancelled_queue.envelopes;
        let cancelled_live = self.work.cancel_all_counts();
        self.local_tasks.clear();
        self.timers.clear();
        self.subscriptions.clear();
        self.send_task_mappers.clear();
        self.host_requests.clear();
        self.mounted_subscription_reconcile_pending.clear();
        self.initial_mounted_subscription_owners.clear();
        self.initial_mounted_outputs.clear();
        (cancelled_queued, cancelled_live)
    }

    pub(crate) fn record_optional(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: Option<WorkSequence>,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
    ) {
        if self.trace.can_admit(MandatoryTracePlan::one_fact()) {
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

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn live_work_record_count_for_test(&self) -> usize {
        self.work.live_record_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn host_response_slot_count_for_test(&self) -> usize {
        self.completion_ingress.host_response_slot_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn send_task_slot_count_for_test(&self) -> usize {
        self.completion_ingress.send_task_slot_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn subscription_slot_count_for_test(&self) -> usize {
        self.completion_ingress.subscription_slot_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn completion_payload_count_for_test(&self) -> usize {
        self.completion_ingress.payload_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn send_task_mapper_count_for_test(&self) -> usize {
        self.send_task_mappers.len()
    }

    pub(crate) fn shutdown(&mut self) -> ShutdownReport {
        if matches!(self.status, RuntimeStatus::Closed) {
            return ShutdownReport {
                already_complete: true,
                cancelled_queued_envelopes: 0,
                unmounted_lifetimes: 0,
                cancelled_live_work: WorkCancellationCounts::default(),
            };
        }
        let (cancelled_queued_envelopes, cancelled_live_work) = self.close_scheduling_authority();
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
            cancelled_live_work,
        }
    }

    pub(crate) fn into_state(mut self) -> State {
        self.shutdown();
        self.state
            .take()
            .unwrap_or_else(|| unreachable!("state is returned exactly once"))
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_generation_for_test(&mut self, generation: u64) {
        self.generation = generation;
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_work_sequence_for_test(&mut self, next: u64) {
        self.queue.seed_next_sequence_for_test(next);
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_work_generation_for_test(&mut self, next: u64) {
        self.work.seed_next_generation_for_test(next);
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_trace_sequence_for_test(&mut self, next: u64) {
        self.trace.seed_next_sequence_for_test(next);
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn routed_sequence_state_for_test(&self) -> (Option<u64>, Option<u64>) {
        (
            self.queue.next_sequence().map(WorkSequence::get),
            self.trace.next_sequence_for_test(),
        )
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn routed_trace_reservations_for_test(&self) -> usize {
        self.trace.reserved_records_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn fail_routed_callback_bridge_for_test(&mut self) {
        self.routed_callback_bridge_failure_for_test = true;
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn fail_routed_semantic_default_for_test(&mut self) {
        self.routed_semantic_default_failure_for_test = true;
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn fail_routed_commit_for_test(&mut self) {
        self.routed_commit_failure_for_test = true;
    }

    fn take_initial_mounted_ledgers(
        &mut self,
        initial_output_count: usize,
    ) -> Option<Vec<OwnedTransactionLedger<Action, Protocol>>> {
        let initial_mounted_outputs = core::mem::take(&mut self.initial_mounted_outputs);
        let mut total_outputs = initial_output_count;
        let mut mounted = Vec::with_capacity(initial_mounted_outputs.len());
        for (owner, outputs) in initial_mounted_outputs {
            total_outputs = total_outputs.checked_add(outputs.len())?;
            if total_outputs > self.limits.transaction_outputs() {
                return None;
            }
            let effects = outputs
                .into_iter()
                .map(mounted_effect_into_effect)
                .collect();
            let ledger =
                TransactionLedger::from_outputs(effects, self.limits.transaction_outputs())
                    .unwrap_or_else(|_| {
                        unreachable!("complete initial transaction allowance was checked")
                    });
            mounted.push(OwnedTransactionLedger {
                owner: WorkOwner::Mounted(owner),
                ledger,
            });
        }
        Some(mounted)
    }

    pub(crate) fn initialize_application_work<App>(&mut self)
    where
        App: UiApp<State = State, Action = Action, HostProtocol = Protocol>,
    {
        if !matches!(self.status, RuntimeStatus::Running) {
            return;
        }
        let effects = App::initial_effects(self.state());
        let Ok(ledger) =
            TransactionLedger::collect(effects.into_effects(), self.limits.transaction_outputs())
        else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        };
        let initial_output_count = ledger.len();
        let mut subscriptions = SubscriptionSet::new();
        App::subscriptions(self.state(), &mut subscriptions);
        let SubscriptionDiff {
            invalidated,
            starts,
            duplicate_keys,
        } = self.derive_subscription_diff(
            &WorkOwner::Application,
            subscriptions.__runtime_into_declarations(),
        );
        let cancelled = invalidated.len();
        let mounted_subscription_dirty =
            core::mem::take(&mut self.initial_mounted_subscription_owners);
        let Some(mounted) = self.take_initial_mounted_ledgers(initial_output_count) else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        };
        let input = ApplicationTransactionInput {
            lifecycle_invalidated: Vec::new(),
            mounted_subscription_dirty,
            application: ledger,
            application_subscription_invalidated: invalidated,
            application_subscription_starts: starts,
            mounted,
        };
        let plan = match PlannedApplicationTransaction::plan(input, &self.work, &self.queue) {
            Ok(plan) => plan,
            Err(error) => {
                let reason = match error {
                    TransactionPlanError::WorkSequenceExhausted => {
                        RuntimeTerminalReason::WorkSequenceExhausted
                    }
                    TransactionPlanError::WorkGenerationExhausted => {
                        RuntimeTerminalReason::WorkGenerationExhausted
                    }
                    TransactionPlanError::QueueFull | TransactionPlanError::RegistryFull => {
                        RuntimeTerminalReason::Poisoned
                    }
                };
                self.enter_terminal(reason, 0);
                return;
            }
        };
        let Some(required_trace_records) =
            required_application_transaction_trace_records(&plan).checked_add(1)
        else {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        };
        if !self
            .trace
            .can_admit(MandatoryTracePlan::planned_scheduler_transaction(
                required_trace_records,
            ))
        {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        }
        let transaction_parent = self.trace.record(
            TraceRecordKind::InitialApplicationTransactionStarted,
            None,
            None,
            None,
            None,
            None,
        );
        if self
            .commit_planned_application_transaction(
                plan,
                &duplicate_keys,
                cancelled,
                transaction_parent,
                HashMap::new(),
            )
            .is_err()
        {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        }
        self.record_optional(
            TraceRecordKind::InitialEffectsCommitted {
                count: initial_output_count,
            },
            None,
            None,
            None,
        );
    }

    fn plan_and_commit_application_transaction(
        &mut self,
        input: ApplicationTransactionInput<Action, Protocol>,
        application_subscription_duplicates: &HashSet<runenui_core::WorkKey>,
        application_subscription_cancelled: usize,
        transaction_parent: Option<TraceSequence>,
        pre_recorded_cancellation_lineage: HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)>,
    ) -> Result<(), CommitError> {
        let plan = PlannedApplicationTransaction::plan(input, &self.work, &self.queue)
            .map_err(|_| CommitError::Registry)?;
        self.commit_planned_application_transaction(
            plan,
            application_subscription_duplicates,
            application_subscription_cancelled,
            transaction_parent,
            pre_recorded_cancellation_lineage,
        )
    }

    fn commit_planned_application_transaction(
        &mut self,
        plan: PlannedApplicationTransaction<Action, Protocol>,
        application_subscription_duplicates: &HashSet<runenui_core::WorkKey>,
        application_subscription_cancelled: usize,
        transaction_parent: Option<TraceSequence>,
        pre_recorded_cancellation_lineage: HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)>,
    ) -> Result<(), CommitError> {
        let PlannedApplicationTransaction {
            invalidated,
            starts,
            application_outputs,
            application_subscription_starts,
            mounted_outputs,
            mounted_subscription_dirty,
            next_generation,
            semantic_events,
        } = plan;
        let required_trace_records = required_application_transaction_trace_records_from_parts(
            &invalidated,
            &starts,
            &application_outputs,
            &mounted_outputs,
        )
        .ok_or(CommitError::Registry)?;
        if !self
            .trace
            .can_admit(MandatoryTracePlan::planned_scheduler_transaction(
                required_trace_records,
            ))
        {
            return Err(CommitError::Registry);
        }
        let cancellation_lineage = self.commit_application_starts(
            &invalidated,
            starts,
            next_generation,
            semantic_events,
            transaction_parent,
            pre_recorded_cancellation_lineage,
        );
        self.append_cancellation_envelopes(&invalidated, &cancellation_lineage);
        for owner in mounted_subscription_dirty {
            self.queue
                .push_mounted_subscription_reconcile(owner.clone(), transaction_parent)
                .unwrap_or_else(|_| unreachable!("application transaction was preflighted"));
            self.mounted_subscription_reconcile_pending.push(owner);
        }
        self.append_planned_outputs(application_outputs, transaction_parent)?;
        let application_subscription_started = application_subscription_starts.len();
        for generation in application_subscription_starts {
            self.queue
                .push_effect_start(generation)
                .unwrap_or_else(|_| unreachable!("application transaction was preflighted"));
        }
        self.append_planned_outputs(mounted_outputs, transaction_parent)?;
        self.record_subscription_duplicates(
            &WorkOwner::Application,
            application_subscription_duplicates,
        );
        self.trace.record(
            TraceRecordKind::SubscriptionDiffCommitted {
                started: application_subscription_started,
                cancelled: application_subscription_cancelled,
                duplicate_keys: application_subscription_duplicates.len(),
            },
            None,
            transaction_parent,
            None,
            None,
            None,
        );
        Ok(())
    }

    fn commit_application_starts(
        &mut self,
        invalidated: &[crate::work::WorkGeneration],
        starts: Vec<crate::transaction::PlannedOwnedStart<Action, Protocol>>,
        next_generation: Option<core::num::NonZeroU64>,
        semantic_events: Vec<PlannedWorkSemanticEvent>,
        transaction_parent: Option<TraceSequence>,
        pre_recorded_lineage: HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)>,
    ) -> HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)> {
        let invalidated_set: HashSet<_> = invalidated.iter().copied().collect();
        let mut identities: HashMap<_, _> = invalidated
            .iter()
            .filter_map(|generation| {
                self.trace_work_identity(*generation)
                    .map(|identity| (generation.get(), identity))
            })
            .collect();
        identities.extend(starts.iter().map(|start| {
            (
                start.generation.get(),
                TraceWorkIdentity::new(
                    trace_work_owner(&start.owner),
                    trace_work_family(start.family),
                    start.generation.get(),
                    start.key.clone(),
                ),
            )
        }));
        self.work.commit_generation_reservation(next_generation);
        for start in starts {
            if !invalidated_set.contains(&start.generation) {
                self.commit_application_start(start);
            }
        }
        let mut semantic_parents: HashMap<_, _> = invalidated
            .iter()
            .map(|generation| (generation.get(), self.work.trace_parent(*generation)))
            .collect();
        let mut lineage = pre_recorded_lineage;
        for event in semantic_events {
            let generation = match event {
                PlannedWorkSemanticEvent::Requested(generation)
                | PlannedWorkSemanticEvent::Invalidated(generation) => generation,
            };
            let identity = identities
                .get(&generation.get())
                .cloned()
                .unwrap_or_else(|| unreachable!("planned semantic event has trace identity"));
            match event {
                PlannedWorkSemanticEvent::Requested(_) => {
                    let requested = self.record_work_fact_with_parent(
                        TraceRecordKind::WorkRequested,
                        transaction_parent,
                        identity.clone(),
                    );
                    let committed = if identity.family() == crate::TraceWorkFamily::Subscription {
                        let declared = self.record_work_fact_with_parent(
                            TraceRecordKind::SubscriptionDeclared,
                            requested,
                            identity.clone(),
                        );
                        self.record_work_fact_with_parent(
                            TraceRecordKind::WorkGenerationCommitted,
                            declared,
                            identity,
                        )
                    } else {
                        self.record_work_fact_with_parent(
                            TraceRecordKind::WorkGenerationCommitted,
                            requested,
                            identity,
                        )
                    };
                    semantic_parents.insert(generation.get(), committed);
                }
                PlannedWorkSemanticEvent::Invalidated(_) => {
                    let parent = semantic_parents
                        .get(&generation.get())
                        .copied()
                        .flatten()
                        .or(transaction_parent);
                    let bound = self.record_work_fact_with_parent(
                        TraceRecordKind::WorkCancellationBound,
                        parent,
                        identity.clone(),
                    );
                    let invalidated = self.record_work_fact_with_parent(
                        TraceRecordKind::WorkLogicallyInvalidated,
                        bound,
                        identity.clone(),
                    );
                    semantic_parents.insert(generation.get(), invalidated);
                    lineage.insert(generation.get(), (identity, invalidated));
                    self.invalidate_generation_now(generation);
                }
            }
        }
        lineage
    }

    fn commit_application_start(
        &mut self,
        start: crate::transaction::PlannedOwnedStart<Action, Protocol>,
    ) -> TraceWorkIdentity {
        let generation = start.generation;
        match start.payload {
            PlannedStartPayload::Effect(effect) => {
                self.work
                    .commit_record(generation, start.owner, start.family, start.key, effect);
            }
            PlannedStartPayload::Subscription(declaration) => {
                let key = declaration.key.clone();
                self.work
                    .commit_subscription_record(generation, start.owner.clone(), key);
                self.subscriptions.push(LiveSubscription::new(
                    generation,
                    start.owner,
                    declaration,
                    self.wake.handle(),
                ));
            }
        }
        self.trace_work_identity(generation)
            .unwrap_or_else(|| unreachable!("committed work has trace identity"))
    }

    fn append_planned_outputs(
        &mut self,
        outputs: Vec<PlannedOutput<Action>>,
        transaction_parent: Option<TraceSequence>,
    ) -> Result<(), CommitError> {
        for output in outputs {
            match output {
                PlannedOutput::Action(action) => {
                    self.commit_preflighted_action(
                        action,
                        transaction_parent,
                        None,
                        ApplicationActionOrigin::ApplicationEffect,
                    )
                    .map_err(|_| CommitError::Registry)?;
                }
                PlannedOutput::Start(generation) => {
                    self.queue
                        .push_effect_start(generation)
                        .unwrap_or_else(|_| {
                            unreachable!("application transaction was preflighted")
                        });
                }
                PlannedOutput::Redraw => self.request_redraw(),
            }
        }
        Ok(())
    }

    fn invalidate_generation_now(&mut self, generation: crate::work::WorkGeneration) {
        self.revoke_generation(generation);
    }

    fn revoke_generation(&mut self, generation: crate::work::WorkGeneration) {
        revoke_generation_authority(
            generation,
            &mut self.work,
            &self.completion_ingress,
            &mut self.local_tasks,
            &mut self.timers,
            &mut self.send_task_mappers,
            &mut self.subscriptions,
            &mut self.host_requests,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn revoke_generation_authority<Action, Protocol: HostProtocol>(
    generation: crate::work::WorkGeneration,
    work: &mut WorkRegistry<Action, Protocol>,
    completion_ingress: &CompletionIngress,
    local_tasks: &mut Vec<LocalTask<Action>>,
    timers: &mut Vec<Timer<Action>>,
    send_task_mappers: &mut Vec<SendTaskMapper<Action>>,
    subscriptions: &mut Vec<LiveSubscription<Action>>,
    host_requests: &mut Vec<LiveHostRequest<Action, Protocol>>,
) {
    let _ = work.invalidate(generation);
    let _ = completion_ingress.revoke_generation(generation);
    local_tasks.retain(|task| task.generation != generation);
    timers.retain(|timer| timer.generation != generation);
    send_task_mappers.retain(|mapper| mapper.generation != generation);
    subscriptions.retain(|subscription| subscription.generation != generation);
    host_requests.retain(|request| request.generation != generation);
}

fn required_application_transaction_trace_records<Action, Protocol: HostProtocol>(
    plan: &PlannedApplicationTransaction<Action, Protocol>,
) -> usize {
    required_application_transaction_trace_records_from_parts(
        &plan.invalidated,
        &plan.starts,
        &plan.application_outputs,
        &plan.mounted_outputs,
    )
    .unwrap_or(usize::MAX)
}

fn required_application_transaction_trace_records_from_parts<Action, Protocol: HostProtocol>(
    invalidated: &[crate::work::WorkGeneration],
    starts: &[crate::transaction::PlannedOwnedStart<Action, Protocol>],
    application_outputs: &[PlannedOutput<Action>],
    mounted_outputs: &[PlannedOutput<Action>],
) -> Option<usize> {
    let action_count = application_outputs
        .iter()
        .chain(mounted_outputs)
        .filter(|output| matches!(output, PlannedOutput::Action(_)))
        .count();
    let subscription_start_count = starts
        .iter()
        .filter(|start| start.family == WorkFamily::Subscription)
        .count();
    invalidated
        .len()
        .checked_mul(2)?
        .checked_add(starts.len().checked_mul(2)?)?
        .checked_add(subscription_start_count)?
        .checked_add(action_count)?
        .checked_add(1)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommitError {
    Queue,
    Registry,
}

impl From<QueueCommitError> for CommitError {
    fn from(_value: QueueCommitError) -> Self {
        Self::Queue
    }
}

impl From<RegistryInsertError> for CommitError {
    fn from(_value: RegistryInsertError) -> Self {
        Self::Registry
    }
}

const fn trace_work_family(family: WorkFamily) -> crate::TraceWorkFamily {
    match family {
        WorkFamily::LocalTask => crate::TraceWorkFamily::LocalTask,
        WorkFamily::SendTask => crate::TraceWorkFamily::SendTask,
        WorkFamily::Timer => crate::TraceWorkFamily::Timer,
        WorkFamily::Subscription => crate::TraceWorkFamily::Subscription,
        WorkFamily::HostRequest => crate::TraceWorkFamily::HostRequest,
    }
}

fn trace_work_owner(owner: &WorkOwner) -> TraceWorkOwner {
    match owner {
        WorkOwner::Application => TraceWorkOwner::Application,
        WorkOwner::Mounted(owner) => TraceWorkOwner::Mounted(owner.clone()),
    }
}

fn public_trace_work_identity(identity: WorkTraceIdentity) -> TraceWorkIdentity {
    let owner = match identity.owner {
        WorkOwner::Application => TraceWorkOwner::Application,
        WorkOwner::Mounted(owner) => TraceWorkOwner::Mounted(owner),
    };
    TraceWorkIdentity::new(
        owner,
        trace_work_family(identity.family),
        identity.generation.get(),
        identity.key,
    )
}

fn mounted_effect_into_effect<Action, Protocol: HostProtocol>(
    effect: MountedEffect<Action>,
) -> Effect<Action, Protocol> {
    match effect {
        MountedEffect::Action(action) => Effect::Action(action),
        MountedEffect::LocalTask(task) => Effect::LocalTask(task),
        MountedEffect::SendTask(task) => Effect::SendTask(task),
        MountedEffect::Timer(timer) => Effect::Timer(timer),
        MountedEffect::Cancel { family, key } => Effect::Cancel { family, key },
    }
}

fn with_routed_parent<Action>(
    output: CollectedRoutedOutput<Action>,
    causal_parent: Option<TraceSequence>,
) -> CollectedRoutedOutput<Action> {
    match output {
        CollectedRoutedOutput::Action {
            action,
            current_target,
            ..
        } => CollectedRoutedOutput::Action {
            action,
            causal_parent,
            current_target,
        },
        CollectedRoutedOutput::Command {
            target,
            command,
            origin,
            ..
        } => CollectedRoutedOutput::Command {
            target,
            command,
            origin,
            causal_parent,
        },
    }
}

#[allow(clippy::too_many_lines)]
pub(crate) fn process_application_action<App: UiApp>(
    runtime: &mut Runtime<App::State, App::Action, App::HostProtocol>,
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
    let mut mutation_phase = MutationPhase::PreMutation;
    if !runtime
        .trace
        .can_admit(MandatoryTracePlan::application_action_base(
            runtime.focus.focused_node().is_some(),
        ))
    {
        let reason = mutation_phase.terminal_reason(RuntimeTerminalReason::TraceSequenceExhausted);
        let cancelled = runtime.enter_terminal(reason, 1);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    }
    let transaction_parent = runtime.trace.record(
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
    let effects = App::update(app_state, action).into_effects();
    mutation_phase = MutationPhase::Mutated;
    let ledger = match TransactionLedger::collect(effects, runtime.limits.transaction_outputs()) {
        Ok(ledger) => ledger,
        Err(_error) => {
            let reason = RuntimeTerminalReason::Poisoned;
            let cancelled = runtime.enter_terminal(reason, 1);
            return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
        }
    };
    let update_output_count = ledger.len();
    runtime.trace.record(
        TraceRecordKind::ApplicationStateUpdated,
        Some(sequence),
        causal_parent,
        Some(before),
        None,
        target.clone(),
    );
    let transient = App::root(app_state).into_element();
    let previous_focus = runtime.focus.focused_node().cloned();
    let mut lifecycle_invalidated = Vec::new();
    let mut lifecycle_invalidated_identities = Vec::new();
    let mounted_public_slot_limit = runtime.mounted_public_slot_limit;
    let reconcile_stats = {
        let (
            tree,
            work,
            completion_ingress,
            local_tasks,
            timers,
            send_task_mappers,
            subscriptions,
            host_requests,
        ) = (
            &mut runtime.tree,
            &mut runtime.work,
            &runtime.completion_ingress,
            &mut runtime.local_tasks,
            &mut runtime.timers,
            &mut runtime.send_task_mappers,
            &mut runtime.subscriptions,
            &mut runtime.host_requests,
        );
        tree.reconcile_with_before_unmount_and_public_slot_limit(
            transient,
            mounted_public_slot_limit,
            &mut |owner| {
                let owner = WorkOwner::Mounted(owner.clone());
                let generations = work.generations_for_owner(&owner);
                for generation in &generations {
                    if let Some(identity) = work.trace_identity(*generation) {
                        lifecycle_invalidated_identities.push(public_trace_work_identity(identity));
                    }
                    revoke_generation_authority(
                        *generation,
                        work,
                        completion_ingress,
                        local_tasks,
                        timers,
                        send_task_mappers,
                        subscriptions,
                        host_requests,
                    );
                }
                lifecycle_invalidated.extend(generations);
            },
        )
    };
    let Ok(reconcile_stats) = reconcile_stats else {
        let reason = RuntimeTerminalReason::Poisoned;
        let cancelled = runtime.enter_terminal(reason, 0);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    };
    let mounted_subscription_owners = reconcile_stats.mounted_owners.clone();
    let unmounted_work_owners = reconcile_stats.unmounted_owners.clone();
    let invalidated_subscription_owners = reconcile_stats.subscription_invalidated.clone();
    let mounted_outputs = reconcile_stats.mounted_outputs;
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
    let Some(lifecycle_trace_plan) =
        MandatoryTracePlan::lifecycle_invalidations(lifecycle_invalidated_identities.len())
    else {
        let reason = mutation_phase.terminal_reason(RuntimeTerminalReason::TraceSequenceExhausted);
        let cancelled = runtime.enter_terminal(reason, 0);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    };
    if !runtime.trace.can_admit(lifecycle_trace_plan) {
        let reason = mutation_phase.terminal_reason(RuntimeTerminalReason::TraceSequenceExhausted);
        let cancelled = runtime.enter_terminal(reason, 0);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    }
    let mut lifecycle_cancellation_lineage = HashMap::new();
    for identity in lifecycle_invalidated_identities {
        let bound = runtime.record_work_fact_with_parent(
            TraceRecordKind::WorkCancellationBound,
            transaction_parent,
            identity.clone(),
        );
        let invalidated = runtime.record_work_fact_with_parent(
            TraceRecordKind::WorkLogicallyInvalidated,
            bound,
            identity.clone(),
        );
        lifecycle_cancellation_lineage.insert(identity.generation(), (identity, invalidated));
    }
    runtime.trace.record(
        TraceRecordKind::TreeReconciled,
        Some(sequence),
        causal_parent,
        Some(before),
        Some(after),
        target,
    );
    for owner in unmounted_work_owners {
        runtime
            .mounted_subscription_reconcile_pending
            .retain(|pending| pending != &owner);
    }

    let mut mounted_subscription_dirty = Vec::new();
    let mut dirty_seen = HashSet::new();
    for owner in mounted_subscription_owners
        .into_iter()
        .chain(invalidated_subscription_owners)
    {
        if runtime.tree.target_status(&owner) == TargetStatus::Live
            && !runtime
                .mounted_subscription_reconcile_pending
                .contains(&owner)
            && dirty_seen.insert(owner.clone())
        {
            mounted_subscription_dirty.push(owner);
        }
    }

    let mut total_outputs = update_output_count;
    let mut mounted_batches = Vec::with_capacity(mounted_outputs.len());
    for (owner, outputs) in mounted_outputs {
        total_outputs = match total_outputs.checked_add(outputs.len()) {
            Some(total) if total <= runtime.limits.transaction_outputs() => total,
            _ => {
                let reason = RuntimeTerminalReason::Poisoned;
                let cancelled = runtime.enter_terminal(reason, 0);
                return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
            }
        };
        let effects = outputs
            .into_iter()
            .map(mounted_effect_into_effect)
            .collect();
        let mounted_ledger =
            TransactionLedger::from_outputs(effects, runtime.limits.transaction_outputs())
                .unwrap_or_else(|_| unreachable!("complete transaction allowance was checked"));
        mounted_batches.push(OwnedTransactionLedger {
            owner: WorkOwner::Mounted(owner),
            ledger: mounted_ledger,
        });
    }

    let mut subscriptions = SubscriptionSet::new();
    App::subscriptions(runtime.state(), &mut subscriptions);
    let SubscriptionDiff {
        invalidated,
        starts,
        duplicate_keys,
    } = runtime.derive_subscription_diff(
        &WorkOwner::Application,
        subscriptions.__runtime_into_declarations(),
    );
    let subscription_cancelled = invalidated.len();
    let input = ApplicationTransactionInput {
        lifecycle_invalidated,
        mounted_subscription_dirty,
        application: ledger,
        application_subscription_invalidated: invalidated,
        application_subscription_starts: starts,
        mounted: mounted_batches,
    };
    if runtime
        .plan_and_commit_application_transaction(
            input,
            &duplicate_keys,
            subscription_cancelled,
            transaction_parent,
            lifecycle_cancellation_lineage,
        )
        .is_err()
    {
        let reason = RuntimeTerminalReason::Poisoned;
        let cancelled = runtime.enter_terminal(reason, 0);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    }
    runtime.record_optional(
        TraceRecordKind::UpdateEffectsCommitted {
            count: update_output_count,
        },
        Some(sequence),
        causal_parent,
        None,
    );
    runtime.request_redraw();
    if let RuntimeStatus::Terminal(reason) = runtime.status {
        return ProcessApplicationActionOutcome::Terminal {
            reason,
            cancelled: 0,
        };
    }
    ProcessApplicationActionOutcome::Completed
}

impl<State, Action, Protocol: HostProtocol> Drop for Runtime<State, Action, Protocol> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
