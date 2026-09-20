//! Host-neutral application-owned text editing protocol values.

use core::{
    error::Error,
    fmt,
    hash::{Hash, Hasher},
};
use std::{rc::Rc, sync::Arc};

use crate::{
    CompositionGeneration, Effects, HostProtocol, TextAffinity, TextDocumentSnapshot, TextPosition,
    TextPositionError, TextRange, TextSelection, runtime_protocol::RuntimeNamespace,
};

/// Runtime-issued identity for one exact mounted editing-session lifetime.
#[derive(Clone)]
pub struct EditingSessionGeneration {
    pub(crate) namespace: RuntimeNamespace,
    pub(crate) generation: u64,
}

impl EditingSessionGeneration {
    #[must_use]
    pub const fn get(&self) -> u64 {
        self.generation
    }
}

impl PartialEq for EditingSessionGeneration {
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation && self.namespace.__runtime_same_as(&other.namespace)
    }
}

impl Eq for EditingSessionGeneration {}

impl Hash for EditingSessionGeneration {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.namespace.__runtime_hash(state);
        self.generation.hash(state);
    }
}

impl fmt::Debug for EditingSessionGeneration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EditingSessionGeneration(..)")
    }
}

/// Runtime-issued identity for one exact application edit proposal.
#[derive(Clone)]
pub struct EditRequestId {
    pub(crate) namespace: RuntimeNamespace,
    pub(crate) request: u64,
}

impl EditRequestId {
    #[must_use]
    pub const fn get(&self) -> u64 {
        self.request
    }
}

impl PartialEq for EditRequestId {
    fn eq(&self, other: &Self) -> bool {
        self.request == other.request && self.namespace.__runtime_same_as(&other.namespace)
    }
}

impl Eq for EditRequestId {}

impl Hash for EditRequestId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.namespace.__runtime_hash(state);
        self.request.hash(state);
    }
}

impl fmt::Debug for EditRequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EditRequestId(..)")
    }
}

/// Confidentiality class attached to one application-owned document presentation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TextSensitivity {
    Public,
    Secret,
}

/// Reconciliation policy for owner-local ephemeral editing state.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EditingSessionPolicy {
    /// Preserve the live session only while document snapshot and text remain exact.
    PreserveExact,
    /// Reset ephemeral state whenever the editable contribution is reconciled.
    Reset,
}

/// Application-visible reason for one document-changing edit proposal.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EditKind {
    Insert,
    DeleteBackward,
    DeleteForward,
    Replace,
    Cut,
    Paste,
    Undo,
    Redo,
}

/// Optional application-owned undo grouping hint.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EditGroupHint(u64);

impl EditGroupHint {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A selection expressed in the projected result text before the application
/// chooses the resulting document revision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EditSelection {
    anchor: usize,
    anchor_affinity: TextAffinity,
    active: usize,
    active_affinity: TextAffinity,
}

impl EditSelection {
    /// Copies the offsets and affinities from an already checked durable selection.
    #[must_use]
    pub const fn from_selection(selection: TextSelection) -> Self {
        Self {
            anchor: selection.anchor().byte_offset(),
            anchor_affinity: selection.anchor().affinity(),
            active: selection.active().byte_offset(),
            active_affinity: selection.active().affinity(),
        }
    }

    /// Validates both UTF-8 offsets against the projected result text.
    ///
    /// # Errors
    ///
    /// Returns [`TextPositionError`] when either offset is out of bounds or is
    /// not a UTF-8 scalar boundary in `projected_text`.
    pub fn new(
        projected_text: &str,
        anchor: usize,
        anchor_affinity: TextAffinity,
        active: usize,
        active_affinity: TextAffinity,
    ) -> Result<Self, TextPositionError> {
        validate_projected_offset(projected_text, anchor)?;
        validate_projected_offset(projected_text, active)?;
        Ok(Self {
            anchor,
            anchor_affinity,
            active,
            active_affinity,
        })
    }

    /// Validates one collapsed projected-result selection.
    ///
    /// # Errors
    ///
    /// Returns [`TextPositionError`] when `offset` is out of bounds or is not a
    /// UTF-8 scalar boundary in `projected_text`.
    pub fn collapsed(
        projected_text: &str,
        offset: usize,
        affinity: TextAffinity,
    ) -> Result<Self, TextPositionError> {
        Self::new(projected_text, offset, affinity, offset, affinity)
    }

    #[must_use]
    pub const fn anchor(self) -> usize {
        self.anchor
    }

    #[must_use]
    pub const fn anchor_affinity(self) -> TextAffinity {
        self.anchor_affinity
    }

