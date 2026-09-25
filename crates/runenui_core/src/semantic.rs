//! Platform-neutral semantic contribution vocabulary.
//!
//! Widgets author owner-local semantic facts. Runtime owns live semantic IDs,
//! mounted ownership, absolute bounds, focus, publication revisions, and action
//! routing. Nothing in this module is tied to AccessKit or a native host API.

use core::fmt;
use std::{collections::BTreeSet, sync::Arc};

use crate::identity::{IdentifierText, validate_identifier};
use crate::{
    ElementId, IdentifierError, LogicalRect, TextDocumentSnapshot, TextSelection, TextSensitivity,
};

/// Stable owner-local identity for one contributed semantic node.
///
/// [`Self::PRIMARY`] is reserved for the ordinary single-node widget case.
/// Additional or virtual nodes use validated authored keys.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticKey(SemanticKeyValue);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum SemanticKeyValue {
    Primary,
    Named(IdentifierText),
}

impl SemanticKey {
    /// Reserved owner-local key for an ordinary widget's primary semantic node.
    pub const PRIMARY: Self = Self(SemanticKeyValue::Primary);

    /// Validates and owns an additional owner-local semantic key.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierError`] under the canonical authored-identifier grammar.
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(SemanticKeyValue::Named(IdentifierText::owned(value))))
    }

    /// Validates a static additional owner-local semantic key without allocation.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierError`] under the canonical authored-identifier grammar.
    pub const fn from_static(value: &'static str) -> Result<Self, IdentifierError> {
        match validate_identifier(value) {
            Ok(()) => Ok(Self(SemanticKeyValue::Named(IdentifierText::from_static(
                value,
            )))),
            Err(error) => Err(error),
        }
    }

    /// Returns whether this is the reserved primary key.
    #[must_use]
    pub const fn is_primary(&self) -> bool {
        matches!(self.0, SemanticKeyValue::Primary)
    }

    /// Returns the authored key text for an additional key.
    #[must_use]
    pub const fn as_str(&self) -> Option<&str> {
        match &self.0 {
            SemanticKeyValue::Primary => None,
            SemanticKeyValue::Named(value) => Some(value.as_str()),
        }
    }
}

impl fmt::Display for SemanticKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_str() {
            Some(value) => formatter.write_str(value),
            None => formatter.write_str("<primary>"),
        }
    }
}

/// Platform-neutral semantic role.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticRole {
    Generic,
    Group,
    Text,
    Button,
    EditableText,
    Checkbox,
    RadioButton,
    RadioGroup,
    Switch,
}

/// Platform-neutral checked state for stateful binary controls.
///
/// This is durable application-authored semantic meaning. Runtime publishes the
/// fact but never toggles or otherwise owns it.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticCheckedState {
    Unchecked,
    Checked,
    Mixed,
}

/// Revision-scoped editable text facts projected through the neutral semantic tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEditable {
    snapshot: TextDocumentSnapshot,
    selection: TextSelection,
    sensitivity: TextSensitivity,
    value: Option<String>,
    read_only: bool,
    caret_offsets: Option<Arc<[usize]>>,
}

impl SemanticEditable {
    /// Builds a fail-closed semantic projection from an authoritative source.
    #[must_use]
    pub fn new(
        snapshot: TextDocumentSnapshot,
        source: &str,
        selection: TextSelection,
        sensitivity: TextSensitivity,
        read_only: bool,
    ) -> Option<Self> {
        if selection.anchor().snapshot() != snapshot
            || selection.active().snapshot() != snapshot
            || selection.anchor().byte_offset() > source.len()
            || selection.active().byte_offset() > source.len()
            || !source.is_char_boundary(selection.anchor().byte_offset())
            || !source.is_char_boundary(selection.active().byte_offset())
        {
            return None;
        }
        Some(Self {
            snapshot,
            selection,
            sensitivity,
            value: (sensitivity == TextSensitivity::Public).then(|| source.to_owned()),
            read_only,
            caret_offsets: None,
        })
    }

    #[must_use]
    pub const fn snapshot(&self) -> TextDocumentSnapshot {
        self.snapshot
    }

    #[must_use]
    pub const fn selection(&self) -> TextSelection {
        self.selection
    }

