//! Immutable, host-neutral observations of the canonical runtime's own input authority.
//!
//! Accepted input receipts are *pending* until the canonical pump settles their
//! work sequence or the exact runtime scope retires. These are not gameplay
//! inputs, host-native window identities, mutable UI state or diagnostic trace.

use core::{
    fmt,
    hash::{Hash, Hasher},
};

use runenui_core::{
    __runtime::RuntimeNamespace, CompositionGeneration, InputDeviceId, MountedNodeId, PointerId,
    SurfaceId, SurfaceInputContext, WidgetTextInput, WorkSequence,
};

use crate::{PumpReport, RuntimeStatus, RuntimeTerminalReason, ShutdownReport};

/// One unforgeable runtime lifetime; disjoint across separately mounted runtimes.
#[derive(Clone)]
pub struct InputArbitrationScope {
    namespace: RuntimeNamespace,
}

impl InputArbitrationScope {
    pub(crate) fn new(namespace: RuntimeNamespace) -> Self {
        Self { namespace }
    }
}

impl PartialEq for InputArbitrationScope {
    fn eq(&self, other: &Self) -> bool {
        self.namespace.__runtime_same_as(&other.namespace)
    }
}
impl Eq for InputArbitrationScope {}
impl Hash for InputArbitrationScope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.namespace.__runtime_hash(state);
    }
}
impl fmt::Debug for InputArbitrationScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputArbitrationScope").finish_non_exhaustive()
    }
}

/// Non-wrapping revision of meaningful public UI input-ownership facts.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InputOwnershipRevision(u64);
impl InputOwnershipRevision {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Focus, editing and keyboard-lifetime facts, not gameplay policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyboardInputOwnership {
    pub(crate) focused_node: Option<MountedNodeId>,
    pub(crate) text_input_capability: WidgetTextInput,
    pub(crate) composition_generation: Option<CompositionGeneration>,
    pub(crate) space_activation_owner: Option<MountedNodeId>,
}
impl KeyboardInputOwnership {
    #[must_use]
    pub const fn focused_node(&self) -> Option<&MountedNodeId> {
        self.focused_node.as_ref()
    }
    #[must_use]
    pub const fn text_input_capability(&self) -> WidgetTextInput {
        self.text_input_capability
    }
    #[must_use]
    pub const fn composition_generation(&self) -> Option<&CompositionGeneration> {
        self.composition_generation.as_ref()
    }
    #[must_use]
    pub const fn space_activation_owner(&self) -> Option<&MountedNodeId> {
        self.space_activation_owner.as_ref()
    }
}

/// One exact active UI pointer stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PointerInputOwnership {
    pub(crate) pointer_id: PointerId,
    pub(crate) device_id: Option<InputDeviceId>,
    pub(crate) surface_id: SurfaceId,
    pub(crate) pressed_owner: Option<MountedNodeId>,
    pub(crate) capture_owner: Option<MountedNodeId>,
}
impl PointerInputOwnership {
    #[must_use]
    pub const fn pointer_id(&self) -> PointerId {
        self.pointer_id
    }
    #[must_use]
    pub const fn device_id(&self) -> Option<InputDeviceId> {
        self.device_id
    }
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface_id
    }
    #[must_use]
    pub const fn pressed_owner(&self) -> Option<&MountedNodeId> {
        self.pressed_owner.as_ref()
    }
    #[must_use]
    pub const fn capture_owner(&self) -> Option<&MountedNodeId> {
        self.capture_owner.as_ref()
    }
}

/// RunenUI-retained logical context; this is **not** a completed native Present.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceInputOwnership {
    pub(crate) surface_id: SurfaceId,
    pub(crate) latest_retained_context: Option<SurfaceInputContext>,
    pub(crate) modal_blocker: Option<MountedNodeId>,
}
impl SurfaceInputOwnership {
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface_id
    }
    #[must_use]
    pub const fn latest_retained_context(&self) -> Option<&SurfaceInputContext> {
        self.latest_retained_context.as_ref()
    }
    #[must_use]
    pub const fn modal_blocker(&self) -> Option<&MountedNodeId> {
        self.modal_blocker.as_ref()
    }
}

