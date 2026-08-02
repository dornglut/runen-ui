use core::num::NonZeroU64;

use runenui_core::{
    CommandOrigin, ElementId, EventPhase, FocusBoundaryPolicy, FocusEventKind, FocusReason,
    InputModality, MonotonicInstant, PointerBoundaryKind, PointerCaptureKind, PointerId,
    PointerPhase, SemanticCommand, WidgetInvalidation, WorkKey,
};

use crate::{
    AutomationMatchDiagnostic, MountedNodeId, ReconciliationGeneration, RuntimeTerminalReason,
    WorkSequence,
};

/// Non-wrapping identity of one canonical trace record.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TraceSequence(NonZeroU64);

impl TraceSequence {
    /// Returns the numeric sequence value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    pub(super) const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }
}

/// Configuration for canonical in-memory trace retention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceConfig {
    capacity: usize,
}

impl TraceConfig {
    /// Creates trace configuration with the requested retained-record capacity.
    #[must_use]
    pub const fn new(capacity: usize) -> Self {
        Self { capacity }
    }

    /// Returns the retained-record capacity.
    #[must_use]
    pub const fn capacity(self) -> usize {
        self.capacity
    }
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self::new(1024)
    }
}

/// Trace target for runtime work caused by a specific mounted node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceTarget {
    mounted_node_id: MountedNodeId,
    authored_id: Option<ElementId>,
}

impl TraceTarget {
    #[must_use]
    pub(crate) const fn new(
        mounted_node_id: MountedNodeId,
        authored_id: Option<ElementId>,
    ) -> Self {
        Self {
            mounted_node_id,
            authored_id,
        }
    }

    /// Returns the mounted node identity for this target.
    #[must_use]
    pub const fn mounted_node_id(&self) -> &MountedNodeId {
        &self.mounted_node_id
    }

    /// Returns the optional authored element identity for this target.
    #[must_use]
    pub const fn authored_id(&self) -> Option<&ElementId> {
        self.authored_id.as_ref()
    }
}

