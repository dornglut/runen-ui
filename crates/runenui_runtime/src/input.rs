//! Public input receipts and canonical keyboard/text ingress.

mod type_ahead;
pub use type_ahead::{FocusGroupTypeAheadState, FocusGroupTypeAheadUpdate};

use core::fmt;
use runenui_core::{
    CommandOrigin, CommittedTextEvent, CompositionCancel, CompositionCancelReason, CompositionEnd,
    CompositionEvent, CompositionGeneration, CompositionRange, CompositionStart, CompositionUpdate,
    ElementId, FocusGroupActivationPolicy, HostProtocol, InputDeviceId, KeyboardCompositionState,
    KeyboardEvent, KeyboardPhase, LogicalKey, MonotonicInstant, PhysicalKey, SemanticCommand,
    ShortcutBinding, UiEvent, WorkSequence,
};

use crate::{
    RuntimeStatus, RuntimeTerminalReason, TraceAutomationContext, TraceCompositionContext,
    TraceContext, TraceDeliveryOutcome, TraceEventContext, TraceEventFamily, TraceInputContext,
    TraceRecordKind, TraceSpaceCleanupReason,
    mounted::{AutomationResolution, TargetStatus},
    queue::{InputEnvelope, InputEnvelopePayload},
    runtime::{RoutedIngressFacts, Runtime},
    trace::{MandatoryTracePlan, TraceRecordDraft},
};

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitKeyboardErrorKind {
    Full,
    Closed,
    Terminal(RuntimeTerminalReason),
    NoFocusedTarget,
    WorkSequenceExhausted,
    TraceSequenceExhausted,
}
#[must_use]
pub struct SubmitKeyboardError {
    kind: SubmitKeyboardErrorKind,
    event: KeyboardEvent,
}
impl SubmitKeyboardError {
    pub(crate) const fn new(kind: SubmitKeyboardErrorKind, event: KeyboardEvent) -> Self {
        Self { kind, event }
    }
    #[must_use]
    pub const fn kind(&self) -> SubmitKeyboardErrorKind {
        self.kind
    }
    #[must_use]
    pub const fn event(&self) -> &KeyboardEvent {
        &self.event
    }
    #[must_use]
    pub fn into_event(self) -> KeyboardEvent {
        self.event
    }
}
impl fmt::Debug for SubmitKeyboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubmitKeyboardError")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}
impl fmt::Display for SubmitKeyboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "keyboard submission rejected: {:?}", self.kind)
    }
}
impl std::error::Error for SubmitKeyboardError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyboardSubmission {
    sequence: WorkSequence,
}
impl KeyboardSubmission {
    pub(crate) const fn new(sequence: WorkSequence) -> Self {
        Self { sequence }
    }
    #[must_use]
    pub const fn sequence(self) -> WorkSequence {
        self.sequence
    }
}

#[derive(Clone, Debug)]
enum KeyboardShortcutCandidate {
    Unique {
        scope: crate::MountedNodeId,
        binding: ShortcutBinding,
    },
    Ambiguous {
        scope: crate::MountedNodeId,
    },
}

impl KeyboardShortcutCandidate {
    const fn reserves_default_output(&self) -> bool {
        matches!(
            self,
            Self::Unique { binding, .. } if binding.command().enabled()
        )
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitTextErrorKind {
    Full,
    Closed,
    Terminal(RuntimeTerminalReason),
    NoFocusedTarget,
    FocusedTargetNotTextCapable,
    EditingUnavailable,
    EditingCapacity,
    EditRequestExhausted,
    WorkSequenceExhausted,
    TraceSequenceExhausted,
}
#[must_use]
pub struct SubmitTextError {
    kind: SubmitTextErrorKind,
    event: CommittedTextEvent,
}
impl SubmitTextError {
    pub(crate) const fn new(kind: SubmitTextErrorKind, event: CommittedTextEvent) -> Self {
        Self { kind, event }
    }
    #[must_use]
    pub const fn kind(&self) -> SubmitTextErrorKind {
        self.kind
    }
    #[must_use]
    pub const fn event(&self) -> &CommittedTextEvent {
        &self.event
    }
    #[must_use]
    pub fn into_event(self) -> CommittedTextEvent {
        self.event
    }
}
impl fmt::Debug for SubmitTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubmitTextError")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}
impl fmt::Display for SubmitTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "committed-text submission rejected: {:?}", self.kind)
    }
}
impl std::error::Error for SubmitTextError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextSubmission {
    sequence: WorkSequence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionStartSubmission {
    sequence: WorkSequence,
    generation: CompositionGeneration,
}
impl CompositionStartSubmission {
    pub(crate) const fn new(sequence: WorkSequence, generation: CompositionGeneration) -> Self {
        Self {
            sequence,
            generation,
        }
    }
    #[must_use]
    pub const fn sequence(&self) -> WorkSequence {
        self.sequence
    }
    #[must_use]
    pub const fn generation(&self) -> &CompositionGeneration {
        &self.generation
    }
}

/// Caller-owned facts for a requested composition start.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionStartRequest {
    device_id: Option<InputDeviceId>,
}

impl CompositionStartRequest {
    #[must_use]
    pub const fn new(device_id: Option<InputDeviceId>) -> Self {
        Self { device_id }
    }

    #[must_use]
    pub const fn device_id(self) -> Option<InputDeviceId> {
        self.device_id
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionSubmission {
    sequence: WorkSequence,
}
impl CompositionSubmission {
    pub(crate) const fn new(sequence: WorkSequence) -> Self {
        Self { sequence }
    }
    #[must_use]
    pub const fn sequence(self) -> WorkSequence {
        self.sequence
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitCompositionErrorKind {
    Full,
    Closed,
    Terminal(RuntimeTerminalReason),
    NoFocusedTarget,
    FocusedTargetNotCompositionCapable,
    MissingGeneration,
    StaleGeneration,
    ForeignGeneration,
    InvalidRange,
    WorkSequenceExhausted,
    CompositionGenerationExhausted,
    TraceSequenceExhausted,
}

#[must_use]
pub struct SubmitCompositionError {
    kind: SubmitCompositionErrorKind,
    event: CompositionEvent,
}
impl SubmitCompositionError {
    const fn new(kind: SubmitCompositionErrorKind, event: CompositionEvent) -> Self {
        Self { kind, event }
    }
    #[must_use]
    pub const fn kind(&self) -> SubmitCompositionErrorKind {
        self.kind
    }
    #[must_use]
    pub const fn event(&self) -> &CompositionEvent {
        &self.event
    }
    #[must_use]
    pub fn into_event(self) -> CompositionEvent {
        self.event
    }
}
impl fmt::Debug for SubmitCompositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubmitCompositionError")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}
impl fmt::Display for SubmitCompositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "composition submission rejected: {:?}", self.kind)
    }
}
impl std::error::Error for SubmitCompositionError {}

#[must_use]
pub struct SubmitCompositionStartError {
    kind: SubmitCompositionErrorKind,
    request: CompositionStartRequest,
}

impl SubmitCompositionStartError {
    const fn new(kind: SubmitCompositionErrorKind, request: CompositionStartRequest) -> Self {
        Self { kind, request }
    }

    #[must_use]
    pub const fn kind(&self) -> SubmitCompositionErrorKind {
        self.kind
    }

    #[must_use]
    pub const fn request(&self) -> CompositionStartRequest {
        self.request
    }

    #[must_use]
    pub const fn into_request(self) -> CompositionStartRequest {
        self.request
    }
}

impl fmt::Debug for SubmitCompositionStartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubmitCompositionStartError")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for SubmitCompositionStartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "composition start rejected: {:?}", self.kind)
    }
}

impl std::error::Error for SubmitCompositionStartError {}

/// Runtime-owned pressed-Space authority for one exact focused lifetime.
pub struct SpaceOwnership {
    pub(crate) target: crate::MountedNodeId,
    pub(crate) device_id: Option<runenui_core::InputDeviceId>,
    pub(crate) down_sequence: WorkSequence,
}