/// Immutable exact runtime-owned input state, never a second ownership registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputOwnershipSnapshot {
    pub(crate) scope: InputArbitrationScope,
    pub(crate) revision: InputOwnershipRevision,
    pub(crate) status: RuntimeStatus,
    pub(crate) keyboard: KeyboardInputOwnership,
    pub(crate) surfaces: Vec<SurfaceInputOwnership>,
    pub(crate) pointers: Vec<PointerInputOwnership>,
}
impl InputOwnershipSnapshot {
    #[must_use]
    pub const fn scope(&self) -> &InputArbitrationScope {
        &self.scope
    }
    #[must_use]
    pub const fn revision(&self) -> InputOwnershipRevision {
        self.revision
    }
    #[must_use]
    pub const fn status(&self) -> RuntimeStatus {
        self.status
    }
    #[must_use]
    pub const fn keyboard(&self) -> &KeyboardInputOwnership {
        &self.keyboard
    }
    #[must_use]
    pub fn surfaces(&self) -> &[SurfaceInputOwnership] {
        &self.surfaces
    }
    #[must_use]
    pub fn pointers(&self) -> &[PointerInputOwnership] {
        &self.pointers
    }

    pub(crate) fn same_ownership_facts(&self, other: &Self) -> bool {
        self.status == other.status
            && self.keyboard == other.keyboard
            && self.surfaces == other.surfaces
            && self.pointers == other.pointers
    }
}

/// One synchronous owner-authored transition after a canonical commit boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputOwnershipTransition {
    pub(crate) before_revision: InputOwnershipRevision,
    pub(crate) after: InputOwnershipSnapshot,
}
impl InputOwnershipTransition {
    #[must_use]
    pub const fn before_revision(&self) -> InputOwnershipRevision {
        self.before_revision
    }
    #[must_use]
    pub const fn after(&self) -> &InputOwnershipSnapshot {
        &self.after
    }
}

/// Accepted host input family, not synthetic internal re-hit or action work.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiInputFamily {
    Keyboard,
    CommittedText,
    Composition,
    Pointer,
}

/// Truthful routed target or presentation barrier.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiInputRoute {
    Unrouted,
    Routed { target: MountedNodeId },
    Captured { target: MountedNodeId },
    PresentationBlocked { root: MountedNodeId },
}

/// Runtime-certified conflict classification; only the host owns game eligibility.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiInputConflict {
    Unclaimed,
    ObservedNonexclusive,
    ExclusiveUi,
    Undetermined,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiInputClaimReason {
    TextOwner,
    CompositionOwner,
    FocusNavigation,
    ActivationDefault,
    ApplicationShortcut,
    PointerPress,
    PointerCapture,
    PointerSelection,
    TouchGesture,
    ModalBarrier,
    PresentationDismissal,
    ExplicitWidgetClaim,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiDefaultDisposition {
    None,
    Prevented,
    Applied,
    Queued,
}

/// Immutable committed routing facts; no secret editor or preedit payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiInputRoutingFacts {
    pub(crate) conflict: UiInputConflict,
    pub(crate) reasons: Vec<UiInputClaimReason>,
    pub(crate) route: UiInputRoute,
    pub(crate) propagation_stopped: bool,
    pub(crate) default_prevented: bool,
    pub(crate) default_disposition: UiDefaultDisposition,
}
impl UiInputRoutingFacts {
    #[must_use]
    pub const fn conflict(&self) -> UiInputConflict {
        self.conflict
    }
    #[must_use]
    pub fn reasons(&self) -> &[UiInputClaimReason] {
        &self.reasons
    }
    #[must_use]
    pub const fn route(&self) -> &UiInputRoute {
        &self.route
    }
    #[must_use]
    pub const fn propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }
    #[must_use]
    pub const fn default_prevented(&self) -> bool {
        self.default_prevented
    }
    #[must_use]
    pub const fn default_disposition(&self) -> UiDefaultDisposition {
        self.default_disposition
    }
}

/// An input reached the queue front but its route/default did not commit.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiInputProcessingRejection {
    MissingTarget,
    StaleTarget,
    ForeignTarget,
    InvalidDisplayedSnapshot,
    MissingDisplayedSnapshot,
    InvalidPointerStream,
    StaleCompositionGeneration,
    InsufficientTransactionCapacity,
}

/// Processing integrity is unknown: never a positive game permission.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiInputAbortReason {
    RuntimeIntegrity,
    TraceOrSequenceExhausted,
    Terminal(RuntimeTerminalReason),
}

/// Exactly one final status per reached host input; admission failure is separate.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiInputFinality {
    Committed(UiInputRoutingFacts),
    ProcessingRejected(UiInputProcessingRejection),
    Aborted(UiInputAbortReason),
}