/// Structured kind of one canonical trace record.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceRecordKind {
    RuntimeMounted,
    ActionSubmissionAccepted,
    CommandSubmissionAccepted,
    PointerSubmissionAccepted {
        pointer_id: PointerId,
        phase: PointerPhase,
    },
    KeyboardSubmissionAccepted,
    KeyboardSubmissionRejected,
    KeyboardProcessingValidated,
    KeyboardDefaultPrevented,
    KeyboardEnterActivationDerived,
    KeyboardSpaceOwnershipEstablished,
    KeyboardSpaceReleaseMatched {
        matched: bool,
    },
    KeyboardSpaceActivationDerived,
    KeyboardSpaceOwnershipCleared {
        reason: TraceSpaceCleanupReason,
    },
    CommittedTextSubmissionAccepted {
        bytes: usize,
        scalars: usize,
    },
    CommittedTextSubmissionRejected,
    CommittedTextProcessingValidated {
        bytes: usize,
        scalars: usize,
    },
    CommittedTextDefaultPrevented,
    CompositionGenerationAllocated,
    CompositionPendingBound,
    CompositionActiveBound,
    CompositionProcessingValidated,
    CompositionUpdated {
        has_range: bool,
    },
    CompositionEnded,
    CompositionCancelled {
        reason: runenui_core::CompositionCancelReason,
    },
    CompositionRetired,
    CompositionProcessingStaleGeneration,
    CompositionSubmissionRejected,
    AutomationResolutionUnique,
    AutomationResolutionMissing,
    AutomationResolutionAmbiguous {
        candidates: Vec<AutomationMatchDiagnostic>,
    },
    AutomationTargetStaleAfterResolution,
    PointerIngressRejected {
        pointer_id: PointerId,
        phase: PointerPhase,
        outcome: TracePointerRejection,
    },
    PointerIngressValidated {
        pointer_id: PointerId,
        phase: PointerPhase,
    },
    PointerContextUnavailable {
        pointer_id: PointerId,
        outcome: TracePointerRejection,
    },
    PointerStreamResolved {
        pointer_id: PointerId,
        new_stream: bool,
    },
    PointerStreamRegistered {
        pointer_id: PointerId,
        registration_sequence: u64,
    },
    PointerStreamObserved {
        pointer_id: PointerId,
    },
    PointerStreamClosed {
        pointer_id: PointerId,
    },
    PointerPhysicalTargetResolved {
        pointer_id: PointerId,
        snapshot: TraceSurfaceSnapshotKind,
        hit_test_generation: u64,
        coordinate_revision: u64,
    },
    PointerBoundaryBundlePlanned {
        pointer_id: PointerId,
        notifications: usize,
    },
    PointerDefaultApplied {
        pointer_id: PointerId,
        phase: PointerPhase,
    },
    PointerDefaultSuppressed {
        pointer_id: PointerId,
        phase: PointerPhase,
    },
    PointerInteractionCommitted {
        pointer_id: PointerId,
    },
    PointerCaptureTransitionQueued {
        pointer_id: PointerId,
        kind: PointerCaptureKind,
    },
    PointerBoundaryNotificationQueued {
        pointer_id: PointerId,
        kind: PointerBoundaryKind,
    },
    PointerActivateCollected {
        pointer_id: PointerId,
    },
    PointerLogicalScrollCollected {
        pointer_id: PointerId,
    },
    PointerStationaryRehitQueued {
        hit_test_generation: u64,
        coordinate_revision: u64,
    },
    PointerCaptureRequestRejected {
        pointer_id: PointerId,
        outcome: TracePointerCaptureRequestRejection,
    },
    PointerIntegrityCleanupCommitted {
        pointer_id: PointerId,
        pressed: bool,
        capture: bool,
        physical_path: bool,
    },
    PointerCaptureNotificationSuppressed {
        pointer_id: PointerId,
        kind: PointerCaptureKind,
    },
    SurfaceContextAccepted {
        ingress: TraceSurfaceIngressKind,
        snapshot: TraceSurfaceSnapshotKind,
        hit_test_generation: u64,
        coordinate_revision: u64,
    },
    SurfaceTargetBound {
        ingress: TraceSurfaceIngressKind,
        hit_test_generation: u64,
    },
    SurfaceCommandRejected {
        ingress: TraceSurfaceIngressKind,
        outcome: TraceSurfaceRejection,
    },
    CommandProcessingRejected {
        outcome: TraceTargetRejection,
    },
    RoutedEventStarted,
    RouteSnapshotCreated {
        invocations: usize,
    },
    EventPhaseInvoked {
        phase: EventPhase,
    },
    RoutedActionCollected,
    DelegatedCommandCollected {
        command: SemanticCommand,
    },
    PropagationStopped,
    DefaultPrevented,
    WidgetStateMutated,
    WidgetInvalidated {
        invalidation: WidgetInvalidation,
    },
    MountedSubscriptionInvalidated,
    SemanticDefaultApplied {
        command: SemanticCommand,
    },
    SemanticDefaultSuppressed {
        command: SemanticCommand,
    },
    RoutedEventCommitted,
    RoutedIntegrityFailed {
        failure: TraceRoutedIntegrityFailure,
    },
    RoutedEventAdmissionRejected {
        capacity: TraceRoutedAdmissionRejection,
    },
    ActionSubmissionRejectedFull,
    ActionSubmissionRejectedClosed,
    ActionSubmissionRejectedTerminal,
    ApplicationActionTransactionStarted,
    ApplicationStateUpdated,
    TreeReconciled,
    FocusRetained,
    FocusCommandEvaluated {
        command: SemanticCommand,
        linear_policy: FocusBoundaryPolicy,
        directional_policy: FocusBoundaryPolicy,
    },
    FocusCandidateSelected {
        outcome: TraceFocusBoundaryOutcome,
    },
    FocusRestorationAccepted,
    FocusRestorationRejected,
    FocusTransitionCommitted {
        reason: FocusReason,
        old_target: Option<MountedNodeId>,
        new_target: Option<MountedNodeId>,
    },
    FocusNotificationQueued {
        kind: FocusEventKind,
    },
    FocusNotificationSuppressed {
        kind: FocusEventKind,
    },
    FocusWithinInvalidated {
        left: usize,
        entered: usize,
    },
    ModalityChanged {
        previous: Option<InputModality>,
        current: InputModality,
    },
    PumpBudgetExhausted,
    InitialEffectsCommitted {
        count: usize,
    },
    InitialApplicationTransactionStarted,
    UpdateEffectsCommitted {
        count: usize,
    },
    WorkRequested,
    WorkGenerationCommitted,
    WorkStartAttempted,
    WorkStartAccepted,
    WorkStartRefused {
        outcome: TraceWorkStartRefusal,
    },
    WorkLogicallyInvalidated,
    WorkCancellationBound,
    WorkCleanupProcessed,
    WorkCompletionImported,
    WorkCompletionRejectedStale,
    WorkCompletionMapped,
    LocalWorkPolled,
    LocalWorkReady,
    TimerPromoted,
    ReadinessCheckpoint {
        imported_completions: usize,
        polled_local_work: usize,
        promoted_timers: usize,
    },
    SubscriptionDeclared,
    SubscriptionDiffCommitted {
        started: usize,
        cancelled: usize,
        duplicate_keys: usize,
    },
    MountedSubscriptionReconciliationSuppressedStale,
    TimerFired,
    TimerTerminated {
        outcome: TraceTimerTerminalOutcome,
    },
    HostRequestExposed,
    HostResponseAccepted,
    HostResponseRejected,
    WakeRequested,
    WakeAcknowledged,
    RedrawRequested {
        revision: u64,
    },
    RedrawTaken {
        revision: u64,
    },
    RedrawAcknowledged {
        revision: u64,
    },
    QueuedWorkCancelled {
        count: usize,
    },
    RuntimeTerminal {
        reason: RuntimeTerminalReason,
    },
    RuntimeShutdown {
        cancelled_queued: usize,
        unmounted_lifetimes: usize,
    },
}