    #[must_use]
    pub const fn active(self) -> usize {
        self.active
    }

    #[must_use]
    pub const fn active_affinity(self) -> TextAffinity {
        self.active_affinity
    }

    /// Binds the projected offsets to an accepted application snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`TextPositionError`] when either projected offset cannot be
    /// represented by the accepted `text` and `snapshot`.
    pub fn bind(
        self,
        snapshot: TextDocumentSnapshot,
        text: &str,
    ) -> Result<TextSelection, TextPositionError> {
        let anchor = TextPosition::new(snapshot, text, self.anchor, self.anchor_affinity)?;
        let active = TextPosition::new(snapshot, text, self.active, self.active_affinity)?;
        TextSelection::new(anchor, active).map_err(|_| TextPositionError::OutOfBounds)
    }
}

const fn validate_projected_offset(text: &str, offset: usize) -> Result<(), TextPositionError> {
    if offset > text.len() {
        Err(TextPositionError::OutOfBounds)
    } else if !text.is_char_boundary(offset) {
        Err(TextPositionError::NotScalarBoundary)
    } else {
        Ok(())
    }
}

/// Deterministic inverse facts an application may retain in its own undo journal.
#[derive(Clone)]
pub struct EditInverse {
    replacement_start: usize,
    replacement_end: usize,
    replacement_text: Arc<str>,
    selection: EditSelection,
}

impl EditInverse {
    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_new(
        replacement: core::ops::Range<usize>,
        replacement_text: impl Into<Arc<str>>,
        selection: EditSelection,
    ) -> Self {
        Self {
            replacement_start: replacement.start,
            replacement_end: replacement.end,
            replacement_text: replacement_text.into(),
            selection,
        }
    }

    #[must_use]
    pub const fn replacement(&self) -> core::ops::Range<usize> {
        self.replacement_start..self.replacement_end
    }

    #[must_use]
    pub fn replacement_text(&self) -> &str {
        &self.replacement_text
    }

    #[must_use]
    pub const fn selection(&self) -> EditSelection {
        self.selection
    }
}

impl fmt::Debug for EditInverse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EditInverse")
            .field(
                "replacement",
                &(self.replacement_start..self.replacement_end),
            )
            .field("replacement_bytes", &self.replacement_text.len())
            .field("selection", &self.selection)
            .finish()
    }
}

/// One immutable application edit proposal carried by the ordinary action FIFO.
#[derive(Clone)]
pub struct EditIntent {
    request: EditRequestId,
    session: EditingSessionGeneration,
    predecessor: Option<EditRequestId>,
    replacement: TextRange,
    replacement_text: Arc<str>,
    proposed_selection: EditSelection,
    kind: EditKind,
    inverse: Option<EditInverse>,
    group: Option<EditGroupHint>,
    sensitivity: TextSensitivity,
    composition: Option<CompositionGeneration>,
}

impl EditIntent {
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn __runtime_new(
        request: EditRequestId,
        session: EditingSessionGeneration,
        predecessor: Option<EditRequestId>,
        replacement: TextRange,
        replacement_text: Arc<str>,
        proposed_selection: EditSelection,
        kind: EditKind,
        inverse: Option<EditInverse>,
        group: Option<EditGroupHint>,
        sensitivity: TextSensitivity,
        composition: Option<CompositionGeneration>,
    ) -> Self {
        Self {
            request,
            session,
            predecessor,
            replacement,
            replacement_text,
            proposed_selection,
            kind,
            inverse,
            group,
            sensitivity,
            composition,
        }
    }

    #[must_use]
    pub const fn request(&self) -> &EditRequestId {
        &self.request
    }
    #[must_use]
    pub const fn session(&self) -> &EditingSessionGeneration {
        &self.session
    }
    #[must_use]
    pub const fn predecessor(&self) -> Option<&EditRequestId> {
        self.predecessor.as_ref()
    }
    #[must_use]
    pub const fn replacement(&self) -> TextRange {
        self.replacement
    }
    #[must_use]
    pub fn replacement_text(&self) -> &str {
        &self.replacement_text
    }
    #[must_use]
    pub const fn proposed_selection(&self) -> EditSelection {
        self.proposed_selection
    }
    #[must_use]
    pub const fn kind(&self) -> EditKind {
        self.kind
    }
    #[must_use]
    pub const fn inverse(&self) -> Option<&EditInverse> {
        self.inverse.as_ref()
    }
    #[must_use]
    pub const fn group(&self) -> Option<EditGroupHint> {
        self.group
    }
    #[must_use]
    pub const fn sensitivity(&self) -> TextSensitivity {
        self.sensitivity
    }
    #[must_use]
    pub const fn composition(&self) -> Option<&CompositionGeneration> {
        self.composition.as_ref()
    }
}