    #[must_use]
    pub const fn sensitivity(&self) -> TextSensitivity {
        self.sensitivity
    }

    /// Returns public text only; secret text is structurally absent.
    #[must_use]
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    #[must_use]
    pub const fn read_only(&self) -> bool {
        self.read_only
    }

    /// Returns shaping-validated selectable UTF-8 boundaries when bound by runtime publication.
    #[must_use]
    pub fn caret_offsets(&self) -> Option<&[usize]> {
        self.caret_offsets.as_deref()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_with_projection(
        mut self,
        source: &str,
        selection: TextSelection,
        offsets: Arc<[usize]>,
    ) -> Option<Self> {
        if selection.anchor().snapshot() != self.snapshot
            || selection.active().snapshot() != self.snapshot
            || selection.anchor().byte_offset() > source.len()
            || selection.active().byte_offset() > source.len()
            || !source.is_char_boundary(selection.anchor().byte_offset())
            || !source.is_char_boundary(selection.active().byte_offset())
            || offsets.first() != Some(&0)
            || offsets.last() != Some(&source.len())
            || offsets
                .windows(2)
                .any(|pair| pair[0] >= pair[1] || !source.is_char_boundary(pair[1]))
        {
            return None;
        }
        self.selection = selection;
        self.caret_offsets = Some(offsets);
        Some(self)
    }
}

/// Read-only value exposed by a semantic node.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticValue {
    Text(String),
    Boolean(bool),
    Integer(i64),
}

/// Plain-text semantic content with room for later text-range extensions.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticText {
    Plain(String),
}

impl SemanticText {
    #[must_use]
    pub fn plain(value: impl Into<String>) -> Self {
        Self::Plain(value.into())
    }

    #[must_use]
    pub const fn as_plain(&self) -> Option<&str> {
        match self {
            Self::Plain(value) => Some(value.as_str()),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}

/// Widget-authored semantic state. Runtime-derived focus is deliberately absent.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticState {
    disabled: bool,
    hidden: bool,
    inert: bool,
    read_only: bool,
    checked: Option<SemanticCheckedState>,
}

impl SemanticState {
    pub const ENABLED: Self = Self {
        disabled: false,
        hidden: false,
        inert: false,
        read_only: false,
        checked: None,
    };

    #[must_use]
    pub const fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub const fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    #[must_use]
    pub const fn with_inert(mut self, inert: bool) -> Self {
        self.inert = inert;
        self
    }

    #[must_use]
    pub const fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Authors the exact checked state for a checkable semantic role.
    #[must_use]
    pub const fn with_checked(mut self, checked: SemanticCheckedState) -> Self {
        self.checked = Some(checked);
        self
    }

    #[must_use]
    pub const fn disabled(self) -> bool {
        self.disabled
    }

    #[must_use]
    pub const fn hidden(self) -> bool {
        self.hidden
    }

    #[must_use]
    pub const fn inert(self) -> bool {
        self.inert
    }

    #[must_use]
    pub const fn read_only(self) -> bool {
        self.read_only
    }

    /// Returns the application-authored checked state when this role is checkable.
    #[must_use]
    pub const fn checked(self) -> Option<SemanticCheckedState> {
        self.checked
    }
}

/// Semantic actions with real `RunenUI` behavior in the accepted M5 design.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SemanticAction {
    Activate,
    RequestFocus,
    OpenMenu,
    OpenContextMenu,
    MoveBackward,
    MoveForward,
    ExtendBackward,
    ExtendForward,
    SelectAll,
    DeleteBackward,
    DeleteForward,
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    SetSelection,
    ReplaceSelection,
}

/// Relationship category expressed without platform-adapter vocabulary.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticRelationshipKind {
    LabelledBy,
    DescribedBy,
    Controls,
}

/// Stable authored target for a semantic relationship.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticReference {
    /// Another semantic key owned by the same mounted widget lifetime.
    Local(SemanticKey),
    /// A uniquely authored mounted owner plus an optional owner-local semantic key.
    Authored {
        element_id: ElementId,
        semantic_key: Option<SemanticKey>,
    },
}

/// One semantic relationship declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticRelationship {
    kind: SemanticRelationshipKind,
    target: SemanticReference,
}