/// Observable focus-scope boundary outcome without exposing scoring details.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceFocusBoundaryOutcome {
    Candidate,
    Delegated,
    Trapped,
    Stopped,
    Wrapped,
    LogicalScroll,
    Empty,
}

/// Exact routed integrity boundary that failed after command acceptance.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceRoutedIntegrityFailure {
    BrokenTopology,
    EventBridgeMismatch,
    CallbackBridgeFailure,
    OutputAllowanceExceeded,
    SemanticDefaultFailure,
    CommitInvariantFailure,
}

/// Exact target-lifetime rejection while processing an accepted command.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceTargetRejection {
    Foreign,
    Stale,
    Missing,
}

/// Why runtime-owned pressed-Space authority was revoked without activation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceSpaceCleanupReason {
    KeyboardCancel,
    FocusTransfer,
    Removal,
    Replacement,
    Disablement,
    CapabilityLoss,
    Terminal,
    Shutdown,
    Drop,
    Release,
}

/// Structured pointer rejection without route or interaction mutation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TracePointerRejection {
    ForeignRuntime,
    ForeignSurface,
    RetiredGeneration,
    MissingGeneration,
    CoordinateRevisionMismatch,
    NoTarget,
    DuplicateStream,
    MissingStream,
    RegistryFull,
    RegistrationSequenceExhausted,
    DeviceMismatch,
    DeviceKindMismatch,
    ForeignStreamSurface,
}

/// Structured rejection of one staged pointer-capture mutation request.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TracePointerCaptureRequestRejection {
    PointerMismatch,
    TargetNotInTransaction,
    TargetUnavailable,
    ReleaseNotOwner,
}

/// Checked displayed-surface ingress path.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceSurfaceIngressKind {
    LogicalCoordinate,
    ResolvedTarget,
}

/// Retained snapshot selected by a checked displayed-surface request.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceSurfaceSnapshotKind {
    Current,
    RetainedHistorical,
}

/// Structured rejection of one checked displayed-surface request.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceSurfaceRejection {
    ForeignRuntime,
    ForeignSurface,
    RetiredGeneration,
    MissingGeneration,
    CoordinateRevisionMismatch,
    NoTarget,
    TargetNotInSnapshot,
    ForeignTarget,
    StaleTarget,
    MissingTarget,
    QueueFull,
    RuntimeClosed,
    RuntimeTerminal,
    WorkSequenceExhausted,
    TraceSequenceExhausted,
}

/// Bounded authority that refused an accepted routed transaction preflight.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceRoutedAdmissionRejection {
    TransactionOutputs,
    WaitingEnvelopes,
    LocalTasks,
    SendTasks,
    Timers,
    WorkSequenceExhausted,
    WorkGenerationExhausted,
    ReconciliationGenerationExhausted,
    TraceSequenceExhausted,
    CheckedArithmeticOverflow,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceWorkFamily {
    LocalTask,
    SendTask,
    Timer,
    Subscription,
    HostRequest,
}

/// Public owner classification for opaque scheduler trace identity.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceWorkOwner {
    Application,
    Mounted(MountedNodeId),
}