pub enum CompositionState {
    None,
    Pending {
        generation: CompositionGeneration,
        owner: crate::MountedNodeId,
        device_id: Option<InputDeviceId>,
        start_sequence: WorkSequence,
    },
    Active {
        generation: CompositionGeneration,
        owner: crate::MountedNodeId,
        device_id: Option<InputDeviceId>,
        start_sequence: WorkSequence,
    },
}

impl CompositionState {
    pub(crate) const fn generation(&self) -> Option<&CompositionGeneration> {
        match self {
            Self::None => None,
            Self::Pending { generation, .. } | Self::Active { generation, .. } => Some(generation),
        }
    }

    pub(crate) const fn owner(&self) -> Option<&crate::MountedNodeId> {
        match self {
            Self::None => None,
            Self::Pending { owner, .. } | Self::Active { owner, .. } => Some(owner),
        }
    }

    pub(crate) const fn device_id(&self) -> Option<InputDeviceId> {
        match self {
            Self::None => None,
            Self::Pending { device_id, .. } | Self::Active { device_id, .. } => *device_id,
        }
    }

    pub(crate) const fn start_sequence(&self) -> Option<WorkSequence> {
        match self {
            Self::None => None,
            Self::Pending { start_sequence, .. } | Self::Active { start_sequence, .. } => {
                Some(*start_sequence)
            }
        }
    }

    pub(crate) fn trace_context(&self) -> Option<TraceCompositionContext> {
        self.generation()
            .cloned()
            .map(|generation| TraceCompositionContext::new(generation, self.device_id()))
    }
}

impl TextSubmission {
    pub(crate) const fn new(sequence: WorkSequence) -> Self {
        Self { sequence }
    }
    #[must_use]
    pub const fn sequence(self) -> WorkSequence {
        self.sequence
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubmitAutomationErrorKind {
    MissingAuthoredId,
    AmbiguousAuthoredId {
        candidates: Vec<crate::AutomationMatchDiagnostic>,
    },
    Command(crate::SubmitCommandErrorKind),
}

#[must_use]
pub struct SubmitAutomationError {
    kind: SubmitAutomationErrorKind,
    authored_id: ElementId,
    command: SemanticCommand,
}
impl SubmitAutomationError {
    const fn new(
        kind: SubmitAutomationErrorKind,
        authored_id: ElementId,
        command: SemanticCommand,
    ) -> Self {
        Self {
            kind,
            authored_id,
            command,
        }
    }
    #[must_use]
    pub const fn kind(&self) -> &SubmitAutomationErrorKind {
        &self.kind
    }
    #[must_use]
    pub const fn authored_id(&self) -> &ElementId {
        &self.authored_id
    }
    #[must_use]
    pub const fn command(&self) -> SemanticCommand {
        self.command
    }
    #[must_use]
    pub fn into_request(self) -> (ElementId, SemanticCommand) {
        (self.authored_id, self.command)
    }
}
impl fmt::Debug for SubmitAutomationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubmitAutomationError")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}
impl fmt::Display for SubmitAutomationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "automation submission rejected: {:?}", self.kind)
    }
}
impl std::error::Error for SubmitAutomationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutomationSubmission {
    sequence: WorkSequence,
}
impl AutomationSubmission {
    const fn new(sequence: WorkSequence) -> Self {
        Self { sequence }
    }
    #[must_use]
    pub const fn sequence(self) -> WorkSequence {
        self.sequence
    }
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn revoke_space_ownership(&mut self, reason: TraceSpaceCleanupReason) {
        let Some(ownership) = self.space_ownership.take() else {
            return;
        };
        self.trace.record(
            TraceRecordKind::KeyboardSpaceOwnershipCleared { reason },
            None,
            None,
            None,
            None,
            Some(self.tree.trace_target(&ownership.target)),
        );
    }

    pub(crate) fn submit_automation_command(
        &mut self,
        authored_id: ElementId,
        command: SemanticCommand,
    ) -> Result<AutomationSubmission, SubmitAutomationError> {
        if !self
            .trace
            .can_admit(MandatoryTracePlan::automation_resolution())
        {
            let reason = RuntimeTerminalReason::TraceSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(SubmitAutomationError::new(
                SubmitAutomationErrorKind::Command(
                    crate::SubmitCommandErrorKind::TraceSequenceExhausted,
                ),
                authored_id,
                command,
            ));
        }
        let instant = self.now();
        let (target, resolution_parent) = match self.tree.resolve_authored_id(&authored_id) {
            AutomationResolution::Missing => {
                self.trace.record_draft(TraceRecordDraft::automation_fact(
                    TraceRecordKind::AutomationResolutionMissing,
                    instant,
                    TraceContext::automation_record(TraceAutomationContext::missing(
                        authored_id.clone(),
                        command,
                    )),
                ));
                return Err(SubmitAutomationError::new(
                    SubmitAutomationErrorKind::MissingAuthoredId,
                    authored_id,
                    command,
                ));
            }
            AutomationResolution::Unique(target) => {
                let parent = self.trace.record_draft(
                    TraceRecordDraft::automation_fact(
                        TraceRecordKind::AutomationResolutionUnique,
                        instant,
                        TraceContext::automation_record(TraceAutomationContext::unique(
                            authored_id.clone(),
                            command,
                        )),
                    )
                    .with_target(Some(self.tree.trace_target(&target))),
                );
                (target, parent)
            }
            AutomationResolution::Ambiguous { candidates } => {
                self.trace.record_draft(TraceRecordDraft::automation_fact(
                    TraceRecordKind::AutomationResolutionAmbiguous,
                    instant,
                    TraceContext::automation_record(TraceAutomationContext::ambiguous(
                        authored_id.clone(),
                        command,
                        candidates.clone(),
                    )),
                ));
                return Err(SubmitAutomationError::new(
                    SubmitAutomationErrorKind::AmbiguousAuthoredId { candidates },
                    authored_id,
                    command,
                ));
            }
        };
        self.submit_command_with_parent(
            target,
            command,
            CommandOrigin::automation(),
            resolution_parent,
        )
        .map(|submission| AutomationSubmission::new(submission.sequence()))
        .map_err(|error| {
            SubmitAutomationError::new(
                SubmitAutomationErrorKind::Command(error.kind()),
                authored_id,
                command,
            )
        })
    }

    pub(crate) fn submit_keyboard(
        &mut self,
        event: KeyboardEvent,
    ) -> Result<KeyboardSubmission, SubmitKeyboardError> {
        let target = self
            .input_target()
            .map_err(|kind| SubmitKeyboardError::new(kind, event.clone()))?;
        self.commit_input(target, InputEnvelopePayload::Keyboard(event))
            .map(KeyboardSubmission::new)
            .map_err(|(kind, event)| {
                let InputEnvelopePayload::Keyboard(event) = event else {
                    unreachable!()
                };
                SubmitKeyboardError::new(kind, event)
            })
    }

    pub(crate) fn submit_text(
        &mut self,
        event: CommittedTextEvent,
    ) -> Result<TextSubmission, SubmitTextError> {
        let target = self
            .input_target_text()
            .map_err(|kind| SubmitTextError::new(kind, event.clone()))?;
        self.commit_input(target, InputEnvelopePayload::CommittedText(event))
            .map(TextSubmission::new)
            .map_err(|(kind, event)| {
                let InputEnvelopePayload::CommittedText(event) = event else {
                    unreachable!()
                };
                SubmitTextError::new(
                    match kind {
                        SubmitKeyboardErrorKind::Full => SubmitTextErrorKind::Full,
                        SubmitKeyboardErrorKind::Closed => SubmitTextErrorKind::Closed,
                        SubmitKeyboardErrorKind::Terminal(reason) => {
                            SubmitTextErrorKind::Terminal(reason)
                        }
                        SubmitKeyboardErrorKind::NoFocusedTarget => {
                            SubmitTextErrorKind::NoFocusedTarget
                        }
                        SubmitKeyboardErrorKind::WorkSequenceExhausted => {
                            SubmitTextErrorKind::WorkSequenceExhausted
                        }
                        SubmitKeyboardErrorKind::TraceSequenceExhausted => {
                            SubmitTextErrorKind::TraceSequenceExhausted
                        }
                    },
                    event,
                )
            })
    }