impl SemanticRelationship {
    #[must_use]
    pub const fn new(kind: SemanticRelationshipKind, target: SemanticReference) -> Self {
        Self { kind, target }
    }

    #[must_use]
    pub const fn kind(&self) -> SemanticRelationshipKind {
        self.kind
    }

    #[must_use]
    pub const fn target(&self) -> &SemanticReference {
        &self.target
    }
}

/// Widget-authored semantic bounds policy.
///
/// `OwnerLocal` is translated by runtime from owner-local coordinates; widgets
/// never author absolute surface coordinates through this type.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SemanticBounds {
    #[default]
    Owner,
    OwnerLocal(LogicalRect),
}

/// One item in an owner-local semantic sequence.
///
/// The variants are semantic vocabulary. Recursive storage remains hidden by the
/// opaque [`SemanticNodeContribution`] representation.
#[derive(Clone, Debug, PartialEq)]
pub enum SemanticItem {
    Node(SemanticNodeContribution),
    MountedChildren,
}

impl SemanticItem {
    #[must_use]
    pub const fn node(node: SemanticNodeContribution) -> Self {
        Self::Node(node)
    }

    #[must_use]
    pub const fn mounted_children() -> Self {
        Self::MountedChildren
    }

    /// Returns the contributed node when this item is a local semantic node.
    #[must_use]
    pub const fn as_node(&self) -> Option<&SemanticNodeContribution> {
        match self {
            Self::Node(node) => Some(node),
            Self::MountedChildren => None,
        }
    }

    /// Returns whether this item is the explicit mounted-children splice marker.
    #[must_use]
    pub const fn is_mounted_children(&self) -> bool {
        matches!(self, Self::MountedChildren)
    }
}

/// One owner-local semantic node description.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticNodeContribution(Box<SemanticNodeData>);

#[derive(Clone, Debug, PartialEq)]
struct SemanticNodeData {
    key: SemanticKey,
    role: SemanticRole,
    name: Option<String>,
    description: Option<String>,
    value: Option<SemanticValue>,
    state: SemanticState,
    actions: Vec<SemanticAction>,
    relationships: Vec<SemanticRelationship>,
    bounds: SemanticBounds,
    text: Option<SemanticText>,
    editable: Option<SemanticEditable>,
    children: Vec<SemanticItem>,
}

impl SemanticNodeContribution {
    #[must_use]
    pub fn new(key: SemanticKey, role: SemanticRole) -> Self {
        Self(Box::new(SemanticNodeData {
            key,
            role,
            name: None,
            description: None,
            value: None,
            state: SemanticState::ENABLED,
            actions: Vec::new(),
            relationships: Vec::new(),
            bounds: SemanticBounds::Owner,
            text: None,
            editable: None,
            children: Vec::new(),
        }))
    }