impl fmt::Debug for EditIntent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EditIntent")
            .field("request", &self.request)
            .field("session", &self.session)
            .field("has_predecessor", &self.predecessor.is_some())
            .field("replacement", &self.replacement)
            .field("replacement_bytes", &self.replacement_text.len())
            .field("proposed_selection", &self.proposed_selection)
            .field("kind", &self.kind)
            .field("has_inverse", &self.inverse.is_some())
            .field("group", &self.group)
            .field("sensitivity", &self.sensitivity)
            .field("has_composition", &self.composition.is_some())
            .finish()
    }
}

/// Validated mapping supplied when an application transforms an edit result.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EditChangeMap {
    replaced: TextRange,
    replacement_bytes: usize,
}

impl EditChangeMap {
    #[must_use]
    pub const fn new(replaced: TextRange, replacement_bytes: usize) -> Self {
        Self {
            replaced,
            replacement_bytes,
        }
    }

    #[must_use]
    pub const fn replaced(self) -> TextRange {
        self.replaced
    }

    #[must_use]
    pub const fn replacement_bytes(self) -> usize {
        self.replacement_bytes
    }
}

/// Application decision for one exact edit proposal.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditResolutionOutcome {
    Accepted,
    Rejected,
    Transformed { mapping: EditChangeMap },
}

/// Immediate transaction-local response required for an edit-origin action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditResolution {
    request: EditRequestId,
    resulting_snapshot: TextDocumentSnapshot,
    outcome: EditResolutionOutcome,
}

impl EditResolution {
    #[must_use]
    pub const fn accepted(
        request: EditRequestId,
        resulting_snapshot: TextDocumentSnapshot,
    ) -> Self {
        Self {
            request,
            resulting_snapshot,
            outcome: EditResolutionOutcome::Accepted,
        }
    }

    #[must_use]
    pub const fn rejected(
        request: EditRequestId,
        authoritative_snapshot: TextDocumentSnapshot,
    ) -> Self {
        Self {
            request,
            resulting_snapshot: authoritative_snapshot,
            outcome: EditResolutionOutcome::Rejected,
        }
    }

    #[must_use]
    pub const fn transformed(
        request: EditRequestId,
        resulting_snapshot: TextDocumentSnapshot,
        mapping: EditChangeMap,
    ) -> Self {
        Self {
            request,
            resulting_snapshot,
            outcome: EditResolutionOutcome::Transformed { mapping },
        }
    }

    #[must_use]
    pub const fn request(&self) -> &EditRequestId {
        &self.request
    }

    #[must_use]
    pub const fn resulting_snapshot(&self) -> TextDocumentSnapshot {
        self.resulting_snapshot
    }

    #[must_use]
    pub const fn outcome(&self) -> &EditResolutionOutcome {
        &self.outcome
    }
}

/// Complete editable-document contribution from one mounted owner.
pub struct EditableContribution<Action> {
    snapshot: TextDocumentSnapshot,
    text: Arc<str>,
    initial_selection: TextSelection,
    sensitivity: TextSensitivity,
    read_only: bool,
    disabled: bool,
    session_policy: EditingSessionPolicy,
    mapper: Rc<dyn Fn(EditIntent) -> Action>,
}

/// Failure while binding an editable contribution to one exact document snapshot.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditableContributionError {
    SelectionSnapshotMismatch,
    Position(TextPositionError),
}

impl fmt::Display for EditableContributionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelectionSnapshotMismatch => {
                formatter.write_str("editable selection addresses another document snapshot")
            }
            Self::Position(error) => write!(formatter, "invalid editable selection: {error}"),
        }
    }
}

impl Error for EditableContributionError {}

impl From<TextPositionError> for EditableContributionError {
    fn from(value: TextPositionError) -> Self {
        Self::Position(value)
    }
}

