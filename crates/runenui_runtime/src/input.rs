//! Public input receipts and canonical keyboard/text ingress.

use core::fmt;
use runenui_core::{
    CommandOrigin, CommittedTextEvent, CompositionCancel, CompositionCancelReason, CompositionEnd,
    CompositionEvent, CompositionGeneration, CompositionRange, CompositionStart, CompositionUpdate,
    ElementId, HostProtocol, InputDeviceId, KeyboardEvent, KeyboardPhase, LogicalKey, PhysicalKey,
    SemanticCommand, UiEvent, WorkSequence,
};

use crate::{
    RuntimeStatus, RuntimeTerminalReason, TraceRecordKind,
    mounted::{AutomationResolution, TargetStatus},
    queue::{InputEnvelope, InputEnvelopePayload},
    runtime::{RoutedIngressFacts, Runtime},
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

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitTextErrorKind {
    Full,
    Closed,
    Terminal(RuntimeTerminalReason),
    NoFocusedTarget,
    FocusedTargetNotTextCapable,
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
        _device_id: Option<InputDeviceId>,
        _start_sequence: WorkSequence,
    },
}

impl CompositionState {
    const fn generation(&self) -> Option<&CompositionGeneration> {
        match self {
            Self::None => None,
            Self::Pending { generation, .. } | Self::Active { generation, .. } => Some(generation),
        }
    }