    /// Creates a node using the reserved primary owner-local semantic key.
    #[must_use]
    pub fn primary(role: SemanticRole) -> Self {
        Self::new(SemanticKey::PRIMARY, role)
    }

    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.0.name = Some(name.into());
        self
    }

    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.0.description = Some(description.into());
        self
    }

    #[must_use]
    pub fn with_value(mut self, value: SemanticValue) -> Self {
        self.0.value = Some(value);
        self
    }

    #[must_use]
    pub fn with_state(mut self, state: SemanticState) -> Self {
        self.0.state = state;
        self
    }

    #[must_use]
    pub fn with_action(mut self, action: SemanticAction) -> Self {
        if !self.0.actions.contains(&action) {
            self.0.actions.push(action);
        }
        self
    }

    #[must_use]
    pub fn with_relationship(mut self, relationship: SemanticRelationship) -> Self {
        if !self.0.relationships.contains(&relationship) {
            self.0.relationships.push(relationship);
        }
        self
    }

    #[must_use]
    pub fn with_bounds(mut self, bounds: SemanticBounds) -> Self {
        self.0.bounds = bounds;
        self
    }

    #[must_use]
    pub fn with_text(mut self, text: SemanticText) -> Self {
        self.0.text = Some(text);
        self
    }

    #[must_use]
    pub fn with_editable(mut self, editable: SemanticEditable) -> Self {
        self.0.editable = Some(editable);
        self
    }

    #[must_use]
    pub fn with_children(mut self, children: Vec<SemanticItem>) -> Self {
        self.0.children = children;
        self
    }

    #[must_use]
    pub fn with_child(mut self, child: Self) -> Self {
        self.0.children.push(SemanticItem::node(child));
        self
    }

    #[must_use]
    pub fn with_mounted_children(mut self) -> Self {
        self.0.children.push(SemanticItem::mounted_children());
        self
    }

    #[must_use]
    pub const fn key(&self) -> &SemanticKey {
        &self.0.key
    }

    #[must_use]
    pub const fn role(&self) -> SemanticRole {
        self.0.role
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }

    #[must_use]
    pub const fn value(&self) -> Option<&SemanticValue> {
        self.0.value.as_ref()
    }

    #[must_use]
    pub const fn state(&self) -> SemanticState {
        self.0.state
    }

    #[must_use]
    pub const fn actions(&self) -> &[SemanticAction] {
        self.0.actions.as_slice()
    }

    #[must_use]
    pub const fn relationships(&self) -> &[SemanticRelationship] {
        self.0.relationships.as_slice()
    }

    #[must_use]
    pub const fn bounds(&self) -> SemanticBounds {
        self.0.bounds
    }

    #[must_use]
    pub const fn text(&self) -> Option<&SemanticText> {
        self.0.text.as_ref()
    }

    #[must_use]
    pub const fn editable(&self) -> Option<&SemanticEditable> {
        self.0.editable.as_ref()
    }

    #[must_use]
    pub const fn children(&self) -> &[SemanticItem] {
        self.0.children.as_slice()
    }
}

/// Read-only structural facts supplied when a widget contributes semantics.
///
/// The context intentionally exposes no mounted IDs, semantic IDs, runtime
/// namespace, layout coordinates, focus, or action authority.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticContributionContext {
    direct_mounted_children: usize,
}

impl SemanticContributionContext {
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(direct_mounted_children: usize) -> Self {
        Self {
            direct_mounted_children,
        }
    }

    #[must_use]
    pub const fn direct_mounted_children(self) -> usize {
        self.direct_mounted_children
    }

    #[must_use]
    pub const fn has_mounted_children(self) -> bool {
        self.direct_mounted_children != 0
    }
}

/// One widget's ordered owner-local semantic forest.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SemanticContribution {
    roots: Vec<SemanticItem>,
}

impl SemanticContribution {
    #[must_use]
    pub const fn empty() -> Self {
        Self { roots: Vec::new() }
    }

    #[must_use]
    pub const fn new(roots: Vec<SemanticItem>) -> Self {
        Self { roots }
    }

    #[must_use]
    pub fn single(node: SemanticNodeContribution) -> Self {
        Self::new(vec![SemanticItem::node(node)])
    }

    #[must_use]
    pub const fn roots(&self) -> &[SemanticItem] {
        self.roots.as_slice()
    }

    /// Validates owner-local identity, references, role/state semantics, and the exact
    /// mounted-child marker contract.
    ///
    /// # Errors
    ///
    /// Returns a deterministic structural error. Validation never inserts a
    /// fallback marker and never chooses one occurrence of a duplicate key.
    pub fn validate(
        &self,
        context: SemanticContributionContext,
    ) -> Result<SemanticContributionValidation, SemanticContributionError> {
        let mut keys = BTreeSet::new();
        let mut ordered_keys = Vec::new();
        let mut marker_count = 0usize;
        collect_structure(
            self.roots(),
            &mut keys,
            &mut ordered_keys,
            &mut marker_count,
        )?;

        if marker_count > 1 {
            return Err(SemanticContributionError::DuplicateMountedChildrenMarker);
        }

        let node_count = ordered_keys.len();
        if node_count == 0 {
            if marker_count != 0 {
                return Err(SemanticContributionError::UnnecessaryMountedChildrenMarker);
            }
        } else if context.has_mounted_children() {
            if marker_count == 0 {
                return Err(SemanticContributionError::MissingMountedChildrenMarker);
            }
        } else if marker_count != 0 {
            return Err(SemanticContributionError::UnnecessaryMountedChildrenMarker);
        }

        validate_local_references(self.roots(), &keys)?;
        validate_role_state_contract(self.roots())?;

        Ok(SemanticContributionValidation { ordered_keys })
    }
}