impl<Action> EditableContribution<Action> {
    /// Creates one checked application-owned editable contribution.
    ///
    /// # Errors
    ///
    /// Returns [`EditableContributionError`] when the selection addresses a
    /// different snapshot or contains a position invalid for `text`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        snapshot: TextDocumentSnapshot,
        text: impl Into<Arc<str>>,
        initial_selection: TextSelection,
        sensitivity: TextSensitivity,
        read_only: bool,
        disabled: bool,
        session_policy: EditingSessionPolicy,
        mapper: impl Fn(EditIntent) -> Action + 'static,
    ) -> Result<Self, EditableContributionError> {
        let text = text.into();
        if initial_selection.anchor().snapshot() != snapshot
            || initial_selection.active().snapshot() != snapshot
        {
            return Err(EditableContributionError::SelectionSnapshotMismatch);
        }
        TextPosition::new(
            snapshot,
            &text,
            initial_selection.anchor().byte_offset(),
            initial_selection.anchor().affinity(),
        )?;
        TextPosition::new(
            snapshot,
            &text,
            initial_selection.active().byte_offset(),
            initial_selection.active().affinity(),
        )?;
        Ok(Self {
            snapshot,
            text,
            initial_selection,
            sensitivity,
            read_only,
            disabled,
            session_policy,
            mapper: Rc::new(mapper),
        })
    }

    #[must_use]
    pub const fn snapshot(&self) -> TextDocumentSnapshot {
        self.snapshot
    }
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
    #[must_use]
    pub const fn initial_selection(&self) -> TextSelection {
        self.initial_selection
    }
    #[must_use]
    pub const fn sensitivity(&self) -> TextSensitivity {
        self.sensitivity
    }
    #[must_use]
    pub const fn read_only(&self) -> bool {
        self.read_only
    }
    #[must_use]
    pub const fn disabled(&self) -> bool {
        self.disabled
    }
    #[must_use]
    pub const fn session_policy(&self) -> EditingSessionPolicy {
        self.session_policy
    }
    #[must_use]
    pub fn map_intent(&self, intent: EditIntent) -> Action {
        (self.mapper)(intent)
    }

    #[must_use]
    pub fn map_action<ParentAction>(
        self,
        mapper: Rc<dyn Fn(Action) -> ParentAction>,
    ) -> EditableContribution<ParentAction>
    where
        Action: 'static,
        ParentAction: 'static,
    {
        let child_mapper = self.mapper;
        EditableContribution {
            snapshot: self.snapshot,
            text: self.text,
            initial_selection: self.initial_selection,
            sensitivity: self.sensitivity,
            read_only: self.read_only,
            disabled: self.disabled,
            session_policy: self.session_policy,
            mapper: Rc::new(move |intent| mapper(child_mapper(intent))),
        }
    }
}

impl<Action> fmt::Debug for EditableContribution<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EditableContribution")
            .field("snapshot", &self.snapshot)
            .field("text_bytes", &self.text.len())
            .field("initial_selection", &self.initial_selection)
            .field("sensitivity", &self.sensitivity)
            .field("read_only", &self.read_only)
            .field("disabled", &self.disabled)
            .field("session_policy", &self.session_policy)
            .finish_non_exhaustive()
    }
}

/// Application update result with one optional immediate edit resolution.
#[must_use]
pub struct UpdateOutput<Action, Protocol: HostProtocol> {
    effects: Effects<Action, Protocol>,
    edit_resolution: Option<EditResolution>,
}

impl<Action, Protocol: HostProtocol> UpdateOutput<Action, Protocol> {
    pub const fn effects(effects: Effects<Action, Protocol>) -> Self {
        Self {
            effects,
            edit_resolution: None,
        }
    }

    pub const fn edit(resolution: EditResolution) -> Self {
        Self {
            effects: Effects::none(),
            edit_resolution: Some(resolution),
        }
    }

    pub const fn edit_with_effects(
        effects: Effects<Action, Protocol>,
        resolution: EditResolution,
    ) -> Self {
        Self {
            effects,
            edit_resolution: Some(resolution),
        }
    }

    #[doc(hidden)]
    pub fn __runtime_into_parts(self) -> (Effects<Action, Protocol>, Option<EditResolution>) {
        (self.effects, self.edit_resolution)
    }
}

/// Conversion used only for the immediate result of [`crate::UiApp::update`].
pub trait IntoUpdateOutput<Action, Protocol: HostProtocol> {
    fn into_update_output(self) -> UpdateOutput<Action, Protocol>;
}

impl<Action, Protocol: HostProtocol> IntoUpdateOutput<Action, Protocol> for () {
    fn into_update_output(self) -> UpdateOutput<Action, Protocol> {
        UpdateOutput::effects(Effects::none())
    }
}

impl<Action, Protocol: HostProtocol> IntoUpdateOutput<Action, Protocol>
    for Effects<Action, Protocol>
{
    fn into_update_output(self) -> UpdateOutput<Action, Protocol> {
        UpdateOutput::effects(self)
    }
}