    const fn owner(&self) -> Option<&crate::MountedNodeId> {
        match self {
            Self::None => None,
            Self::Pending { owner, .. } | Self::Active { owner, .. } => Some(owner),
        }
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitAutomationErrorKind {
    MissingAuthoredId,
    AmbiguousAuthoredId { matches: usize },
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
    pub const fn kind(&self) -> SubmitAutomationErrorKind {
        self.kind
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
    pub(crate) fn retire_composition_for_owner(
        &mut self,
        owner: &crate::MountedNodeId,
        reason: CompositionCancelReason,
    ) {
        if self.composition.owner() != Some(owner) {
            return;
        }
        self.composition = CompositionState::None;
        self.trace.record(
            TraceRecordKind::CompositionCancelled { reason },
            None,
            None,
            None,
            None,
            Some(self.tree.trace_target(owner)),
        );
        self.trace.record(
            TraceRecordKind::CompositionRetired,
            None,
            None,
            None,
            None,
            Some(self.tree.trace_target(owner)),
        );
    }

    pub(crate) fn retire_composition_for_terminal(&mut self, reason: CompositionCancelReason) {
        let owner = self.composition.owner().cloned();
        if let Some(owner) = owner {
            self.retire_composition_for_owner(&owner, reason);
        }
    }

    pub(crate) fn submit_automation_command(
        &mut self,
        authored_id: ElementId,
        command: SemanticCommand,
    ) -> Result<AutomationSubmission, SubmitAutomationError> {
        let (target, resolution_parent) = match self.tree.resolve_authored_id(&authored_id) {
            AutomationResolution::Missing => {
                self.trace.record(
                    TraceRecordKind::AutomationResolutionMissing,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
                return Err(SubmitAutomationError::new(
                    SubmitAutomationErrorKind::MissingAuthoredId,
                    authored_id,
                    command,
                ));
            }
            AutomationResolution::Unique(target) => {
                let parent = self.trace.record(
                    TraceRecordKind::AutomationResolutionUnique,
                    None,
                    None,
                    None,
                    None,
                    Some(self.tree.trace_target(&target)),
                );
                (target, parent)
            }
            AutomationResolution::Ambiguous { matches } => {
                self.trace.record(
                    TraceRecordKind::AutomationResolutionAmbiguous { matches },
                    None,
                    None,
                    None,
                    None,
                    None,
                );
                return Err(SubmitAutomationError::new(
                    SubmitAutomationErrorKind::AmbiguousAuthoredId { matches },
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
            .input_target(&event, false)
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
            .input_target_text(&event)
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
    ) -> Result<CompositionStartSubmission, SubmitCompositionError> {
        let target = self.composition_start_target()?;
        let Some(next) = self.next_composition_generation else {
            return Err(SubmitCompositionError::new(
                SubmitCompositionErrorKind::CompositionGenerationExhausted,
                CompositionEvent::Start(CompositionStart::__runtime_new(
                    self.tree.composition_generation(u64::MAX),
                    device_id,
                )),
            ));
        };
        let Some(reservation) = self.trace.reserve_input_outcome() else {
            return Err(self.composition_start_error(
                SubmitCompositionErrorKind::TraceSequenceExhausted,
                device_id,
            ));
        };
        let Some(sequence) = self.queue.next_sequence() else {
            self.trace.release_reservation(reservation);
            return Err(self.composition_start_error(
                SubmitCompositionErrorKind::WorkSequenceExhausted,
                device_id,
            ));
        };
        let generation = self.tree.composition_generation(next.get());
        let event = CompositionEvent::Start(CompositionStart::__runtime_new(
            generation.clone(),
            device_id,
        ));
        let accepted = self.trace.record(
            TraceRecordKind::CompositionGenerationAllocated,
            Some(sequence),
            None,
            None,
            None,
            Some(self.tree.trace_target(&target)),
        );
        if self.trace.is_enabled() && accepted.is_none() {
            self.trace.release_reservation(reservation);
            return Err(SubmitCompositionError::new(
                SubmitCompositionErrorKind::TraceSequenceExhausted,
                event,
            ));
        }
        self.composition = CompositionState::Pending {
            generation: generation.clone(),
            owner: target.clone(),
            device_id,
            start_sequence: sequence,
        };
        self.next_composition_generation = next
            .get()
            .checked_add(1)
            .and_then(core::num::NonZeroU64::new);
        let committed = self
            .queue
            .push_input_preflighted(
                target,
                InputEnvelopePayload::Composition(event),
                self.now(),
                accepted,
                reservation,
            )
            .unwrap_or_else(|_| unreachable!("composition queue was preflighted"));
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

    fn composition_start_target(&mut self) -> Result<crate::MountedNodeId, SubmitCompositionError> {
        match self.status {
            RuntimeStatus::Running => {}
            RuntimeStatus::Closed => {
                return Err(self.composition_start_error(SubmitCompositionErrorKind::Closed, None));
            }
            RuntimeStatus::Terminal(reason) => {
                return Err(self
                    .composition_start_error(SubmitCompositionErrorKind::Terminal(reason), None));
            }
        }
        if !matches!(self.composition, CompositionState::None) {
            return Err(
                self.composition_start_error(SubmitCompositionErrorKind::StaleGeneration, None)
            );
        }
        match self.queue.preflight_commit(1) {
            Ok(()) => {}
            Err(crate::queue::QueueCommitError::Full) => {
                return Err(self.composition_start_error(SubmitCompositionErrorKind::Full, None));
            }
            Err(crate::queue::QueueCommitError::SequenceExhausted) => {
                return Err(self.composition_start_error(
                    SubmitCompositionErrorKind::WorkSequenceExhausted,
                    None,
                ));
            }
        }
        let target = self.focus.focused_node().cloned().ok_or_else(|| {
            self.composition_start_error(SubmitCompositionErrorKind::NoFocusedTarget, None)
        })?;
        let capability = self.tree.text_input_probe(&target).map_err(|_| {
            self.composition_start_error(
                SubmitCompositionErrorKind::FocusedTargetNotCompositionCapable,
                None,
            )
        })?;
        capability
            .accepts_composition()
            .then_some(target)
            .ok_or_else(|| {
                self.composition_start_error(
                    SubmitCompositionErrorKind::FocusedTargetNotCompositionCapable,
                    None,
                )
            })
    }

    fn composition_start_error(
        &self,
        kind: SubmitCompositionErrorKind,
        device_id: Option<InputDeviceId>,
    ) -> SubmitCompositionError {
        SubmitCompositionError::new(
            kind,
            CompositionEvent::Start(CompositionStart::__runtime_new(
                self.tree.composition_generation(0),
                device_id,
            )),
        )
    }

    fn submit_existing_composition(
        &mut self,
        event: CompositionEvent,
    ) -> Result<CompositionSubmission, SubmitCompositionError> {
        match self.status {
            RuntimeStatus::Running => {}
            RuntimeStatus::Closed => {
                return Err(SubmitCompositionError::new(
                    SubmitCompositionErrorKind::Closed,
                    event,
                ));
            }
            RuntimeStatus::Terminal(reason) => {
                return Err(SubmitCompositionError::new(
                    SubmitCompositionErrorKind::Terminal(reason),
                    event,
                ));
            }
        }
        let generation = event.generation();
        if !self.tree.composition_generation_is_local(generation) {
            return Err(SubmitCompositionError::new(
                SubmitCompositionErrorKind::ForeignGeneration,
                event,
            ));
        }
        let Some(owner) = self.composition.owner().cloned() else {
            return Err(SubmitCompositionError::new(
                SubmitCompositionErrorKind::MissingGeneration,
                event,
            ));
        };
        if self.composition.generation() != Some(generation) {
            return Err(SubmitCompositionError::new(
                SubmitCompositionErrorKind::StaleGeneration,
                event,
            ));
        }
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
        let trace_kind = match &event {
            CompositionEvent::Update(update) => TraceRecordKind::CompositionUpdated {
                has_range: update.range().is_some(),
            },
            CompositionEvent::End(_) => TraceRecordKind::CompositionEnded,
            CompositionEvent::Cancel(cancel) => TraceRecordKind::CompositionCancelled {
                reason: cancel.reason(),
            },
            CompositionEvent::Start(_) => unreachable!("existing composition excludes start"),
            _ => unreachable!("unknown composition event cannot be submitted"),
        };
        let parent = self.trace.record(
            trace_kind,
            Some(sequence),
            None,
            None,
            None,
            Some(self.tree.trace_target(&owner)),
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
                self.now(),
                parent,
                reservation,
            )
            .unwrap_or_else(|_| unreachable!("composition queue was preflighted"));
        self.external_queue_commit_accepted();
        Ok(CompositionSubmission::new(sequence))
    }

    fn input_target(
        &self,
        _event: &KeyboardEvent,
        _: bool,
    ) -> Result<crate::MountedNodeId, SubmitKeyboardErrorKind> {
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

    fn input_target_text(
        &mut self,
        _: &CommittedTextEvent,
    ) -> Result<crate::MountedNodeId, SubmitTextErrorKind> {
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
            .then_some(target)
            .ok_or(SubmitTextErrorKind::FocusedTargetNotTextCapable)
    }

    fn commit_input(
        &mut self,
        target: crate::MountedNodeId,
        payload: InputEnvelopePayload,
    ) -> Result<WorkSequence, (SubmitKeyboardErrorKind, InputEnvelopePayload)> {
        // The routed event reservation supplies bounded processing admission. No raw text is recorded.
        let Some(reservation) = self.trace.reserve_input_outcome() else {
            return Err((SubmitKeyboardErrorKind::TraceSequenceExhausted, payload));
        };
        let Some(sequence) = self.queue.next_sequence() else {
            self.trace.release_reservation(reservation);
            return Err((SubmitKeyboardErrorKind::WorkSequenceExhausted, payload));
        };
        let trace_kind = match &payload {
            InputEnvelopePayload::Keyboard(_) => TraceRecordKind::KeyboardSubmissionAccepted,
            InputEnvelopePayload::CommittedText(event) => {
                TraceRecordKind::CommittedTextSubmissionAccepted {
                    bytes: event.text().len(),
                    scalars: event.text().chars().count(),
                }
            }
            InputEnvelopePayload::Composition(_) => {
                unreachable!("composition has dedicated ingress")
            }
        };
        let accepted = self
            .trace
            .record(trace_kind, Some(sequence), None, None, None, None);
        if self.trace.is_enabled() && accepted.is_none() {
            self.trace.release_reservation(reservation);
            return Err((SubmitKeyboardErrorKind::TraceSequenceExhausted, payload));
        }
        let instant = self.now();
        let event = payload;
        let committed = self
            .queue
            .push_input_preflighted(target, event, instant, accepted, reservation)
            .unwrap_or_else(|_| unreachable!("input queue was preflighted"));
        self.external_queue_commit_accepted();
        Ok(committed)
    }

    pub(crate) fn process_input_envelope(&mut self, envelope: InputEnvelope) {
        let InputEnvelope {
            sequence,
            target,
            payload,
            instant,
            causal_parent,
            trace_reservation,
        } = envelope;
        let origin = CommandOrigin::__runtime_keyboard();
        let facts = RoutedIngressFacts::new(
            sequence,
            target.clone(),
            origin,
            instant,
            causal_parent,
            trace_reservation,
        );
        let Some(mut transaction) = self.begin_routed_transaction(facts) else {
            if let InputEnvelopePayload::Composition(event) = payload
                && self.composition.generation() == Some(event.generation())
            {
                self.composition = CompositionState::None;
                self.trace.record(
                    TraceRecordKind::CompositionRetired,
                    Some(sequence),
                    causal_parent,
                    None,
                    None,
                    None,
                );
            }
            return;
        };
        let event = match &payload {
            InputEnvelopePayload::Keyboard(event) => UiEvent::Keyboard(event.clone()),
            InputEnvelopePayload::CommittedText(event) => UiEvent::CommittedText(event.clone()),
            InputEnvelopePayload::Composition(event) => UiEvent::Composition(event.clone()),
        };
        if self
            .invoke_routed_callbacks(&mut transaction, &event, None)
            .is_err()
        {
            if let InputEnvelopePayload::Composition(event) = payload
                && self.composition.generation() == Some(event.generation())
            {
                self.composition = CompositionState::None;
            }
            return;
        }
        let default_prevented = transaction.default_prevented;
        if self.commit_routed_transaction(transaction).is_err() {
            if let InputEnvelopePayload::Composition(event) = payload
                && self.composition.generation() == Some(event.generation())
            {
                self.composition = CompositionState::None;
            }
            return;
        }
        match payload {
            InputEnvelopePayload::Keyboard(keyboard) if !default_prevented => {
                self.derive_keyboard_default(&target, &keyboard, sequence);
            }
            InputEnvelopePayload::Composition(event) => {
                self.finish_composition_event(&target, &event);
            }
            InputEnvelopePayload::Keyboard(_) | InputEnvelopePayload::CommittedText(_) => {}
        }
    }

    fn finish_composition_event(
        &mut self,
        target: &crate::MountedNodeId,
        event: &CompositionEvent,
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
                        self.trace.record(
                            TraceRecordKind::CompositionActiveBound,
                            None,
                            None,
                            None,
                            None,
                            Some(self.tree.trace_target(&owner)),
                        );
                        CompositionState::Active {
                            generation,
                            owner,
                            _device_id: device_id,
                            _start_sequence: start_sequence,
                        }
                    }
                    other => other,
                };
            }
            CompositionEvent::End(_) | CompositionEvent::Cancel(_) => {
                self.composition = CompositionState::None;
                self.trace.record(
                    TraceRecordKind::CompositionRetired,
                    None,
                    None,
                    None,
                    None,
                    Some(self.tree.trace_target(target)),
                );
            }
            _ => {}
        }
    }

    fn derive_keyboard_default(
        &mut self,
        target: &crate::MountedNodeId,
        event: &KeyboardEvent,
        sequence: WorkSequence,
    ) {
        if matches!(event.phase(), KeyboardPhase::Cancel) {
            self.space_ownership = None;
            return;
        }
        if matches!(event.physical_key(), PhysicalKey::Space) {
            match event.phase() {
                KeyboardPhase::Down
                    if !event.is_repeat() && self.keyboard_activation_eligible(target) =>
                {
                    self.space_ownership = Some(SpaceOwnership {
                        target: target.clone(),
                        device_id: event.device_id(),
                        down_sequence: sequence,
                    });
                }
                KeyboardPhase::Up => {
                    let eligible = self.keyboard_activation_eligible(target);
                    let matches = self.space_ownership.as_ref().is_some_and(|owner| {
                        owner.target == *target
                            && owner.device_id == event.device_id()
                            && owner.down_sequence.get() > 0
                            && eligible
                    });
                    self.space_ownership = None;
                    if matches {
                        let _ = self.submit_command(
                            target.clone(),
                            SemanticCommand::Activate,
                            CommandOrigin::__runtime_keyboard_default(),
                        );
                    }
                }
                _ => {}
            }
            return;
        }
        if event.phase() != KeyboardPhase::Down {
            return;
        }
        let command = match event.logical_key() {
            LogicalKey::Tab if event.modifiers().shift() => Some(SemanticCommand::FocusPrevious),
            LogicalKey::Tab => Some(SemanticCommand::FocusNext),
            LogicalKey::ArrowLeft => Some(SemanticCommand::FocusLeft),
            LogicalKey::ArrowRight => Some(SemanticCommand::FocusRight),
            LogicalKey::ArrowUp => Some(SemanticCommand::FocusUp),
            LogicalKey::ArrowDown => Some(SemanticCommand::FocusDown),
            LogicalKey::Escape => Some(SemanticCommand::CancelOrBack),
            LogicalKey::Enter if !event.is_repeat() => Some(SemanticCommand::Activate),
            _ => None,
        };
        if matches!(command, Some(SemanticCommand::Activate))
            && !self.keyboard_activation_eligible(target)
        {
            return;
        }
        if let Some(command) = command {
            let _ = self.submit_command(
                target.clone(),
                command,
                CommandOrigin::__runtime_keyboard_default(),
            );
        }
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