/// Successful structural validation result in deterministic contribution order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticContributionValidation {
    ordered_keys: Vec<SemanticKey>,
}

impl SemanticContributionValidation {
    #[must_use]
    pub const fn ordered_keys(&self) -> &[SemanticKey] {
        self.ordered_keys.as_slice()
    }
}

/// Deterministic owner-local semantic contribution rejection.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticContributionError {
    DuplicateKey {
        key: SemanticKey,
    },
    MissingMountedChildrenMarker,
    DuplicateMountedChildrenMarker,
    UnnecessaryMountedChildrenMarker,
    MissingLocalReference {
        source: SemanticKey,
        target: SemanticKey,
    },
    MissingRequiredCheckedState {
        key: SemanticKey,
        role: SemanticRole,
    },
    CheckedStateNotSupported {
        key: SemanticKey,
        role: SemanticRole,
    },
    MixedCheckedStateNotSupported {
        key: SemanticKey,
        role: SemanticRole,
    },
}

impl fmt::Display for SemanticContributionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateKey { key } => {
                write!(formatter, "duplicate owner-local semantic key `{key}`")
            }
            Self::MissingMountedChildrenMarker => {
                formatter.write_str("semantic contribution is missing its mounted-children marker")
            }
            Self::DuplicateMountedChildrenMarker => formatter
                .write_str("semantic contribution contains more than one mounted-children marker"),
            Self::UnnecessaryMountedChildrenMarker => formatter
                .write_str("semantic contribution contains an unnecessary mounted-children marker"),
            Self::MissingLocalReference { source, target } => write!(
                formatter,
                "semantic node `{source}` references missing owner-local semantic key `{target}`"
            ),
            Self::MissingRequiredCheckedState { key, role } => write!(
                formatter,
                "semantic node `{key}` with role {role:?} requires an authored checked state"
            ),
            Self::CheckedStateNotSupported { key, role } => write!(
                formatter,
                "semantic node `{key}` with role {role:?} does not support checked state"
            ),
            Self::MixedCheckedStateNotSupported { key, role } => write!(
                formatter,
                "semantic node `{key}` with role {role:?} does not support mixed checked state"
            ),
        }
    }
}

impl std::error::Error for SemanticContributionError {}

fn collect_structure(
    items: &[SemanticItem],
    keys: &mut BTreeSet<SemanticKey>,
    ordered_keys: &mut Vec<SemanticKey>,
    marker_count: &mut usize,
) -> Result<(), SemanticContributionError> {
    for item in items {
        match item {
            SemanticItem::MountedChildren => {
                *marker_count = marker_count.saturating_add(1);
            }
            SemanticItem::Node(node) => {
                let key = node.key().clone();
                if !keys.insert(key.clone()) {
                    return Err(SemanticContributionError::DuplicateKey { key });
                }
                ordered_keys.push(key);
                collect_structure(node.children(), keys, ordered_keys, marker_count)?;
            }
        }
    }
    Ok(())
}

fn validate_role_state_contract(
    items: &[SemanticItem],
) -> Result<(), SemanticContributionError> {
    for item in items {
        let SemanticItem::Node(node) = item else {
            continue;
        };
        let checked = node.state().checked();
        match node.role() {
            SemanticRole::Checkbox => match checked {
                None => {
                    return Err(SemanticContributionError::MissingRequiredCheckedState {
                        key: node.key().clone(),
                        role: node.role(),
                    });
                }
                Some(
                    SemanticCheckedState::Unchecked
                    | SemanticCheckedState::Checked
                    | SemanticCheckedState::Mixed,
                ) => {}
                #[allow(unreachable_patterns)]
                Some(_) => {
                    return Err(SemanticContributionError::CheckedStateNotSupported {
                        key: node.key().clone(),
                        role: node.role(),
                    });
                }
            },
            SemanticRole::RadioButton | SemanticRole::Switch => match checked {
                None => {
                    return Err(SemanticContributionError::MissingRequiredCheckedState {
                        key: node.key().clone(),
                        role: node.role(),
                    });
                }
                Some(SemanticCheckedState::Mixed) => {
                    return Err(SemanticContributionError::MixedCheckedStateNotSupported {
                        key: node.key().clone(),
                        role: node.role(),
                    });
                }
                Some(SemanticCheckedState::Unchecked | SemanticCheckedState::Checked) => {}
                #[allow(unreachable_patterns)]
                Some(_) => {
                    return Err(SemanticContributionError::CheckedStateNotSupported {
                        key: node.key().clone(),
                        role: node.role(),
                    });
                }
            },
            _ if checked.is_some() => {
                return Err(SemanticContributionError::CheckedStateNotSupported {
                    key: node.key().clone(),
                    role: node.role(),
                });
            }
            _ => {}
        }
        validate_role_state_contract(node.children())?;
    }
    Ok(())
}