/// Opaque, read-only identity of one exact scheduler work generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceWorkIdentity {
    owner: TraceWorkOwner,
    family: TraceWorkFamily,
    generation: u64,
    key: Option<WorkKey>,
}

impl TraceWorkIdentity {
    pub(crate) const fn new(
        owner: TraceWorkOwner,
        family: TraceWorkFamily,
        generation: u64,
        key: Option<WorkKey>,
    ) -> Self {
        Self {
            owner,
            family,
            generation,
            key,
        }
    }

    /// Returns the application or exact mounted owner classification.
    #[must_use]
    pub const fn owner(&self) -> &TraceWorkOwner {
        &self.owner
    }

    /// Returns the scheduler family.
    #[must_use]
    pub const fn family(&self) -> TraceWorkFamily {
        self.family
    }

    /// Returns the exact private generation as a read-only diagnostic value.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns the optional authored key.
    #[must_use]
    pub const fn key(&self) -> Option<&WorkKey> {
        self.key.as_ref()
    }
}

/// Structured executor or timer start refusal.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceWorkStartRefusal {
    ExecutorUnavailable,
    ExecutorFull,
    ExecutorClosed,
    ExecutorRejected,
    SubscriptionUnavailable,
    SubscriptionFull,
    SubscriptionClosed,
    SubscriptionRejected,
    TimerZeroInterval,
    TimerDeadlineOverflow,
}

/// Structured terminal outcome of one timer generation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceTimerTerminalOutcome {
    Completed,
    RepeatDeadlineOverflow,
}

/// One immutable canonical trace record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceRecord {
    pub(super) sequence: TraceSequence,
    pub(super) kind: TraceRecordKind,
    pub(super) work_sequence: Option<WorkSequence>,
    pub(super) causal_parent: Option<TraceSequence>,
    pub(super) reconciliation_before: Option<ReconciliationGeneration>,
    pub(super) reconciliation_after: Option<ReconciliationGeneration>,
    pub(super) target: Option<TraceTarget>,
    pub(super) work: Option<TraceWorkIdentity>,
    pub(super) instant: Option<MonotonicInstant>,
    pub(super) original_target: Option<MountedNodeId>,
    pub(super) current_target: Option<MountedNodeId>,
    pub(super) command_origin: Option<CommandOrigin>,
}

impl TraceRecord {
    /// Returns this record's trace sequence.
    #[must_use]
    pub const fn sequence(&self) -> TraceSequence {
        self.sequence
    }

    /// Returns this record's structured kind.
    #[must_use]
    pub const fn kind(&self) -> &TraceRecordKind {
        &self.kind
    }

    /// Returns the associated global work sequence, when applicable.
    #[must_use]
    pub const fn work_sequence(&self) -> Option<WorkSequence> {
        self.work_sequence
    }

    /// Returns the causal parent trace sequence, when applicable.
    #[must_use]
    pub const fn causal_parent(&self) -> Option<TraceSequence> {
        self.causal_parent
    }

    /// Returns the reconciliation generation before this record's transaction.
    #[must_use]
    pub const fn reconciliation_before(&self) -> Option<ReconciliationGeneration> {
        self.reconciliation_before
    }

    /// Returns the reconciliation generation after this record's transaction.
    #[must_use]
    pub const fn reconciliation_after(&self) -> Option<ReconciliationGeneration> {
        self.reconciliation_after
    }

    /// Returns the mounted trace target, when applicable.
    #[must_use]
    pub const fn target(&self) -> Option<&TraceTarget> {
        self.target.as_ref()
    }

    /// Returns the exact scheduler work identity for work-specific facts.
    #[must_use]
    pub const fn work(&self) -> Option<&TraceWorkIdentity> {
        self.work.as_ref()
    }

    /// Returns the accepted routed transaction time, when applicable.
    #[must_use]
    pub const fn instant(&self) -> Option<MonotonicInstant> {
        self.instant
    }

    /// Returns the immutable original routed target, when applicable.
    #[must_use]
    pub const fn original_target(&self) -> Option<&MountedNodeId> {
        self.original_target.as_ref()
    }

    /// Returns the callback's current routed target, when applicable.
    #[must_use]
    pub const fn current_target(&self) -> Option<&MountedNodeId> {
        self.current_target.as_ref()
    }

    /// Returns the normalized command origin, when applicable.
    #[must_use]
    pub const fn command_origin(&self) -> Option<CommandOrigin> {
        self.command_origin
    }
}