/// Exact scope and sequence prevent correlation to another runtime lifetime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiInputSettlement {
    pub(crate) scope: InputArbitrationScope,
    pub(crate) sequence: WorkSequence,
    pub(crate) family: UiInputFamily,
    pub(crate) surface_id: Option<SurfaceId>,
    pub(crate) device_id: Option<InputDeviceId>,
    pub(crate) pointer_id: Option<PointerId>,
    pub(crate) finality: UiInputFinality,
    pub(crate) ownership_revision: InputOwnershipRevision,
}
impl UiInputSettlement {
    #[must_use]
    pub const fn scope(&self) -> &InputArbitrationScope {
        &self.scope
    }
    #[must_use]
    pub const fn sequence(&self) -> WorkSequence {
        self.sequence
    }
    #[must_use]
    pub const fn family(&self) -> UiInputFamily {
        self.family
    }
    #[must_use]
    pub const fn surface_id(&self) -> Option<&SurfaceId> {
        self.surface_id.as_ref()
    }
    #[must_use]
    pub const fn device_id(&self) -> Option<InputDeviceId> {
        self.device_id
    }
    #[must_use]
    pub const fn pointer_id(&self) -> Option<PointerId> {
        self.pointer_id
    }
    #[must_use]
    pub const fn finality(&self) -> &UiInputFinality {
        &self.finality
    }
    #[must_use]
    pub const fn ownership_revision(&self) -> InputOwnershipRevision {
        self.ownership_revision
    }
}

/// One runtime-lifetime retirement is enough to invalidate all pending receipts.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputScopeRetirementReason {
    Shutdown,
    Terminal(RuntimeTerminalReason),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputScopeRetirement {
    pub(crate) scope: InputArbitrationScope,
    pub(crate) reason: InputScopeRetirementReason,
}
impl InputScopeRetirement {
    #[must_use]
    pub const fn scope(&self) -> &InputArbitrationScope {
        &self.scope
    }
    #[must_use]
    pub const fn reason(&self) -> InputScopeRetirementReason {
        self.reason
    }
}

/// A **single ordered** returned stream, not input and revision replay queues.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputArbitrationRecord {
    InputSettled(UiInputSettlement),
    OwnershipChanged(InputOwnershipTransition),
    ScopeRetired(InputScopeRetirement),
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPumpPauseReason {
    ObservationCapacity,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputObservationError {
    Capacity,
    RevisionExhausted,
}
impl fmt::Display for InputObservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "host input observation failed: {self:?}")
    }
}
impl std::error::Error for InputObservationError {}

/// Caller-owned bounded canonical pump observations.
pub struct InputPumpBatch {
    pub(crate) report: PumpReport,
    pub(crate) ordered_records: Vec<InputArbitrationRecord>,
    pub(crate) processed_through: Option<WorkSequence>,
    pub(crate) final_ownership: InputOwnershipSnapshot,
    pub(crate) pause_reason: Option<InputPumpPauseReason>,
}
impl InputPumpBatch {
    #[must_use]
    pub const fn report(&self) -> &PumpReport {
        &self.report
    }
    #[must_use]
    pub fn ordered_records(&self) -> &[InputArbitrationRecord] {
        &self.ordered_records
    }
    #[must_use]
    pub const fn processed_through(&self) -> Option<WorkSequence> {
        self.processed_through
    }
    #[must_use]
    pub const fn final_ownership(&self) -> &InputOwnershipSnapshot {
        &self.final_ownership
    }
    #[must_use]
    pub const fn pause_reason(&self) -> Option<InputPumpPauseReason> {
        self.pause_reason
    }
}

/// Caller-owned final closed state and bounded terminal retirement observations.
pub struct InputShutdownBatch {
    pub(crate) report: ShutdownReport,
    pub(crate) ordered_records: Vec<InputArbitrationRecord>,
    pub(crate) final_ownership: InputOwnershipSnapshot,
}
impl InputShutdownBatch {
    #[must_use]
    pub const fn report(&self) -> &ShutdownReport {
        &self.report
    }
    #[must_use]
    pub fn ordered_records(&self) -> &[InputArbitrationRecord] {
        &self.ordered_records
    }
    #[must_use]
    pub const fn final_ownership(&self) -> &InputOwnershipSnapshot {
        &self.final_ownership
    }
}