    pub(crate) fn start_composition(
        &mut self,
        device_id: Option<InputDeviceId>,
    ) -> Result<CompositionStartSubmission, SubmitCompositionStartError> {
        let request = CompositionStartRequest::new(device_id);
        let target = self.composition_start_target(request)?;
        let Some(next) = self.next_composition_generation else {
            return Err(SubmitCompositionStartError::new(
                SubmitCompositionErrorKind::CompositionGenerationExhausted,
                request,
            ));
        };
        let trace_plan = MandatoryTracePlan::composition_start_acceptance()
            .checked_add(MandatoryTracePlan::input_acceptance())
            .unwrap_or_else(|| unreachable!("composition trace plan has a fixed bounded size"));
        if !self.trace.can_admit(trace_plan) {
            return Err(Self::composition_start_error(
                SubmitCompositionErrorKind::TraceSequenceExhausted,
                request,
            ));
        }
        let Some(reservation) = self.trace.reserve_input_outcome() else {
            return Err(Self::composition_start_error(
                SubmitCompositionErrorKind::TraceSequenceExhausted,
                request,
            ));
        };
        let Some(sequence) = self.queue.next_sequence() else {
            self.trace.release_reservation(reservation);
            return Err(Self::composition_start_error(
                SubmitCompositionErrorKind::WorkSequenceExhausted,
                request,
            ));
        };
        let generation = self.tree.composition_generation(next.get());
        let event = CompositionEvent::Start(CompositionStart::__runtime_new(
            generation.clone(),
            request.device_id(),
        ));
        let instant = self.now();
        let accepted = self.trace.record_draft(
            TraceRecordDraft::input_fact(
                TraceRecordKind::CompositionGenerationAllocated,
                instant,
                TraceContext::input_record(TraceInputContext::composition_identity(
                    TraceCompositionContext::new(generation.clone(), request.device_id()),
                )),
            )
            .with_work_sequence(Some(sequence))
            .with_target(Some(self.tree.trace_target(&target))),
        );
        debug_assert!(accepted.is_some() || !self.trace.is_enabled());
        self.composition = CompositionState::Pending {
            generation: generation.clone(),
            owner: target.clone(),
            device_id: request.device_id(),
            start_sequence: sequence,
        };
        let pending_bound = self.trace.record_draft(
            TraceRecordDraft::input_marker(TraceRecordKind::CompositionPendingBound, instant)
                .with_work_sequence(Some(sequence))
                .with_causal_parent(accepted)
                .with_target(Some(self.tree.trace_target(&target))),
        );
        debug_assert!(pending_bound.is_some() || !self.trace.is_enabled());
        self.next_composition_generation = next
            .get()
            .checked_add(1)
            .and_then(core::num::NonZeroU64::new);
        let committed = self
            .queue
            .push_input_preflighted(
                target,
                InputEnvelopePayload::Composition(event),
                instant,
                pending_bound,
                reservation,
            )
            .unwrap_or_else(|_| unreachable!("composition queue was preflighted"));
        self.last_issued_composition_generation = Some(next);
        self.external_queue_commit_accepted();
        Ok(CompositionStartSubmission::new(committed, generation))
    }

    pub(crate) fn submit_composition_update(
        &mut self,
        generation: CompositionGeneration,
        preedit: String,
        range: Option<CompositionRange>,
    ) -> Result<CompositionSubmission, SubmitCompositionError> {
        let event =
            CompositionEvent::Update(CompositionUpdate::__runtime_new(generation, preedit, range));
        if let CompositionEvent::Update(update) = &event
            && let Some(range) = update.range()
            && CompositionRange::new(update.preedit(), range.start(), range.end()).is_err()
        {
            return Err(SubmitCompositionError::new(
                SubmitCompositionErrorKind::InvalidRange,
                event,
            ));
        }
        self.submit_existing_composition(event)
    }

    pub(crate) fn submit_composition_end(
        &mut self,
        generation: CompositionGeneration,
    ) -> Result<CompositionSubmission, SubmitCompositionError> {
        self.submit_existing_composition(CompositionEvent::End(CompositionEnd::__runtime_new(
            generation,
        )))
    }

    pub(crate) fn cancel_composition(
        &mut self,
        generation: CompositionGeneration,
    ) -> Result<CompositionSubmission, SubmitCompositionError> {
        self.submit_existing_composition(CompositionEvent::Cancel(
            CompositionCancel::__runtime_new(generation, CompositionCancelReason::Explicit),
        ))
    }

    fn composition_start_target(
        &mut self,
        request: CompositionStartRequest,
    ) -> Result<crate::MountedNodeId, SubmitCompositionStartError> {
        match self.status {
            RuntimeStatus::Running => {}
            RuntimeStatus::Closed => {
                return Err(Self::composition_start_error(
                    SubmitCompositionErrorKind::Closed,
                    request,
                ));
            }
            RuntimeStatus::Terminal(reason) => {
                return Err(Self::composition_start_error(
                    SubmitCompositionErrorKind::Terminal(reason),
                    request,
                ));
            }
        }
        if !matches!(self.composition, CompositionState::None) {
            return Err(Self::composition_start_error(
                SubmitCompositionErrorKind::StaleGeneration,
                request,
            ));
        }
        let target = self.focus.focused_node().cloned().ok_or_else(|| {
            Self::composition_start_error(SubmitCompositionErrorKind::NoFocusedTarget, request)
        })?;
        let capability = self.tree.text_input_probe(&target).map_err(|_| {
            Self::composition_start_error(
                SubmitCompositionErrorKind::FocusedTargetNotCompositionCapable,
                request,
            )
        })?;
        if !capability.accepts_composition() {
            return Err(Self::composition_start_error(
                SubmitCompositionErrorKind::FocusedTargetNotCompositionCapable,
                request,
            ));
        }
        match self.queue.preflight_commit(1) {
            Ok(()) => Ok(target),
            Err(crate::queue::QueueCommitError::Full) => Err(Self::composition_start_error(
                SubmitCompositionErrorKind::Full,
                request,
            )),
            Err(crate::queue::QueueCommitError::SequenceExhausted) => {
                Err(Self::composition_start_error(
                    SubmitCompositionErrorKind::WorkSequenceExhausted,
                    request,
                ))
            }
        }
    }

    const fn composition_start_error(
        kind: SubmitCompositionErrorKind,
        request: CompositionStartRequest,
    ) -> SubmitCompositionStartError {
        SubmitCompositionStartError::new(kind, request)
    }