impl<Action, Protocol: HostProtocol> IntoUpdateOutput<Action, Protocol>
    for UpdateOutput<Action, Protocol>
{
    fn into_update_output(self) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::{rc::Rc, sync::Arc};

    use crate::{
        __runtime::RuntimeNamespace, EditIntent, EditableContribution, EditableContributionError,
        EditingSessionPolicy, NoHostProtocol, SemanticEditable, TextAffinity, TextDocumentId,
        TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextPositionError, TextRange,
        TextSelection, TextSensitivity, UpdateOutput,
    };

    fn snapshot(revision: u64) -> TextDocumentSnapshot {
        TextDocumentSnapshot::new(TextDocumentId::new(4), TextDocumentRevision::new(revision))
    }

    fn selection(source: &str, offset: usize) -> TextSelection {
        TextSelection::collapsed(
            TextPosition::new(snapshot(1), source, offset, TextAffinity::Downstream)
                .unwrap_or_else(|_| unreachable!("test selection is valid")),
        )
    }

    #[test]
    fn projected_selection_checks_utf8_boundaries() {
        assert_eq!(
            super::EditSelection::collapsed("aé", 2, TextAffinity::Downstream),
            Err(TextPositionError::NotScalarBoundary)
        );
        assert!(super::EditSelection::collapsed("aé", 3, TextAffinity::Downstream).is_ok());
    }

    #[test]
    fn editable_contribution_rejects_foreign_selection_snapshot() {
        let source = "abc";
        let foreign = TextSelection::collapsed(
            TextPosition::new(snapshot(2), source, 0, TextAffinity::Downstream)
                .unwrap_or_else(|_| unreachable!("test position is valid")),
        );
        let result = EditableContribution::<()>::new(
            snapshot(1),
            source,
            foreign,
            TextSensitivity::Public,
            false,
            false,
            EditingSessionPolicy::PreserveExact,
            |_| (),
        );
        assert!(matches!(
            result,
            Err(EditableContributionError::SelectionSnapshotMismatch)
        ));
    }

    #[test]
    fn recursive_mapper_maps_only_the_opaque_action() {
        let source = "abc";
        let contribution = EditableContribution::new(
            snapshot(1),
            source,
            selection(source, 3),
            TextSensitivity::Public,
            false,
            false,
            EditingSessionPolicy::PreserveExact,
            |_| 7_u8,
        )
        .unwrap_or_else(|_| unreachable!("test contribution is valid"))
        .map_action(Rc::new(|value| usize::from(value) + 1));
        let namespace = RuntimeNamespace::__runtime_new();
        let request = namespace.__runtime_edit_request_id(1);
        let session = namespace.__runtime_editing_session_generation(1);
        let replacement = TextRange::new(snapshot(1), source, 3, 3)
            .unwrap_or_else(|_| unreachable!("test range is valid"));
        let intent = EditIntent::__runtime_new(
            request,
            session,
            None,
            replacement,
            Arc::from("x"),
            super::EditSelection::collapsed("abcx", 4, TextAffinity::Downstream)
                .unwrap_or_else(|_| unreachable!("test projected selection is valid")),
            super::EditKind::Insert,
            None,
            None,
            TextSensitivity::Public,
            None,
        );
        assert_eq!(contribution.map_intent(intent), 8);
    }

    #[test]
    fn secret_values_and_edit_payload_debug_are_redacted() {
        let source = "never-log-this";
        let semantic = SemanticEditable::new(
            snapshot(1),
            source,
            selection(source, source.len()),
            TextSensitivity::Secret,
            false,
        )
        .unwrap_or_else(|| unreachable!("test semantic projection is valid"));
        assert_eq!(semantic.value(), None);
        assert!(!format!("{semantic:?}").contains(source));

        let namespace = RuntimeNamespace::__runtime_new();
        let intent = EditIntent::__runtime_new(
            namespace.__runtime_edit_request_id(1),
            namespace.__runtime_editing_session_generation(1),
            None,
            TextRange::new(snapshot(1), "", 0, 0)
                .unwrap_or_else(|_| unreachable!("test range is valid")),
            Arc::from(source),
            super::EditSelection::collapsed(source, source.len(), TextAffinity::Downstream)
                .unwrap_or_else(|_| unreachable!("test selection is valid")),
            super::EditKind::Paste,
            None,
            None,
            TextSensitivity::Secret,
            None,
        );
        assert!(!format!("{intent:?}").contains(source));
    }

    #[test]
    fn update_output_keeps_resolution_out_of_effect_composition() {
        let namespace = RuntimeNamespace::__runtime_new();
        let resolution =
            super::EditResolution::accepted(namespace.__runtime_edit_request_id(1), snapshot(2));
        let (effects, resolution) =
            UpdateOutput::<(), NoHostProtocol>::edit(resolution).__runtime_into_parts();
        assert!(effects.__runtime_into_items().is_empty());
        assert!(resolution.is_some());
    }
}
