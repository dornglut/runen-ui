//! Persistent runtime state, action transactions, and terminal authority.

#![allow(clippy::redundant_pub_crate)]

mod access;
mod application;
mod automation;
mod focus;
mod helpers;
mod ingress;
mod lifecycle;
mod mount;
mod pointer;
mod routed;
mod scheduler;
mod surface_publication;

pub(crate) use application::process_application_action;

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

use crate::trace::{MandatoryTracePlan, TraceReservation};
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

mod model;

use crate::input::{CompositionState, SpaceOwnership};
use automation::AutomationSubmissionPolicy;
pub(in crate::runtime) use helpers::{
    CommitError, mounted_effect_into_effect, public_trace_work_identity, trace_work_family,
    trace_work_owner, with_routed_parent,
};
pub(in crate::runtime) use lifecycle::revoke_generation_authority;
pub(crate) use model::CollectedRoutedOutput;
pub(in crate::runtime) use model::{ActionCommitError, MutationPhase};
pub use model::{
    HostRequestCancelError, HostResponseError, ReconciliationDiagnostic, ReconciliationGeneration,
    ReconciliationReport, RuntimeError, RuntimeStatus, RuntimeTerminalReason, ShutdownReport,
    SubscriptionDiagnostic, SubscriptionOwnerKind, TimerFiringOutcome, TimerStartOutcome,
};
use pointer::PointerRegistry;
pub(crate) use routed::PointerDispatchFacts;
pub(crate) use routed::{RoutedIngressFacts, RoutedTransaction};
use surface_publication::SurfacePublicationState;

pub(crate) struct Runtime<State, Action, Protocol: HostProtocol = NoHostProtocol> {
    state: Option<State>,
    pub(crate) tree: MountedTree<Action>,
    pub(crate) queue: WorkQueue<Action>,
    pub(crate) trace: Trace,
    pub(crate) focus: FocusState,
    pointer_registry: PointerRegistry,
    pub(crate) space_ownership: Option<SpaceOwnership>,
    pub(crate) composition: CompositionState,
    pub(crate) next_composition_generation: Option<core::num::NonZeroU64>,
    /// Highest generation successfully committed to the canonical input FIFO.
    ///
    /// This is deliberately distinct from the next allocator value: exhaustion
    /// alone must not make a fabricated generation appear to have been issued.
    pub(crate) last_issued_composition_generation: Option<core::num::NonZeroU64>,
    generation: u64,
    report: ReconciliationReport,
    pub(crate) status: RuntimeStatus,
    /// Synchronous public automation ingress returns exhaustion as rejection
    /// rather than changing global runtime status. No callback or pump work can
    /// observe this scope because automation submission is non-reentrant.
    automation_submission_policy: AutomationSubmissionPolicy,
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
    surface_publication: SurfacePublicationState,
    surface_trace: SurfaceTraceState,
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

struct SurfaceTraceState {
    latest_redraw_revision: Option<u64>,
    latest_redraw_request: Option<TraceSequence>,
    publication_reservation: TraceReservation,
}

impl SurfaceTraceState {
    const fn new(
        latest_redraw_revision: Option<u64>,
        latest_redraw_request: Option<TraceSequence>,
        publication_reservation: TraceReservation,
    ) -> Self {
        Self {
            latest_redraw_revision,
            latest_redraw_request,
            publication_reservation,
        }
    }

    fn note_request(&mut self, revision: u64, request: Option<TraceSequence>) {
        self.latest_redraw_revision = Some(revision);
        self.latest_redraw_request = request;
    }

    const fn request_parent(&self, revision: u64) -> Option<TraceSequence> {
        if self.latest_redraw_revision == Some(revision) {
            self.latest_redraw_request
        } else {
            None
        }
    }

    fn clear_if_acknowledged(&mut self, revision: u64, still_dirty: bool) {
        if !still_dirty && self.latest_redraw_revision.is_some_and(|current| current <= revision) {
            self.latest_redraw_revision = None;
            self.latest_redraw_request = None;
        }
    }
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