    fn submit_existing_composition(
        &mut self,
        event: CompositionEvent,
    ) -> Result<CompositionSubmission, SubmitCompositionError> {
        let validation = self.validate_existing_composition(&event);
        let (owner, composition) = match validation {
            Ok(validated) => validated,
            Err(kind) => return Err(SubmitCompositionError::new(kind, event)),
        };
        match self.queue.preflight_commit(1) {
            Ok(()) => {}
            Err(crate::queue::QueueCommitError::Full) => {
                return Err(SubmitCompositionError::new(
                    SubmitCompositionErrorKind::Full,
                    event,
                ));
            }
            Err(crate::queue::QueueCommitError::SequenceExhausted) => {
                return Err(SubmitCompositionError::new(
                    SubmitCompositionErrorKind::WorkSequenceExhausted,
                    event,
                ));
            }
        }
        let Some(reservation) = self.trace.reserve_input_outcome() else {
            return Err(SubmitCompositionError::new(
                SubmitCompositionErrorKind::TraceSequenceExhausted,
                event,
            ));
        };
        let sequence = self
            .queue
            .next_sequence()
            .unwrap_or_else(|| unreachable!("composition sequence was preflighted"));
        let instant = self.now();
        let payload_capture =
            if self.editing.sensitivity(&owner) == Some(runenui_core::TextSensitivity::Secret) {
                crate::TracePayloadCapture::Redacted
            } else {
                self.trace.payload_capture()
            };
        let (trace_kind, context) =
            Self::existing_composition_trace(&event, composition, payload_capture);
        let parent = self.trace.record_draft(
            TraceRecordDraft::input_fact(trace_kind, instant, TraceContext::input_record(context))
                .with_work_sequence(Some(sequence))
                .with_target(Some(self.tree.trace_target(&owner))),
        );
        if self.trace.is_enabled() && parent.is_none() {
            self.trace.release_reservation(reservation);
            return Err(SubmitCompositionError::new(
                SubmitCompositionErrorKind::TraceSequenceExhausted,
                event,
            ));
        }
        let sequence = self
            .queue
            .push_input_preflighted(
                owner,
                InputEnvelopePayload::Composition(event),
                instant,
                parent,
                reservation,
            )
            .unwrap_or_else(|_| unreachable!("composition queue was preflighted"));
        self.external_queue_commit_accepted();
        Ok(CompositionSubmission::new(sequence))
    }

    fn validate_existing_composition(
        &self,
        event: &CompositionEvent,
    ) -> Result<(crate::MountedNodeId, TraceCompositionContext), SubmitCompositionErrorKind> {
        match self.status {
            RuntimeStatus::Running => {}
            RuntimeStatus::Closed => return Err(SubmitCompositionErrorKind::Closed),
            RuntimeStatus::Terminal(reason) => {
                return Err(SubmitCompositionErrorKind::Terminal(reason));
            }
        }
        let generation = event.generation();
        if !self.tree.composition_generation_is_local(generation) {
            return Err(SubmitCompositionErrorKind::ForeignGeneration);
        }
        let Some(owner) = self.composition.owner().cloned() else {
            return Err(self.composition_generation_error_kind(generation));
        };
        if self.composition.generation() != Some(generation) {
            return Err(self.composition_generation_error_kind(generation));
        }
        let composition = self
            .composition
            .trace_context()
            .unwrap_or_else(|| unreachable!("accepted existing composition has identity"));
        Ok((owner, composition))
    }

    fn existing_composition_trace(
        event: &CompositionEvent,
        composition: TraceCompositionContext,
        payload_capture: crate::TracePayloadCapture,
    ) -> (TraceRecordKind, TraceInputContext) {
        match event {
            CompositionEvent::Update(update) => (
                TraceRecordKind::CompositionUpdateSubmitted,
                TraceInputContext::composition_update_with_capture(
                    composition,
                    update.preedit(),
                    update.range(),
                    payload_capture,
                ),
            ),
            CompositionEvent::End(_) => (
                TraceRecordKind::CompositionEndSubmitted,
                TraceInputContext::composition_identity(composition),
            ),
            CompositionEvent::Cancel(_) => (
                TraceRecordKind::CompositionCancelSubmitted,
                TraceInputContext::composition_identity(composition),
            ),
            CompositionEvent::Start(_) => unreachable!("existing composition excludes start"),
            _ => unreachable!("unknown composition event cannot be submitted"),
        }
    }

    fn composition_generation_was_issued(&self, generation: &CompositionGeneration) -> bool {
        self.last_issued_composition_generation
            .is_some_and(|last| generation.get() != 0 && generation.get() <= last.get())
    }

    fn composition_generation_error_kind(
        &self,
        generation: &CompositionGeneration,
    ) -> SubmitCompositionErrorKind {
        if self.composition_generation_was_issued(generation) {
            SubmitCompositionErrorKind::StaleGeneration
        } else {
            SubmitCompositionErrorKind::MissingGeneration
        }
    }

    fn input_target(&self) -> Result<crate::MountedNodeId, SubmitKeyboardErrorKind> {
        match self.status {
            RuntimeStatus::Running => {}
            RuntimeStatus::Closed => return Err(SubmitKeyboardErrorKind::Closed),
            RuntimeStatus::Terminal(reason) => {
                return Err(SubmitKeyboardErrorKind::Terminal(reason));
            }
        }
        if self.queue.is_full() {
            return Err(SubmitKeyboardErrorKind::Full);
        }
        if !self.queue.has_sequence() {
            return Err(SubmitKeyboardErrorKind::WorkSequenceExhausted);
        }
        self.focus
            .focused_node()
            .cloned()
            .ok_or(SubmitKeyboardErrorKind::NoFocusedTarget)
    }

    fn input_target_text(&mut self) -> Result<crate::MountedNodeId, SubmitTextErrorKind> {
        match self.status {
            RuntimeStatus::Running => {}
            RuntimeStatus::Closed => return Err(SubmitTextErrorKind::Closed),
            RuntimeStatus::Terminal(reason) => return Err(SubmitTextErrorKind::Terminal(reason)),
        }
        if self.queue.is_full() {
            return Err(SubmitTextErrorKind::Full);
        }
        if !self.queue.has_sequence() {
            return Err(SubmitTextErrorKind::WorkSequenceExhausted);
        }
        let target = self
            .focus
            .focused_node()
            .cloned()
            .ok_or(SubmitTextErrorKind::NoFocusedTarget)?;
        let capability = self
            .tree
            .text_input_probe(&target)
            .map_err(|_| SubmitTextErrorKind::FocusedTargetNotTextCapable)?;
        capability
            .accepts_committed_text()
            .then_some(())
            .ok_or(SubmitTextErrorKind::FocusedTargetNotTextCapable)?;
        if self.editing.has_owner(&target) {
            let reserved = self.queue.pending_committed_text_for(&target);
            self.editing
                .can_prepare_after_reserved(&target, reserved)
                .map_err(|error| match error {
                    crate::editing::EditPrepareError::Unavailable => {
                        SubmitTextErrorKind::EditingUnavailable
                    }
                    crate::editing::EditPrepareError::PendingCapacity => {
                        SubmitTextErrorKind::EditingCapacity
                    }
                    crate::editing::EditPrepareError::RequestExhausted => {
                        SubmitTextErrorKind::EditRequestExhausted
                    }
                    crate::editing::EditPrepareError::MissingOwner
                    | crate::editing::EditPrepareError::InvalidCoordinates => {
                        SubmitTextErrorKind::FocusedTargetNotTextCapable
                    }
                })?;
        }
        Ok(target)
    }