fn validate_local_references(
    items: &[SemanticItem],
    keys: &BTreeSet<SemanticKey>,
) -> Result<(), SemanticContributionError> {
    for item in items {
        let SemanticItem::Node(node) = item else {
            continue;
        };
        for relationship in node.relationships() {
            if let SemanticReference::Local(target) = relationship.target()
                && !keys.contains(target)
            {
                return Err(SemanticContributionError::MissingLocalReference {
                    source: node.key().clone(),
                    target: target.clone(),
                });
            }
        }
        validate_local_references(node.children(), keys)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        SemanticCheckedState, SemanticContribution, SemanticContributionContext,
        SemanticContributionError, SemanticEditable, SemanticItem, SemanticKey,
        SemanticNodeContribution, SemanticReference, SemanticRelationship,
        SemanticRelationshipKind, SemanticRole, SemanticState,
    };
    use crate::{
        TextAffinity, TextDocumentId, TextDocumentRevision, TextDocumentSnapshot, TextPosition,
        TextSelection, TextSensitivity,
    };

    fn group_with_marker() -> SemanticNodeContribution {
        SemanticNodeContribution::primary(SemanticRole::Group).with_mounted_children()
    }

    #[test]
    fn runtime_editable_projection_preserves_caret_offset_allocation_identity() {
        let source = "abc";
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(266), TextDocumentRevision::new(1));
        let position = TextPosition::new(snapshot, source, 1, TextAffinity::Downstream)
            .unwrap_or_else(|_| unreachable!("ASCII fixture position is valid"));
        let selection = TextSelection::collapsed(position);
        let offsets: Arc<[usize]> = vec![0, 1, 2, 3].into();
        let retained = Arc::clone(&offsets);

        let editable =
            SemanticEditable::new(snapshot, source, selection, TextSensitivity::Public, false)
                .and_then(|editable| editable.__runtime_with_projection(source, selection, offsets))
                .unwrap_or_else(|| unreachable!("controlled semantic projection is valid"));
        let published = editable
            .caret_offsets
            .as_ref()
            .unwrap_or_else(|| unreachable!("runtime projection retains caret offsets"));

        assert!(Arc::ptr_eq(&retained, published));
    }

    #[test]
    fn checked_state_contract_is_role_aware_and_fail_closed() {
        let context = SemanticContributionContext::default();
        for checked in [
            SemanticCheckedState::Unchecked,
            SemanticCheckedState::Checked,
            SemanticCheckedState::Mixed,
        ] {
            let checkbox = SemanticNodeContribution::primary(SemanticRole::Checkbox)
                .with_state(SemanticState::ENABLED.with_checked(checked));
            assert!(SemanticContribution::single(checkbox).validate(context).is_ok());
        }

        for role in [SemanticRole::RadioButton, SemanticRole::Switch] {
            for checked in [
                SemanticCheckedState::Unchecked,
                SemanticCheckedState::Checked,
            ] {
                let node = SemanticNodeContribution::primary(role)
                    .with_state(SemanticState::ENABLED.with_checked(checked));
                assert!(SemanticContribution::single(node).validate(context).is_ok());
            }

            let mixed = SemanticNodeContribution::primary(role).with_state(
                SemanticState::ENABLED.with_checked(SemanticCheckedState::Mixed),
            );
            assert_eq!(
                SemanticContribution::single(mixed).validate(context),
                Err(SemanticContributionError::MixedCheckedStateNotSupported {
                    key: SemanticKey::PRIMARY,
                    role,
                })
            );

            let missing = SemanticNodeContribution::primary(role);
            assert_eq!(
                SemanticContribution::single(missing).validate(context),
                Err(SemanticContributionError::MissingRequiredCheckedState {
                    key: SemanticKey::PRIMARY,
                    role,
                })
            );
        }

        let missing_checkbox = SemanticNodeContribution::primary(SemanticRole::Checkbox);
        assert_eq!(
            SemanticContribution::single(missing_checkbox).validate(context),
            Err(SemanticContributionError::MissingRequiredCheckedState {
                key: SemanticKey::PRIMARY,
                role: SemanticRole::Checkbox,
            })
        );

        for role in [
            SemanticRole::Generic,
            SemanticRole::Group,
            SemanticRole::Text,
            SemanticRole::Button,
            SemanticRole::EditableText,
            SemanticRole::RadioGroup,
        ] {
            let node = SemanticNodeContribution::primary(role).with_state(
                SemanticState::ENABLED.with_checked(SemanticCheckedState::Checked),
            );
            assert_eq!(
                SemanticContribution::single(node).validate(context),
                Err(SemanticContributionError::CheckedStateNotSupported {
                    key: SemanticKey::PRIMARY,
                    role,
                })
            );
        }
    }

    #[test]
    fn reserved_primary_and_named_keys_are_distinct() {
        let named = SemanticKey::from_static("primary")
            .unwrap_or_else(|_| unreachable!("static test key is valid"));
        assert!(SemanticKey::PRIMARY.is_primary());
        assert!(!named.is_primary());
        assert_ne!(SemanticKey::PRIMARY, named);
    }

    #[test]
    fn semantic_items_hide_node_storage_while_preserving_typed_variants() {
        let item = SemanticItem::node(SemanticNodeContribution::primary(SemanticRole::Text));
        assert_eq!(
            item.as_node().map(SemanticNodeContribution::role),
            Some(SemanticRole::Text)
        );
        assert!(!item.is_mounted_children());
        let marker = SemanticItem::mounted_children();
        assert!(marker.as_node().is_none());
        assert!(marker.is_mounted_children());
    }

    #[test]
    fn exact_mounted_child_marker_contract_is_validated_without_repair() {
        let children = SemanticContributionContext::__runtime_new(1);
        let leaf = SemanticContributionContext::__runtime_new(0);

        assert!(SemanticContribution::empty().validate(children).is_ok());
        assert!(
            SemanticContribution::single(group_with_marker())
                .validate(children)
                .is_ok()
        );
        assert_eq!(
            SemanticContribution::single(SemanticNodeContribution::primary(SemanticRole::Group))
                .validate(children),
            Err(SemanticContributionError::MissingMountedChildrenMarker)
        );
        assert_eq!(
            SemanticContribution::single(group_with_marker()).validate(leaf),
            Err(SemanticContributionError::UnnecessaryMountedChildrenMarker)
        );
        assert_eq!(
            SemanticContribution::new(vec![
                SemanticItem::node(group_with_marker()),
                SemanticItem::mounted_children(),
            ])
            .validate(children),
            Err(SemanticContributionError::DuplicateMountedChildrenMarker)
        );
    }

    #[test]
    fn duplicate_keys_and_missing_local_relationships_never_first_match() {
        let duplicate = SemanticContribution::new(vec![
            SemanticItem::node(SemanticNodeContribution::primary(SemanticRole::Text)),
            SemanticItem::node(SemanticNodeContribution::primary(SemanticRole::Button)),
        ]);
        assert_eq!(
            duplicate.validate(SemanticContributionContext::default()),
            Err(SemanticContributionError::DuplicateKey {
                key: SemanticKey::PRIMARY,
            })
        );

        let missing = SemanticKey::from_static("missing")
            .unwrap_or_else(|_| unreachable!("static test key is valid"));
        let source = SemanticNodeContribution::primary(SemanticRole::Text).with_relationship(
            SemanticRelationship::new(
                SemanticRelationshipKind::DescribedBy,
                SemanticReference::Local(missing.clone()),
            ),
        );
        assert_eq!(
            SemanticContribution::single(source).validate(SemanticContributionContext::default()),
            Err(SemanticContributionError::MissingLocalReference {
                source: SemanticKey::PRIMARY,
                target: missing,
            })
        );
    }
}