    fn commit_input(
        &mut self,
        target: crate::MountedNodeId,
        payload: InputEnvelopePayload,
    ) -> Result<WorkSequence, (SubmitKeyboardErrorKind, InputEnvelopePayload)> {
        let Some(reservation) = self.trace.reserve_input_outcome() else {
            return Err((SubmitKeyboardErrorKind::TraceSequenceExhausted, payload));
        };
        let Some(sequence) = self.queue.next_sequence() else {
            self.trace.release_reservation(reservation);
            return Err((SubmitKeyboardErrorKind::WorkSequenceExhausted, payload));
        };
        let instant = self.now();
        let payload_capture =
            if self.editing.sensitivity(&target) == Some(runenui_core::TextSensitivity::Secret) {
                crate::TracePayloadCapture::Redacted
            } else {
                self.trace.payload_capture()
            };
        let (trace_kind, context) = match &payload {
            InputEnvelopePayload::Keyboard(event) => (
                TraceRecordKind::KeyboardSubmissionAccepted,
                TraceInputContext::keyboard(event.device_id()),
            ),
            InputEnvelopePayload::CommittedText(event) => (
                TraceRecordKind::CommittedTextSubmissionAccepted,
                TraceInputContext::committed_text_with_capture(
                    event.text(),
                    event.device_id(),
                    payload_capture,
                ),
            ),
            InputEnvelopePayload::Composition(_) => {
                unreachable!("composition has dedicated ingress")
            }
        };
        let accepted = self.trace.record_draft(
            TraceRecordDraft::input_fact(trace_kind, instant, TraceContext::input_record(context))
                .with_work_sequence(Some(sequence))
                .with_target(Some(self.tree.trace_target(&target))),
        );
        if self.trace.is_enabled() && accepted.is_none() {
            self.trace.release_reservation(reservation);
            return Err((SubmitKeyboardErrorKind::TraceSequenceExhausted, payload));
        }
        let committed = self
            .queue
            .push_input_preflighted(target, payload, instant, accepted, reservation)
            .unwrap_or_else(|_| unreachable!("input queue was preflighted"));
        self.external_queue_commit_accepted();
        Ok(committed)
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn process_input_envelope(&mut self, envelope: InputEnvelope) -> crate::UiInputFinality {
        let InputEnvelope {
            sequence,
            target,
            payload,
            instant,
            causal_parent,
            trace_reservation,
        } = envelope;
        let origin = CommandOrigin::__runtime_keyboard();
        let event_context = Self::routed_input_event_context(&payload);
        let facts = RoutedIngressFacts::new(
            sequence,
            target.clone(),
            origin,
            instant,
            event_context,
            causal_parent,
            trace_reservation,
        );
        let shortcut_candidate = match &payload {
            InputEnvelopePayload::Keyboard(event) => {
                self.keyboard_shortcut_candidate(event, &target)
            }
            InputEnvelopePayload::CommittedText(_) | InputEnvelopePayload::Composition(_) => None,
        };
        let type_ahead_context = match &payload {
            InputEnvelopePayload::Keyboard(event) if shortcut_candidate.is_none() => {
                self.keyboard_type_ahead_context(event, &target)
            }
            InputEnvelopePayload::Keyboard(_)
            | InputEnvelopePayload::CommittedText(_)
            | InputEnvelopePayload::Composition(_) => None,
        };
        let mandatory_default_outputs = match &payload {
            InputEnvelopePayload::Keyboard(event) => {
                usize::from(self.keyboard_default_output_is_possible(
                    event,
                    &target,
                    shortcut_candidate.as_ref(),
                    type_ahead_context.as_ref(),
                ))
            }
            InputEnvelopePayload::CommittedText(_) => usize::from(self.editing.has_owner(&target)),
            InputEnvelopePayload::Composition(_) => 0,
        };
        let shortcut_trace = match &payload {
            InputEnvelopePayload::Keyboard(event)
                if shortcut_candidate.is_some()
                    && self.editor_owned_keyboard_default(event, &target).is_none() =>
            {
                MandatoryTracePlan::one_fact()
            }
            InputEnvelopePayload::Keyboard(_)
            | InputEnvelopePayload::CommittedText(_)
            | InputEnvelopePayload::Composition(_) => MandatoryTracePlan::none(),
        };
        let processing_trace = MandatoryTracePlan::input_processing()
            .checked_add(shortcut_trace)
            .unwrap_or_else(|| unreachable!("keyboard shortcut trace plan has a fixed bound"));
        if let InputEnvelopePayload::Composition(event) = &payload
            && !self.composition_processing_matches(&target, event)
        {
            self.trace.record_reserved_event(
                trace_reservation,
                TraceRecordKind::CompositionProcessingStaleGeneration,
                sequence,
                causal_parent,
                Some(self.tree.trace_target(&target)),
                instant,
                &target,
                None,
                CommandOrigin::__runtime_keyboard(),
            );
            return crate::UiInputFinality::ProcessingRejected(
                crate::UiInputProcessingRejection::StaleCompositionGeneration,
            );
        }
        let transaction_result = if type_ahead_context.is_some() {
            self.try_begin_focus_input_transaction(
                facts,
                processing_trace,
                mandatory_default_outputs,
            )
        } else {
            self.try_begin_routed_transaction_with_trace_and_default_outputs(
                facts,
                processing_trace,
                mandatory_default_outputs,
            )
        };
        let mut transaction = match transaction_result {
            Ok(transaction) => transaction,
            Err(failure) => {
                self.retire_failed_composition(
                    &target,
                    &payload,
                    sequence,
                    failure.causal_parent().or(causal_parent),
                    instant,
                );
                return match self.status() {
                    RuntimeStatus::Terminal(reason) => crate::UiInputFinality::Aborted(
                        crate::UiInputAbortReason::Terminal(reason),
                    ),
                    RuntimeStatus::Running | RuntimeStatus::Closed =>
                        crate::UiInputFinality::ProcessingRejected(
                            crate::UiInputProcessingRejection::InsufficientTransactionCapacity,
                        ),
                };
            }
        };
        self.record_input_processing_validation(&mut transaction, &payload);
        let event = match &payload {
            InputEnvelopePayload::Keyboard(event) => UiEvent::Keyboard(event.clone()),
            InputEnvelopePayload::CommittedText(event) => UiEvent::CommittedText(event.clone()),
            InputEnvelopePayload::Composition(event) => UiEvent::Composition(event.clone()),
        };
        if let Err(failure) = self.invoke_routed_callbacks(&mut transaction, &event, None) {
            let current = transaction.failure_current_target.clone();
            self.poison_transaction(&transaction, failure, current.as_ref());
            return crate::UiInputFinality::Aborted(crate::UiInputAbortReason::RuntimeIntegrity);
        }
        if let Err(failure) =
            self.collect_input_default(&mut transaction, &payload, shortcut_candidate.as_ref())
        {
            let current = transaction.failure_current_target.clone();
            self.poison_transaction(&transaction, failure, current.as_ref());
            return crate::UiInputFinality::Aborted(crate::UiInputAbortReason::RuntimeIntegrity);
        }
        // Capture committed routing/claim facts before the transaction is consumed.
        // A candidate shortcut alone does not prove an accepted shortcut default.
        let explicit = transaction.host_input_claimed;
        let has_default_output = !transaction.default_outputs.is_empty();
        let text_domain = matches!(payload, InputEnvelopePayload::CommittedText(_) | InputEnvelopePayload::Composition(_));
        let mut reasons = Vec::new();
        if explicit {
            reasons.push(crate::UiInputClaimReason::ExplicitWidgetClaim);
        }
        if text_domain {
            reasons.push(if matches!(payload, InputEnvelopePayload::Composition(_)) {
                crate::UiInputClaimReason::CompositionOwner
            } else {
                crate::UiInputClaimReason::TextOwner
            });
        }
        if has_default_output {
            reasons.push(crate::UiInputClaimReason::ActivationDefault);
        }
        let claims_ui = !reasons.is_empty();
        let routing = crate::UiInputRoutingFacts {
            conflict: if claims_ui {
                crate::UiInputConflict::ExclusiveUi
            } else {
                crate::UiInputConflict::ObservedNonexclusive
            },
            reasons,
            route: crate::UiInputRoute::Routed { target: target.clone() },
            propagation_stopped: transaction.propagation_stopped,
            default_prevented: transaction.default_prevented,
            default_disposition: if transaction.default_prevented {
                crate::UiDefaultDisposition::Prevented
            } else if has_default_output {
                crate::UiDefaultDisposition::Queued
            } else {
                crate::UiDefaultDisposition::None
            },
        };
        let completion_parent = transaction.parent;
        let completion_instant = transaction.instant;
        let completion_origin = transaction.origin;
        let failure_facts = transaction.failure_facts();
        if self.commit_routed_transaction(transaction).is_err() {
            self.poison_routed_event(
                &failure_facts,
                crate::TraceRoutedIntegrityFailure::CommitInvariantFailure,
                Some(&target),
            );
            return crate::UiInputFinality::Aborted(crate::UiInputAbortReason::RuntimeIntegrity);
        }
        match payload {
            InputEnvelopePayload::Composition(event) => {
                self.finish_composition_event(
                    &target,
                    &event,
                    sequence,
                    completion_parent,
                    completion_instant,
                    completion_origin,
                );
            }
            InputEnvelopePayload::Keyboard(_) | InputEnvelopePayload::CommittedText(_) => {}
        }
        crate::UiInputFinality::Committed(routing)
    }

    const fn routed_input_event_context(payload: &InputEnvelopePayload) -> TraceEventContext {
        match payload {
            InputEnvelopePayload::Keyboard(_) => {
                TraceEventContext::new(TraceEventFamily::Keyboard, true)
            }
            InputEnvelopePayload::CommittedText(_) => {
                TraceEventContext::new(TraceEventFamily::CommittedText, true)
            }
            InputEnvelopePayload::Composition(_) => {
                TraceEventContext::new(TraceEventFamily::Composition, false)
            }
        }
    }

    fn composition_processing_matches(
        &self,
        target: &crate::MountedNodeId,
        event: &CompositionEvent,
    ) -> bool {
        match (event, &self.composition) {
            (
                CompositionEvent::Start(start),
                CompositionState::Pending {
                    generation, owner, ..
                },
            ) => generation == start.generation() && owner == target,
            (
                CompositionEvent::Update(update),
                CompositionState::Pending {
                    generation, owner, ..
                }
                | CompositionState::Active {
                    generation, owner, ..
                },
            ) => generation == update.generation() && owner == target,
            (
                CompositionEvent::End(end),
                CompositionState::Pending {
                    generation, owner, ..
                }
                | CompositionState::Active {
                    generation, owner, ..
                },
            ) => generation == end.generation() && owner == target,
            (
                CompositionEvent::Cancel(cancel),
                CompositionState::Pending {
                    generation, owner, ..
                }
                | CompositionState::Active {
                    generation, owner, ..
                },
            ) => generation == cancel.generation() && owner == target,
            _ => false,
        }
    }

    fn record_input_processing_validation(
        &mut self,
        transaction: &mut crate::runtime::RoutedTransaction<Action>,
        payload: &InputEnvelopePayload,
    ) {
        let kind = match payload {
            InputEnvelopePayload::Keyboard(_) => TraceRecordKind::KeyboardProcessingValidated,
            InputEnvelopePayload::CommittedText(_) => {
                TraceRecordKind::CommittedTextProcessingValidated
            }
            InputEnvelopePayload::Composition(_) => TraceRecordKind::CompositionProcessingValidated,
        };
        transaction.parent = self.trace.record_event(
            kind,
            transaction.sequence,
            transaction.parent,
            Some(transaction.target_trace.clone()),
            transaction.instant,
            &transaction.target,
            Some(&transaction.target),
            transaction.origin,
        );
    }

    fn collect_input_default(
        &mut self,
        transaction: &mut crate::runtime::RoutedTransaction<Action>,
        payload: &InputEnvelopePayload,
        shortcut_candidate: Option<&KeyboardShortcutCandidate>,
    ) -> Result<(), crate::TraceRoutedIntegrityFailure> {
        if matches!(
            payload,
            InputEnvelopePayload::Composition(
                CompositionEvent::End(_) | CompositionEvent::Cancel(_)
            )
        ) && self
            .editing
            .clear_preedit(&transaction.target, payload_composition_generation(payload))
        {
            self.tree.mark_runtime_text_projection_dirty();
        }
        if transaction.default_prevented {
            let kind = match payload {
                InputEnvelopePayload::Keyboard(_) => TraceRecordKind::KeyboardDefaultPrevented,
                InputEnvelopePayload::CommittedText(_) => {
                    TraceRecordKind::CommittedTextDefaultPrevented
                }
                InputEnvelopePayload::Composition(_) => TraceRecordKind::DefaultPrevented,
            };
            transaction.parent = self.trace.record_event(
                kind,
                transaction.sequence,
                transaction.parent,
                Some(transaction.target_trace.clone()),
                transaction.instant,
                &transaction.target,
                Some(&transaction.target),
                transaction.origin,
            );
            return Ok(());
        }
        match payload {
            InputEnvelopePayload::Keyboard(keyboard) => {
                self.collect_keyboard_default(transaction, keyboard, shortcut_candidate)?;
            }
            InputEnvelopePayload::CommittedText(committed) => {
                self.collect_committed_text_default(transaction, committed)?;
            }
            InputEnvelopePayload::Composition(CompositionEvent::Update(update)) => {
                if self.editing.has_owner(&transaction.target) {
                    self.editing
                        .stage_preedit(
                            &transaction.target,
                            update.generation().clone(),
                            update.preedit(),
                            update.range(),
                        )
                        .map_err(|_| crate::TraceRoutedIntegrityFailure::CommitInvariantFailure)?;
                    self.tree.mark_runtime_text_projection_dirty();
                    transaction.invalidate(
                        runenui_core::WidgetInvalidation::LAYOUT
                            | runenui_core::WidgetInvalidation::PAINT
                            | runenui_core::WidgetInvalidation::SEMANTICS,
                    );
                }
            }
            InputEnvelopePayload::Composition(_) => {}
        }
        Ok(())
    }

    fn collect_committed_text_default(
        &mut self,
        transaction: &mut crate::runtime::RoutedTransaction<Action>,
        committed: &CommittedTextEvent,
    ) -> Result<(), crate::TraceRoutedIntegrityFailure> {
        let target = transaction.target.clone();
        if !self.editing.has_owner(&target) {
            return Ok(());
        }
        transaction.consume_mandatory_default_output()?;
        let composition = self
            .composition
            .owner()
            .filter(|owner| *owner == &target)
            .and_then(|_| self.composition.generation().cloned());
        let namespace = self.tree.runtime_namespace();
        let prepared = self
            .editing
            .prepare_insert(&namespace, &target, committed.text(), composition)
            .map_err(|_| crate::TraceRoutedIntegrityFailure::CommitInvariantFailure)?;
        if let Some(generation) = self.composition.generation().cloned()
            && self.editing.clear_preedit(&target, &generation)
        {
            self.tree.mark_runtime_text_projection_dirty();
        }
        transaction
            .default_outputs
            .push(crate::runtime::CollectedRoutedOutput::EditAction {
                action: prepared.action,
                origin: prepared.origin,
                causal_parent: transaction.parent,
                current_target: target,
            });
        Ok(())
    }

    fn retire_failed_composition(
        &mut self,
        target: &crate::MountedNodeId,
        payload: &InputEnvelopePayload,
        sequence: WorkSequence,
        causal_parent: Option<crate::TraceSequence>,
        instant: MonotonicInstant,
    ) {
        let InputEnvelopePayload::Composition(event) = payload else {
            return;
        };
        if self.composition.generation() != Some(event.generation())
            || self.composition.owner() != Some(target)
        {
            return;
        }
        let composition = self
            .composition
            .trace_context()
            .unwrap_or_else(|| unreachable!("retired composition retains exact identity"));
        self.composition = CompositionState::None;
        self.trace.record_draft(
            TraceRecordDraft::input_fact(
                TraceRecordKind::CompositionRetired,
                instant,
                TraceContext::input_record(TraceInputContext::composition_cleanup(
                    composition,
                    TraceDeliveryOutcome::Suppressed,
                )),
            )
            .with_work_sequence(Some(sequence))
            .with_causal_parent(causal_parent)
            .with_target(Some(self.tree.trace_target(target))),
        );
    }

    fn finish_composition_event(
        &mut self,
        target: &crate::MountedNodeId,
        event: &CompositionEvent,
        sequence: WorkSequence,
        causal_parent: Option<crate::TraceSequence>,
        instant: MonotonicInstant,
        origin: CommandOrigin,
    ) {
        let generation = event.generation();
        if self.composition.generation() != Some(generation)
            || self.composition.owner() != Some(target)
        {
            return;
        }
        match event {
            CompositionEvent::Start(_) => {
                let state = core::mem::replace(&mut self.composition, CompositionState::None);
                self.composition = match state {
                    CompositionState::Pending {
                        generation,
                        owner,
                        device_id,
                        start_sequence,
                    } => {
                        self.trace.record_event(
                            TraceRecordKind::CompositionActiveBound,
                            sequence,
                            causal_parent,
                            Some(self.tree.trace_target(&owner)),
                            instant,
                            target,
                            Some(&owner),
                            origin,
                        );
                        CompositionState::Active {
                            generation,
                            owner,
                            device_id,
                            start_sequence,
                        }
                    }
                    other => other,
                };
            }
            CompositionEvent::End(_) => {
                self.composition = CompositionState::None;
                self.trace.record_event(
                    TraceRecordKind::CompositionRetired,
                    sequence,
                    causal_parent,
                    Some(self.tree.trace_target(target)),
                    instant,
                    target,
                    Some(target),
                    origin,
                );
            }
            CompositionEvent::Cancel(cancel) => {
                let composition = self
                    .composition
                    .trace_context()
                    .unwrap_or_else(|| unreachable!("completed cancellation retains identity"));
                self.composition = CompositionState::None;
                let cancelled = self.trace.record_draft(
                    TraceRecordDraft::input_fact(
                        TraceRecordKind::CompositionCancelled {
                            reason: cancel.reason(),
                        },
                        instant,
                        TraceContext::input_record(TraceInputContext::composition_cleanup(
                            composition,
                            TraceDeliveryOutcome::Delivered,
                        )),
                    )
                    .with_work_sequence(Some(sequence))
                    .with_causal_parent(causal_parent)
                    .with_target(Some(self.tree.trace_target(target)))
                    .with_routed_endpoints(
                        target.clone(),
                        Some(target.clone()),
                        origin,
                    ),
                );
                self.trace.record_draft(
                    TraceRecordDraft::input_marker(TraceRecordKind::CompositionRetired, instant)
                        .with_work_sequence(Some(sequence))
                        .with_causal_parent(cancelled)
                        .with_target(Some(self.tree.trace_target(target))),
                );
            }
            _ => {}
        }
    }

    fn keyboard_shortcut_candidate(
        &self,
        event: &KeyboardEvent,
        target: &crate::MountedNodeId,
    ) -> Option<KeyboardShortcutCandidate> {
        if event.phase() != KeyboardPhase::Down
            || matches!(event.physical_key(), PhysicalKey::Space)
        {
            return None;
        }
        let route = self.tree.event_route(target).ok()?;
        for scope in route.into_iter().rev() {
            let bindings = self.tree.shortcut_bindings(&scope)?;
            let mut matching = bindings
                .iter()
                .filter(|binding| binding.chord().matches(event));
            let Some(binding) = matching.next() else {
                continue;
            };
            if matching.next().is_some() {
                return Some(KeyboardShortcutCandidate::Ambiguous { scope });
            }
            return Some(KeyboardShortcutCandidate::Unique {
                scope,
                binding: binding.clone(),
            });
        }
        None
    }

    fn editor_owned_keyboard_default(
        &self,
        event: &KeyboardEvent,
        target: &crate::MountedNodeId,
    ) -> Option<SemanticCommand> {
        if event.phase() != KeyboardPhase::Down || !self.editing.has_owner(target) {
            return None;
        }
        match event.logical_key() {
            LogicalKey::Command(_) if event.is_repeat() => None,
            LogicalKey::Backspace => Some(SemanticCommand::DeleteBackward),
            LogicalKey::Delete => Some(SemanticCommand::DeleteForward),
            LogicalKey::Command(command)
                if matches!(
                    command,
                    SemanticCommand::SelectAll
                        | SemanticCommand::Copy
                        | SemanticCommand::Cut
                        | SemanticCommand::Paste
                        | SemanticCommand::Undo
                        | SemanticCommand::Redo
                ) =>
            {
                Some(*command)
            }
            LogicalKey::ArrowLeft if event.modifiers().shift() => {
                Some(SemanticCommand::ExtendBackward)
            }
            LogicalKey::ArrowRight if event.modifiers().shift() => {
                Some(SemanticCommand::ExtendForward)
            }
            LogicalKey::ArrowLeft => Some(SemanticCommand::MoveBackward),
            LogicalKey::ArrowRight => Some(SemanticCommand::MoveForward),
            LogicalKey::ArrowUp if event.modifiers().shift() => Some(SemanticCommand::ExtendUp),
            LogicalKey::ArrowDown if event.modifiers().shift() => Some(SemanticCommand::ExtendDown),
            LogicalKey::ArrowUp => Some(SemanticCommand::MoveUp),
            LogicalKey::ArrowDown => Some(SemanticCommand::MoveDown),
            LogicalKey::Escape if self.editing.has_stable_range_selection(target) => {
                Some(SemanticCommand::MoveBackward)
            }
            _ => None,
        }
    }

    const fn generic_keyboard_default(event: &KeyboardEvent) -> Option<SemanticCommand> {
        if !matches!(event.phase(), KeyboardPhase::Down) {
            return None;
        }
        match event.logical_key() {
            LogicalKey::Tab if event.modifiers().shift() => Some(SemanticCommand::FocusPrevious),
            LogicalKey::Tab => Some(SemanticCommand::FocusNext),
            LogicalKey::ArrowLeft => Some(SemanticCommand::FocusLeft),
            LogicalKey::ArrowRight => Some(SemanticCommand::FocusRight),
            LogicalKey::ArrowUp => Some(SemanticCommand::FocusUp),
            LogicalKey::ArrowDown => Some(SemanticCommand::FocusDown),
            LogicalKey::Escape => Some(SemanticCommand::CancelOrBack),
            LogicalKey::Enter if !event.is_repeat() => Some(SemanticCommand::Activate),
            _ => None,
        }
    }

    fn keyboard_default_output_is_possible(
        &self,
        event: &KeyboardEvent,
        target: &crate::MountedNodeId,
        shortcut_candidate: Option<&KeyboardShortcutCandidate>,
        type_ahead_context: Option<&crate::focus::FocusGroupTypeAheadContext>,
    ) -> bool {
        if matches!(event.physical_key(), PhysicalKey::Space) {
            return matches!(event.phase(), KeyboardPhase::Up) && !event.is_repeat();
        }
        if self.editor_owned_keyboard_default(event, target).is_some() {
            return true;
        }
        if event.composition_state() == KeyboardCompositionState::Active {
            return Self::generic_keyboard_default(event)
                .is_some_and(|command| command != SemanticCommand::CancelOrBack);
        }
        if let Some(candidate) = shortcut_candidate {
            return candidate.reserves_default_output();
        }
        type_ahead_context
            .is_some_and(|context| context.activation == FocusGroupActivationPolicy::ActivateTarget)
            || Self::generic_keyboard_default(event).is_some()
    }

    fn collect_keyboard_default(
        &mut self,
        transaction: &mut crate::runtime::RoutedTransaction<Action>,
        event: &KeyboardEvent,
        shortcut_candidate: Option<&KeyboardShortcutCandidate>,
    ) -> Result<(), crate::TraceRoutedIntegrityFailure> {
        let target = transaction.target.clone();
        if matches!(event.phase(), KeyboardPhase::Cancel) {
            if matches!(event.physical_key(), PhysicalKey::Space)
                && self.space_ownership.as_ref().is_some_and(|ownership| {
                    ownership.target == target && ownership.device_id == event.device_id()
                })
            {
                self.revoke_space_ownership_in_transaction(
                    transaction,
                    TraceSpaceCleanupReason::KeyboardCancel,
                );
            }
            return Ok(());
        }
        if matches!(event.physical_key(), PhysicalKey::Space) {
            match event.phase() {
                KeyboardPhase::Down
                    if !event.is_repeat()
                        && self.space_ownership.is_none()
                        && self.keyboard_activation_eligible(&target) =>
                {
                    self.space_ownership = Some(SpaceOwnership {
                        target: target.clone(),
                        device_id: event.device_id(),
                        down_sequence: transaction.sequence,
                    });
                    transaction.parent = self.trace.record_event(
                        TraceRecordKind::KeyboardSpaceOwnershipEstablished,
                        transaction.sequence,
                        transaction.parent,
                        Some(self.tree.trace_target(&target)),
                        transaction.instant,
                        &transaction.target,
                        Some(&target),
                        transaction.origin,
                    );
                }
                KeyboardPhase::Up => {
                    let eligible = self.keyboard_activation_eligible(&target);
                    let matches = !event.is_repeat()
                        && self.space_ownership.as_ref().is_some_and(|owner| {
                            owner.target == target
                                && owner.device_id == event.device_id()
                                && owner.down_sequence.get() > 0
                                && eligible
                        });
                    transaction.parent = self.trace.record_event(
                        TraceRecordKind::KeyboardSpaceReleaseMatched { matched: matches },
                        transaction.sequence,
                        transaction.parent,
                        Some(self.tree.trace_target(&target)),
                        transaction.instant,
                        &transaction.target,
                        Some(&target),
                        transaction.origin,
                    );
                    if matches {
                        self.revoke_space_ownership_in_transaction(
                            transaction,
                            TraceSpaceCleanupReason::Release,
                        );
                        transaction.parent = self.trace.record_event(
                            TraceRecordKind::KeyboardSpaceActivationDerived,
                            transaction.sequence,
                            transaction.parent,
                            Some(self.tree.trace_target(&target)),
                            transaction.instant,
                            &transaction.target,
                            Some(&target),
                            transaction.origin,
                        );
                        Self::collect_keyboard_default_command(
                            transaction,
                            target,
                            SemanticCommand::Activate,
                        )?;
                    }
                }
                _ => {}
            }
            return Ok(());
        }
        self.collect_non_space_keyboard_default(transaction, event, target, shortcut_candidate)
    }

    fn collect_non_space_keyboard_default(
        &mut self,
        transaction: &mut crate::runtime::RoutedTransaction<Action>,
        event: &KeyboardEvent,
        target: crate::MountedNodeId,
        shortcut_candidate: Option<&KeyboardShortcutCandidate>,
    ) -> Result<(), crate::TraceRoutedIntegrityFailure> {
        if event.phase() != KeyboardPhase::Down {
            return Ok(());
        }
        if let Some(command) = self.editor_owned_keyboard_default(event, &target) {
            Self::collect_keyboard_default_command(transaction, target, command)?;
            return Ok(());
        }
        if self.collect_keyboard_shortcut_default(transaction, event, shortcut_candidate)? {
            return Ok(());
        }
        if self.collect_focus_group_type_ahead_default(transaction, event, &target)? {
            return Ok(());
        }
        let command = Self::generic_keyboard_default(event);
        if event.composition_state() == KeyboardCompositionState::Active
            && command == Some(SemanticCommand::CancelOrBack)
        {
            return Ok(());
        }
        if matches!(command, Some(SemanticCommand::Activate))
            && !self.keyboard_activation_eligible(&target)
        {
            return Ok(());
        }
        if matches!(command, Some(SemanticCommand::Activate)) {
            transaction.parent = self.trace.record_event(
                TraceRecordKind::KeyboardEnterActivationDerived,
                transaction.sequence,
                transaction.parent,
                Some(self.tree.trace_target(&target)),
                transaction.instant,
                &transaction.target,
                Some(&target),
                transaction.origin,
            );
        }
        if let Some(command) = command {
            Self::collect_keyboard_default_command(transaction, target, command)?;
        }
        Ok(())
    }

    fn collect_keyboard_shortcut_default(
        &mut self,
        transaction: &mut crate::runtime::RoutedTransaction<Action>,
        event: &KeyboardEvent,
        candidate: Option<&KeyboardShortcutCandidate>,
    ) -> Result<bool, crate::TraceRoutedIntegrityFailure> {
        let Some(candidate) = candidate else {
            return Ok(false);
        };
        if event.composition_state() == KeyboardCompositionState::Active {
            let scope = match candidate {
                KeyboardShortcutCandidate::Unique { scope, .. }
                | KeyboardShortcutCandidate::Ambiguous { scope } => scope,
            };
            self.record_keyboard_shortcut_outcome(
                transaction,
                scope,
                TraceRecordKind::KeyboardShortcutCompositionSuppressed,
            );
            return Ok(false);
        }
        let (scope, binding) = match candidate {
            KeyboardShortcutCandidate::Ambiguous { scope } => {
                self.record_keyboard_shortcut_outcome(
                    transaction,
                    scope,
                    TraceRecordKind::KeyboardShortcutAmbiguous,
                );
                return Ok(true);
            }
            KeyboardShortcutCandidate::Unique { scope, binding } => (scope, binding),
        };
        if !binding.command().enabled() {
            self.record_keyboard_shortcut_outcome(
                transaction,
                scope,
                TraceRecordKind::KeyboardShortcutDisabled,
            );
            return Ok(true);
        }
        if !binding.repeat_policy().allows(event.is_repeat()) {
            self.record_keyboard_shortcut_outcome(
                transaction,
                scope,
                TraceRecordKind::KeyboardShortcutRepeatSuppressed,
            );
            return Ok(true);
        }
        if let Err(failure) = transaction.consume_mandatory_default_output() {
            transaction.failure_current_target = Some(scope.clone());
            return Err(failure);
        }
        self.record_keyboard_shortcut_outcome(
            transaction,
            scope,
            TraceRecordKind::KeyboardShortcutMatched,
        );
        transaction.default_outputs.push(
            crate::runtime::CollectedRoutedOutput::ApplicationCommand {
                target: transaction.target.clone(),
                command: binding.command().id().clone(),
                origin: CommandOrigin::__runtime_keyboard_default(),
                causal_parent: transaction.parent,
            },
        );
        Ok(true)
    }

    fn record_keyboard_shortcut_outcome(
        &mut self,
        transaction: &mut crate::runtime::RoutedTransaction<Action>,
        scope: &crate::MountedNodeId,
        kind: TraceRecordKind,
    ) {
        transaction.parent = self.trace.record_event(
            kind,
            transaction.sequence,
            transaction.parent,
            Some(transaction.target_trace.clone()),
            transaction.instant,
            &transaction.target,
            Some(scope),
            transaction.origin,
        );
    }

    fn collect_keyboard_default_command(
        transaction: &mut crate::runtime::RoutedTransaction<Action>,
        target: crate::MountedNodeId,
        command: SemanticCommand,
    ) -> Result<(), crate::TraceRoutedIntegrityFailure> {
        if let Err(failure) = transaction.consume_mandatory_default_output() {
            transaction.failure_current_target = Some(target);
            return Err(failure);
        }
        transaction
            .default_outputs
            .push(crate::runtime::CollectedRoutedOutput::Command {
                target,
                command,
                origin: CommandOrigin::__runtime_keyboard_default(),
                causal_parent: transaction.parent,
            });
        Ok(())
    }

    fn revoke_space_ownership_in_transaction(
        &mut self,
        transaction: &mut crate::runtime::RoutedTransaction<Action>,
        reason: TraceSpaceCleanupReason,
    ) {
        let Some(ownership) = self.space_ownership.take() else {
            return;
        };
        transaction.parent = self.trace.record_event(
            TraceRecordKind::KeyboardSpaceOwnershipCleared { reason },
            transaction.sequence,
            transaction.parent,
            Some(self.tree.trace_target(&ownership.target)),
            transaction.instant,
            &transaction.target,
            Some(&ownership.target),
            transaction.origin,
        );
    }

    fn keyboard_activation_eligible(&mut self, target: &crate::MountedNodeId) -> bool {
        self.focus.focused_node() == Some(target)
            && self.tree.target_status(target) == TargetStatus::Live
            && self
                .tree
                .activation_probe(target)
                .is_ok_and(|activation| activation.enabled() && activation.is_actionable())
    }
}

fn payload_composition_generation(payload: &InputEnvelopePayload) -> &CompositionGeneration {
    match payload {
        InputEnvelopePayload::Composition(event) => event.generation(),
        InputEnvelopePayload::Keyboard(_) | InputEnvelopePayload::CommittedText(_) => {
            unreachable!("caller proved composition payload")
        }
    }
}
