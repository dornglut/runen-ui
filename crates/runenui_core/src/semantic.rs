//! Platform-neutral semantic contribution vocabulary.
//!
//! Widgets author owner-local semantic facts. Runtime owns live semantic IDs,
//! mounted ownership, absolute bounds, focus, publication revisions, and action
//! routing. Nothing in this module is tied to AccessKit or a native host API.

use core::fmt;
use std::{collections::BTreeSet, sync::Arc};

use crate::identity::{IdentifierText, validate_identifier};
use crate::{
    ElementId, IdentifierError, LogicalRect, ScrollControlSnapshot, TextDocumentSnapshot,
    TextSelection, TextSensitivity,
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
    Link,
    Image,
    ComboBox,
    Slider,
    ScrollBar,
    Progress,
    SpinButton,
    ListBox,
    Option,
    TabList,
    Tab,
    TabPanel,
    Toolbar,
    Menu,
    MenuBar,
    MenuItem,
    MenuItemCheckbox,
    MenuItemRadio,
    Dialog,
    Tooltip,
    Separator,
    Splitter,
    Tree,
    TreeItem,
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

impl From<bool> for SemanticCheckedState {
    fn from(checked: bool) -> Self {
        if checked {
            Self::Checked
        } else {
            Self::Unchecked
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticPressedState {
    Unpressed,
    Pressed,
    Mixed,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticInvalidState {
    Invalid,
    Grammar,
    Spelling,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticPopupKind {
    Menu,
    ListBox,
    Dialog,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticOrientation {
    Horizontal,
    Vertical,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticSelectionMode {
    Single,
    Multiple,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticAutocomplete {
    None,
    Inline,
    List,
    Both,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticEditableMode {
    SingleLine,
    Multiline,
}

/// Finite semantic numeric value with deterministic equality and hashing.
///
/// Negative zero is canonicalized to positive zero. NaN and infinities are rejected.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct SemanticNumber(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticNumberError;

impl SemanticNumber {
    /// Creates a finite semantic number.
    ///
    /// # Errors
    ///
    /// Returns `SemanticNumberError` for NaN or either infinity.
    pub fn new(value: f64) -> Result<Self, SemanticNumberError> {
        if !value.is_finite() {
            return Err(SemanticNumberError);
        }
        let value = if value == 0.0 { 0.0 } else { value };
        Ok(Self(value.to_bits()))
    }

    #[must_use]
    pub const fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

impl fmt::Debug for SemanticNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(formatter)
    }
}

impl fmt::Display for SemanticNumberError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("semantic number must be finite")
    }
}

impl std::error::Error for SemanticNumberError {}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticRangeError {
    ReversedBounds,
    CurrentBelowMinimum,
    CurrentAboveMaximum,
    NonPositiveSmallStep,
    NonPositiveLargeStep,
    ValueTextWithoutCurrent,
}

impl fmt::Display for SemanticRangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ReversedBounds => "semantic range minimum exceeds maximum",
            Self::CurrentBelowMinimum => "semantic range current value is below minimum",
            Self::CurrentAboveMaximum => "semantic range current value is above maximum",
            Self::NonPositiveSmallStep => "semantic range small step must be positive",
            Self::NonPositiveLargeStep => "semantic range large step must be positive",
            Self::ValueTextWithoutCurrent => "semantic range value text requires a current value",
        })
    }
}

impl std::error::Error for SemanticRangeError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticRange {
    minimum: Option<SemanticNumber>,
    maximum: Option<SemanticNumber>,
    current: Option<SemanticNumber>,
    small_step: Option<SemanticNumber>,
    large_step: Option<SemanticNumber>,
    value_text: Option<String>,
}

impl SemanticRange {
    /// Creates a checked semantic range.
    ///
    /// # Errors
    ///
    /// Rejects reversed bounds or a current value outside authored bounds.
    pub fn new(
        minimum: Option<SemanticNumber>,
        maximum: Option<SemanticNumber>,
        current: Option<SemanticNumber>,
    ) -> Result<Self, SemanticRangeError> {
        if minimum
            .zip(maximum)
            .is_some_and(|(minimum, maximum)| minimum.get() > maximum.get())
        {
            return Err(SemanticRangeError::ReversedBounds);
        }
        if current
            .zip(minimum)
            .is_some_and(|(current, minimum)| current.get() < minimum.get())
        {
            return Err(SemanticRangeError::CurrentBelowMinimum);
        }
        if current
            .zip(maximum)
            .is_some_and(|(current, maximum)| current.get() > maximum.get())
        {
            return Err(SemanticRangeError::CurrentAboveMaximum);
        }
        Ok(Self {
            minimum,
            maximum,
            current,
            small_step: None,
            large_step: None,
            value_text: None,
        })
    }

    /// Adds a positive small increment.
    ///
    /// # Errors
    ///
    /// Rejects zero or negative steps.
    pub fn with_small_step(mut self, step: SemanticNumber) -> Result<Self, SemanticRangeError> {
        if step.get() <= 0.0 {
            return Err(SemanticRangeError::NonPositiveSmallStep);
        }
        self.small_step = Some(step);
        Ok(self)
    }

    /// Adds a positive larger/page increment.
    ///
    /// # Errors
    ///
    /// Rejects zero or negative steps.
    pub fn with_large_step(mut self, step: SemanticNumber) -> Result<Self, SemanticRangeError> {
        if step.get() <= 0.0 {
            return Err(SemanticRangeError::NonPositiveLargeStep);
        }
        self.large_step = Some(step);
        Ok(self)
    }

    /// Adds human-readable value text for a determinate current value.
    ///
    /// # Errors
    ///
    /// Rejects value text when the range has no current value.
    pub fn with_value_text(
        mut self,
        value_text: impl Into<String>,
    ) -> Result<Self, SemanticRangeError> {
        if self.current.is_none() {
            return Err(SemanticRangeError::ValueTextWithoutCurrent);
        }
        self.value_text = Some(value_text.into());
        Ok(self)
    }

    #[must_use]
    pub const fn minimum(&self) -> Option<SemanticNumber> {
        self.minimum
    }
    #[must_use]
    pub const fn maximum(&self) -> Option<SemanticNumber> {
        self.maximum
    }
    #[must_use]
    pub const fn current(&self) -> Option<SemanticNumber> {
        self.current
    }
    #[must_use]
    pub const fn small_step(&self) -> Option<SemanticNumber> {
        self.small_step
    }
    #[must_use]
    pub const fn large_step(&self) -> Option<SemanticNumber> {
        self.large_step
    }
    #[must_use]
    pub fn value_text(&self) -> Option<&str> {
        self.value_text.as_deref()
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticCollectionPositionError {
    EmptyKnownSet,
    PositionOutsideKnownSet,
}

impl fmt::Display for SemanticCollectionPositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptyKnownSet => "known semantic collection size must be non-zero",
            Self::PositionOutsideKnownSet => {
                "semantic collection position is outside the known set"
            }
        })
    }
}

impl std::error::Error for SemanticCollectionPositionError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SemanticCollectionPosition {
    index: u64,
    known_size: Option<u64>,
}

impl SemanticCollectionPosition {
    /// Creates zero-based collection position metadata.
    ///
    /// # Errors
    ///
    /// Rejects zero known size and positions outside a known set.
    pub const fn new(
        index: u64,
        known_size: Option<u64>,
    ) -> Result<Self, SemanticCollectionPositionError> {
        if let Some(known_size) = known_size {
            if known_size == 0 {
                return Err(SemanticCollectionPositionError::EmptyKnownSet);
            }
            if index >= known_size {
                return Err(SemanticCollectionPositionError::PositionOutsideKnownSet);
            }
        }
        Ok(Self { index, known_size })
    }

    #[must_use]
    pub const fn index(self) -> u64 {
        self.index
    }

    #[must_use]
    pub const fn known_size(self) -> Option<u64> {
        self.known_size
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SemanticHierarchyLevel(u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticHierarchyLevelError;

impl SemanticHierarchyLevel {
    /// Creates a positive one-based hierarchy level.
    ///
    /// # Errors
    ///
    /// Rejects level zero.
    pub const fn new(level: u32) -> Result<Self, SemanticHierarchyLevelError> {
        if level == 0 {
            Err(SemanticHierarchyLevelError)
        } else {
            Ok(Self(level))
        }
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SemanticHierarchyLevelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("semantic hierarchy level must be positive")
    }
}

impl std::error::Error for SemanticHierarchyLevelError {}

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
    read_only_authored: bool,
    checked: Option<SemanticCheckedState>,
    pressed: Option<SemanticPressedState>,
    selected: Option<bool>,
    expanded: Option<bool>,
    required: Option<bool>,
    invalid: Option<SemanticInvalidState>,
    modal: Option<bool>,
}

impl SemanticState {
    pub const ENABLED: Self = Self {
        disabled: false,
        hidden: false,
        inert: false,
        read_only: false,
        read_only_authored: false,
        checked: None,
        pressed: None,
        selected: None,
        expanded: None,
        required: None,
        invalid: None,
        modal: None,
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
        self.read_only_authored = true;
        self
    }

    /// Authors the exact checked state for a checkable semantic role.
    #[must_use]
    pub const fn with_checked(mut self, checked: SemanticCheckedState) -> Self {
        self.checked = Some(checked);
        self
    }

    #[must_use]
    pub const fn with_pressed(mut self, pressed: SemanticPressedState) -> Self {
        self.pressed = Some(pressed);
        self
    }

    #[must_use]
    pub const fn with_selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    #[must_use]
    pub const fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }

    #[must_use]
    pub const fn with_required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }

    #[must_use]
    pub const fn with_invalid(mut self, invalid: SemanticInvalidState) -> Self {
        self.invalid = Some(invalid);
        self
    }

    #[must_use]
    pub const fn with_modal(mut self, modal: bool) -> Self {
        self.modal = Some(modal);
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

    #[must_use]
    pub const fn read_only_is_authored(self) -> bool {
        self.read_only_authored
    }

    #[must_use]
    pub const fn pressed(self) -> Option<SemanticPressedState> {
        self.pressed
    }

    #[must_use]
    pub const fn selected(self) -> Option<bool> {
        self.selected
    }

    #[must_use]
    pub const fn expanded(self) -> Option<bool> {
        self.expanded
    }

    #[must_use]
    pub const fn required(self) -> Option<bool> {
        self.required
    }

    #[must_use]
    pub const fn invalid(self) -> Option<SemanticInvalidState> {
        self.invalid
    }

    #[must_use]
    pub const fn modal(self) -> Option<bool> {
        self.modal
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
    Increment,
    Decrement,
    SetValue,
    Expand,
    Collapse,
}

/// Relationship category expressed without platform-adapter vocabulary.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticRelationshipKind {
    LabelledBy,
    DescribedBy,
    Controls,
    ErrorMessage,
    ActiveDescendant,
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
    range: Option<SemanticRange>,
    orientation: Option<SemanticOrientation>,
    popup: Option<SemanticPopupKind>,
    selection_mode: Option<SemanticSelectionMode>,
    collection_position: Option<SemanticCollectionPosition>,
    hierarchy_level: Option<SemanticHierarchyLevel>,
    placeholder: Option<String>,
    autocomplete: Option<SemanticAutocomplete>,
    editable_mode: Option<SemanticEditableMode>,
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
            range: None,
            orientation: None,
            popup: None,
            selection_mode: None,
            collection_position: None,
            hierarchy_level: None,
            placeholder: None,
            autocomplete: None,
            editable_mode: None,
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
    pub fn with_range(mut self, range: SemanticRange) -> Self {
        self.0.range = Some(range);
        self
    }

    #[must_use]
    pub fn with_orientation(mut self, orientation: SemanticOrientation) -> Self {
        self.0.orientation = Some(orientation);
        self
    }

    #[must_use]
    pub fn with_popup(mut self, popup: SemanticPopupKind) -> Self {
        self.0.popup = Some(popup);
        self
    }

    #[must_use]
    pub fn with_selection_mode(mut self, selection_mode: SemanticSelectionMode) -> Self {
        self.0.selection_mode = Some(selection_mode);
        self
    }

    #[must_use]
    pub fn with_collection_position(
        mut self,
        collection_position: SemanticCollectionPosition,
    ) -> Self {
        self.0.collection_position = Some(collection_position);
        self
    }

    #[must_use]
    pub fn with_hierarchy_level(mut self, hierarchy_level: SemanticHierarchyLevel) -> Self {
        self.0.hierarchy_level = Some(hierarchy_level);
        self
    }

    #[must_use]
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.0.placeholder = Some(placeholder.into());
        self
    }

    #[must_use]
    pub fn with_autocomplete(mut self, autocomplete: SemanticAutocomplete) -> Self {
        self.0.autocomplete = Some(autocomplete);
        self
    }

    #[must_use]
    pub fn with_editable_mode(mut self, editable_mode: SemanticEditableMode) -> Self {
        self.0.editable_mode = Some(editable_mode);
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
    pub const fn range(&self) -> Option<&SemanticRange> {
        self.0.range.as_ref()
    }

    #[must_use]
    pub const fn orientation(&self) -> Option<SemanticOrientation> {
        self.0.orientation
    }

    #[must_use]
    pub const fn popup(&self) -> Option<SemanticPopupKind> {
        self.0.popup
    }

    #[must_use]
    pub const fn selection_mode(&self) -> Option<SemanticSelectionMode> {
        self.0.selection_mode
    }

    #[must_use]
    pub const fn collection_position(&self) -> Option<SemanticCollectionPosition> {
        self.0.collection_position
    }

    #[must_use]
    pub const fn hierarchy_level(&self) -> Option<SemanticHierarchyLevel> {
        self.0.hierarchy_level
    }

    #[must_use]
    pub fn placeholder(&self) -> Option<&str> {
        self.0.placeholder.as_deref()
    }

    #[must_use]
    pub const fn autocomplete(&self) -> Option<SemanticAutocomplete> {
        self.0.autocomplete
    }

    #[must_use]
    pub const fn editable_mode(&self) -> Option<SemanticEditableMode> {
        self.0.editable_mode
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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SemanticContributionContext {
    direct_mounted_children: usize,
    scroll_control: Option<ScrollControlSnapshot>,
}

impl SemanticContributionContext {
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(direct_mounted_children: usize) -> Self {
        Self {
            direct_mounted_children,
            scroll_control: None,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_with_scroll_control(
        direct_mounted_children: usize,
        scroll_control: Option<ScrollControlSnapshot>,
    ) -> Self {
        Self {
            direct_mounted_children,
            scroll_control,
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

    /// Returns the runtime-derived bound scroll snapshot when this owner is a bound control.
    #[must_use]
    pub const fn scroll_control_snapshot(self) -> Option<ScrollControlSnapshot> {
        self.scroll_control
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
    MissingRequiredProperty {
        key: SemanticKey,
        role: SemanticRole,
        property: &'static str,
    },
    PropertyNotSupported {
        key: SemanticKey,
        role: SemanticRole,
        property: &'static str,
    },
    PopupKindNotSupported {
        key: SemanticKey,
        role: SemanticRole,
        popup: SemanticPopupKind,
    },
    RelationshipNotSupported {
        key: SemanticKey,
        role: SemanticRole,
        kind: SemanticRelationshipKind,
    },
    DuplicateSingularRelationship {
        key: SemanticKey,
        kind: SemanticRelationshipKind,
    },
    ActiveDescendantRequiresControls {
        key: SemanticKey,
    },
    ActiveDescendantNotSupportedForDialogPopup {
        key: SemanticKey,
    },
    EditableCombinationNotSupported {
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
            Self::MissingRequiredProperty {
                key,
                role,
                property,
            } => write!(
                formatter,
                "semantic node `{key}` with role {role:?} requires semantic property `{property}`"
            ),
            Self::PropertyNotSupported {
                key,
                role,
                property,
            } => write!(
                formatter,
                "semantic node `{key}` with role {role:?} does not support semantic property `{property}`"
            ),
            Self::PopupKindNotSupported { key, role, popup } => write!(
                formatter,
                "semantic node `{key}` with role {role:?} does not support popup kind {popup:?}"
            ),
            Self::RelationshipNotSupported { key, role, kind } => write!(
                formatter,
                "semantic node `{key}` with role {role:?} does not support relationship {kind:?}"
            ),
            Self::DuplicateSingularRelationship { key, kind } => write!(
                formatter,
                "semantic node `{key}` has more than one singular relationship {kind:?}"
            ),
            Self::ActiveDescendantRequiresControls { key } => write!(
                formatter,
                "semantic ComboBox node `{key}` with ActiveDescendant must also control its popup"
            ),
            Self::ActiveDescendantNotSupportedForDialogPopup { key } => write!(
                formatter,
                "semantic ComboBox node `{key}` cannot expose ActiveDescendant for a dialog popup"
            ),
            Self::EditableCombinationNotSupported { key, role } => write!(
                formatter,
                "semantic node `{key}` with role {role:?} has an unsupported editable role/mode/sensitivity combination"
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

fn validate_role_state_contract(items: &[SemanticItem]) -> Result<(), SemanticContributionError> {
    for item in items {
        let SemanticItem::Node(node) = item else {
            continue;
        };
        validate_checked_contract(node)?;
        validate_authored_state_contract(node)?;
        validate_range_contract(node)?;
        validate_node_property_contract(node)?;
        validate_editable_contract(node)?;
        validate_relationship_contract(node)?;
        validate_role_state_contract(node.children())?;
    }
    Ok(())
}

fn validate_checked_contract(
    node: &SemanticNodeContribution,
) -> Result<(), SemanticContributionError> {
    let role = node.role();
    match role {
        SemanticRole::Checkbox | SemanticRole::MenuItemCheckbox => match node.state().checked() {
            None => Err(SemanticContributionError::MissingRequiredCheckedState {
                key: node.key().clone(),
                role,
            }),
            Some(
                SemanticCheckedState::Unchecked
                | SemanticCheckedState::Checked
                | SemanticCheckedState::Mixed,
            ) => Ok(()),
            #[allow(unreachable_patterns)]
            Some(_) => Err(SemanticContributionError::CheckedStateNotSupported {
                key: node.key().clone(),
                role,
            }),
        },
        SemanticRole::RadioButton | SemanticRole::Switch | SemanticRole::MenuItemRadio => {
            match node.state().checked() {
                None => Err(SemanticContributionError::MissingRequiredCheckedState {
                    key: node.key().clone(),
                    role,
                }),
                Some(SemanticCheckedState::Mixed) => {
                    Err(SemanticContributionError::MixedCheckedStateNotSupported {
                        key: node.key().clone(),
                        role,
                    })
                }
                Some(SemanticCheckedState::Unchecked | SemanticCheckedState::Checked) => Ok(()),
                #[allow(unreachable_patterns)]
                Some(_) => Err(SemanticContributionError::CheckedStateNotSupported {
                    key: node.key().clone(),
                    role,
                }),
            }
        }
        _ if node.state().checked().is_some() => {
            Err(SemanticContributionError::CheckedStateNotSupported {
                key: node.key().clone(),
                role,
            })
        }
        _ => Ok(()),
    }
}

const fn is_input_state_role(role: SemanticRole) -> bool {
    matches!(
        role,
        SemanticRole::EditableText
            | SemanticRole::ComboBox
            | SemanticRole::SpinButton
            | SemanticRole::ListBox
    )
}

fn validate_authored_state_contract(
    node: &SemanticNodeContribution,
) -> Result<(), SemanticContributionError> {
    let role = node.role();
    let state = node.state();
    if state.pressed().is_some() && role != SemanticRole::Button {
        return property_not_supported(node, "pressed");
    }
    if state.selected().is_some()
        && !matches!(
            role,
            SemanticRole::Option | SemanticRole::Tab | SemanticRole::TreeItem
        )
    {
        return property_not_supported(node, "selected");
    }
    if state.expanded().is_some()
        && !matches!(
            role,
            SemanticRole::Button
                | SemanticRole::ComboBox
                | SemanticRole::MenuItem
                | SemanticRole::MenuItemCheckbox
                | SemanticRole::MenuItemRadio
                | SemanticRole::TreeItem
        )
    {
        return property_not_supported(node, "expanded");
    }
    let input_state_role = is_input_state_role(role);
    if state.required().is_some() && !input_state_role {
        return property_not_supported(node, "required");
    }
    if state.invalid().is_some() && !input_state_role {
        return property_not_supported(node, "invalid");
    }
    if state.read_only_is_authored() && !input_state_role {
        return property_not_supported(node, "read_only");
    }
    if state.modal().is_some() && role != SemanticRole::Dialog {
        return property_not_supported(node, "modal");
    }
    Ok(())
}

fn validate_range_contract(
    node: &SemanticNodeContribution,
) -> Result<(), SemanticContributionError> {
    match node.role() {
        SemanticRole::Slider | SemanticRole::Splitter => {
            let range = required_range(node)?;
            if range.minimum().is_none() {
                return missing_property(node, "range.minimum");
            }
            if range.maximum().is_none() {
                return missing_property(node, "range.maximum");
            }
            if range.current().is_none() {
                return missing_property(node, "range.current");
            }
        }
        SemanticRole::ScrollBar if node.range().is_some() => {
            return property_not_supported(node, "range");
        }
        SemanticRole::Progress => {
            let range = required_range(node)?;
            if range.minimum().is_none() {
                return missing_property(node, "range.minimum");
            }
            if range.maximum().is_none() {
                return missing_property(node, "range.maximum");
            }
        }
        SemanticRole::SpinButton => {
            let _ = required_range(node)?;
        }
        _ if node.range().is_some() => return property_not_supported(node, "range"),
        _ => {}
    }
    Ok(())
}

fn validate_node_property_contract(
    node: &SemanticNodeContribution,
) -> Result<(), SemanticContributionError> {
    let role = node.role();
    if node.orientation().is_some()
        && !matches!(
            role,
            SemanticRole::Slider
                | SemanticRole::ListBox
                | SemanticRole::TabList
                | SemanticRole::Toolbar
                | SemanticRole::Menu
                | SemanticRole::MenuBar
                | SemanticRole::Separator
                | SemanticRole::Splitter
        )
    {
        return property_not_supported(node, "orientation");
    }
    if let Some(popup) = node.popup() {
        let role_supports_popup = matches!(
            role,
            SemanticRole::Button
                | SemanticRole::ComboBox
                | SemanticRole::MenuItem
                | SemanticRole::MenuItemCheckbox
                | SemanticRole::MenuItemRadio
        );
        if !role_supports_popup {
            return property_not_supported(node, "popup");
        }
        let kind_supported = match role {
            SemanticRole::Button => true,
            SemanticRole::ComboBox => {
                matches!(
                    popup,
                    SemanticPopupKind::ListBox | SemanticPopupKind::Dialog
                )
            }
            SemanticRole::MenuItem
            | SemanticRole::MenuItemCheckbox
            | SemanticRole::MenuItemRadio => popup == SemanticPopupKind::Menu,
            _ => false,
        };
        if !kind_supported {
            return Err(SemanticContributionError::PopupKindNotSupported {
                key: node.key().clone(),
                role,
                popup,
            });
        }
    }
    if node.selection_mode().is_some()
        && !matches!(role, SemanticRole::ListBox | SemanticRole::Tree)
    {
        return property_not_supported(node, "selection_mode");
    }
    if node.collection_position().is_some()
        && !matches!(role, SemanticRole::Option | SemanticRole::TreeItem)
    {
        return property_not_supported(node, "collection_position");
    }
    if node.hierarchy_level().is_some() && role != SemanticRole::TreeItem {
        return property_not_supported(node, "hierarchy_level");
    }
    if node.placeholder().is_some()
        && !matches!(
            role,
            SemanticRole::EditableText | SemanticRole::ComboBox | SemanticRole::SpinButton
        )
    {
        return property_not_supported(node, "placeholder");
    }
    if node.autocomplete().is_some() && role != SemanticRole::ComboBox {
        return property_not_supported(node, "autocomplete");
    }
    Ok(())
}

fn validate_editable_contract(
    node: &SemanticNodeContribution,
) -> Result<(), SemanticContributionError> {
    let role = node.role();
    let compatible_role = matches!(
        role,
        SemanticRole::EditableText | SemanticRole::ComboBox | SemanticRole::SpinButton
    );
    if let Some(mode) = node.editable_mode()
        && (node.editable().is_none()
            || !compatible_role
            || (mode == SemanticEditableMode::Multiline && role != SemanticRole::EditableText))
    {
        return Err(SemanticContributionError::EditableCombinationNotSupported {
            key: node.key().clone(),
            role,
        });
    }
    if let Some(editable) = node.editable() {
        let secret_incompatible = editable.sensitivity() == TextSensitivity::Secret
            && (node.editable_mode() == Some(SemanticEditableMode::Multiline)
                || matches!(role, SemanticRole::ComboBox | SemanticRole::SpinButton));
        if !compatible_role || secret_incompatible {
            return Err(SemanticContributionError::EditableCombinationNotSupported {
                key: node.key().clone(),
                role,
            });
        }
    }
    Ok(())
}

fn validate_relationship_contract(
    node: &SemanticNodeContribution,
) -> Result<(), SemanticContributionError> {
    let role = node.role();
    let mut active_descendant_count = 0usize;
    let mut has_controls = false;
    for relationship in node.relationships() {
        match relationship.kind() {
            SemanticRelationshipKind::Controls if role == SemanticRole::ScrollBar => {
                return Err(SemanticContributionError::RelationshipNotSupported {
                    key: node.key().clone(),
                    role,
                    kind: relationship.kind(),
                });
            }
            SemanticRelationshipKind::ErrorMessage if !is_input_state_role(role) => {
                return Err(SemanticContributionError::RelationshipNotSupported {
                    key: node.key().clone(),
                    role,
                    kind: relationship.kind(),
                });
            }
            SemanticRelationshipKind::ActiveDescendant => {
                if role != SemanticRole::ComboBox {
                    return Err(SemanticContributionError::RelationshipNotSupported {
                        key: node.key().clone(),
                        role,
                        kind: relationship.kind(),
                    });
                }
                active_descendant_count = active_descendant_count.saturating_add(1);
            }
            SemanticRelationshipKind::Controls => has_controls = true,
            _ => {}
        }
    }
    if active_descendant_count > 1 {
        return Err(SemanticContributionError::DuplicateSingularRelationship {
            key: node.key().clone(),
            kind: SemanticRelationshipKind::ActiveDescendant,
        });
    }
    if active_descendant_count == 1 && node.popup() == Some(SemanticPopupKind::Dialog) {
        return Err(
            SemanticContributionError::ActiveDescendantNotSupportedForDialogPopup {
                key: node.key().clone(),
            },
        );
    }
    if active_descendant_count == 1 && !has_controls {
        return Err(
            SemanticContributionError::ActiveDescendantRequiresControls {
                key: node.key().clone(),
            },
        );
    }
    Ok(())
}

fn property_not_supported(
    node: &SemanticNodeContribution,
    property: &'static str,
) -> Result<(), SemanticContributionError> {
    Err(SemanticContributionError::PropertyNotSupported {
        key: node.key().clone(),
        role: node.role(),
        property,
    })
}

fn missing_property(
    node: &SemanticNodeContribution,
    property: &'static str,
) -> Result<(), SemanticContributionError> {
    Err(SemanticContributionError::MissingRequiredProperty {
        key: node.key().clone(),
        role: node.role(),
        property,
    })
}

fn required_range(
    node: &SemanticNodeContribution,
) -> Result<&SemanticRange, SemanticContributionError> {
    node.range()
        .ok_or_else(|| SemanticContributionError::MissingRequiredProperty {
            key: node.key().clone(),
            role: node.role(),
            property: "range",
        })
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
        SemanticContributionError, SemanticEditable, SemanticEditableMode, SemanticItem,
        SemanticKey, SemanticNodeContribution, SemanticNumber, SemanticNumberError,
        SemanticPopupKind, SemanticPressedState, SemanticRange, SemanticRangeError,
        SemanticReference, SemanticRelationship, SemanticRelationshipKind, SemanticRole,
        SemanticSelectionMode, SemanticState,
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
            assert!(
                SemanticContribution::single(checkbox)
                    .validate(context)
                    .is_ok()
            );
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

            let mixed = SemanticNodeContribution::primary(role)
                .with_state(SemanticState::ENABLED.with_checked(SemanticCheckedState::Mixed));
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

        let radio_group = SemanticNodeContribution::primary(SemanticRole::RadioGroup);
        assert!(
            SemanticContribution::single(radio_group)
                .validate(context)
                .is_ok()
        );

        for role in [
            SemanticRole::Generic,
            SemanticRole::Group,
            SemanticRole::Text,
            SemanticRole::Button,
            SemanticRole::EditableText,
            SemanticRole::RadioGroup,
        ] {
            let node = SemanticNodeContribution::primary(role)
                .with_state(SemanticState::ENABLED.with_checked(SemanticCheckedState::Checked));
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
    fn semantic_numbers_and_ranges_are_checked_and_deterministic() {
        let zero = SemanticNumber::new(0.0)
            .unwrap_or_else(|_| unreachable!("finite zero is a valid semantic number"));
        let negative_zero = SemanticNumber::new(-0.0)
            .unwrap_or_else(|_| unreachable!("finite negative zero is canonicalized"));
        assert_eq!(zero, negative_zero);
        assert_eq!(SemanticNumber::new(f64::NAN), Err(SemanticNumberError));
        assert_eq!(SemanticNumber::new(f64::INFINITY), Err(SemanticNumberError));

        let minimum = SemanticNumber::new(0.0)
            .unwrap_or_else(|_| unreachable!("controlled minimum is finite"));
        let maximum = SemanticNumber::new(10.0)
            .unwrap_or_else(|_| unreachable!("controlled maximum is finite"));
        let current = SemanticNumber::new(5.0)
            .unwrap_or_else(|_| unreachable!("controlled current is finite"));
        let step =
            SemanticNumber::new(1.0).unwrap_or_else(|_| unreachable!("controlled step is finite"));

        let range = SemanticRange::new(Some(minimum), Some(maximum), Some(current))
            .and_then(|range| range.with_small_step(step))
            .and_then(|range| range.with_value_text("five"))
            .unwrap_or_else(|_| unreachable!("controlled range is valid"));
        assert_eq!(range.minimum(), Some(minimum));
        assert_eq!(range.maximum(), Some(maximum));
        assert_eq!(range.current(), Some(current));
        assert_eq!(range.value_text(), Some("five"));

        assert_eq!(
            SemanticRange::new(Some(maximum), Some(minimum), Some(current)),
            Err(SemanticRangeError::ReversedBounds)
        );
        assert_eq!(
            SemanticRange::new(Some(minimum), Some(maximum), None)
                .and_then(|range| range.with_value_text("unknown")),
            Err(SemanticRangeError::ValueTextWithoutCurrent)
        );
        let negative_step = SemanticNumber::new(-1.0)
            .unwrap_or_else(|_| unreachable!("finite negative value is representable"));
        assert_eq!(
            SemanticRange::new(Some(minimum), Some(maximum), Some(current))
                .and_then(|range| range.with_small_step(negative_step)),
            Err(SemanticRangeError::NonPositiveSmallStep)
        );
    }

    #[test]
    fn standard_control_state_properties_are_role_aware_and_fail_closed() {
        let context = SemanticContributionContext::default();

        let pressed = SemanticNodeContribution::primary(SemanticRole::Button)
            .with_state(SemanticState::ENABLED.with_pressed(SemanticPressedState::Pressed));
        assert!(
            SemanticContribution::single(pressed)
                .validate(context)
                .is_ok()
        );

        let invalid_pressed = SemanticNodeContribution::primary(SemanticRole::Generic)
            .with_state(SemanticState::ENABLED.with_pressed(SemanticPressedState::Pressed));
        assert_eq!(
            SemanticContribution::single(invalid_pressed).validate(context),
            Err(SemanticContributionError::PropertyNotSupported {
                key: SemanticKey::PRIMARY,
                role: SemanticRole::Generic,
                property: "pressed",
            })
        );

        let selected = SemanticNodeContribution::primary(SemanticRole::Option)
            .with_state(SemanticState::ENABLED.with_selected(false));
        assert!(
            SemanticContribution::single(selected)
                .validate(context)
                .is_ok()
        );
    }

    #[test]
    fn standard_control_range_collection_and_relationship_properties_are_validated() {
        let context = SemanticContributionContext::default();

        let missing_range = SemanticNodeContribution::primary(SemanticRole::Slider);
        assert_eq!(
            SemanticContribution::single(missing_range).validate(context),
            Err(SemanticContributionError::MissingRequiredProperty {
                key: SemanticKey::PRIMARY,
                role: SemanticRole::Slider,
                property: "range",
            })
        );

        let minimum = SemanticNumber::new(0.0)
            .unwrap_or_else(|_| unreachable!("controlled minimum is finite"));
        let maximum = SemanticNumber::new(100.0)
            .unwrap_or_else(|_| unreachable!("controlled maximum is finite"));
        let current = SemanticNumber::new(25.0)
            .unwrap_or_else(|_| unreachable!("controlled current is finite"));
        let slider = SemanticNodeContribution::primary(SemanticRole::Slider).with_range(
            SemanticRange::new(Some(minimum), Some(maximum), Some(current))
                .unwrap_or_else(|_| unreachable!("controlled slider range is valid")),
        );
        assert!(
            SemanticContribution::single(slider)
                .validate(context)
                .is_ok()
        );

        let scrollbar = SemanticNodeContribution::primary(SemanticRole::ScrollBar);
        assert!(
            SemanticContribution::single(scrollbar)
                .validate(context)
                .is_ok()
        );

        let authored_scrollbar_range =
            SemanticNodeContribution::primary(SemanticRole::ScrollBar).with_range(
                SemanticRange::new(Some(minimum), Some(maximum), Some(current))
                    .unwrap_or_else(|_| unreachable!("controlled scrollbar range is valid")),
            );
        assert_eq!(
            SemanticContribution::single(authored_scrollbar_range).validate(context),
            Err(SemanticContributionError::PropertyNotSupported {
                key: SemanticKey::PRIMARY,
                role: SemanticRole::ScrollBar,
                property: "range",
            })
        );

        let authored_scrollbar_orientation =
            SemanticNodeContribution::primary(SemanticRole::ScrollBar)
                .with_orientation(SemanticOrientation::Horizontal);
        assert_eq!(
            SemanticContribution::single(authored_scrollbar_orientation).validate(context),
            Err(SemanticContributionError::PropertyNotSupported {
                key: SemanticKey::PRIMARY,
                role: SemanticRole::ScrollBar,
                property: "orientation",
            })
        );

        let listbox = SemanticNodeContribution::primary(SemanticRole::ListBox)
            .with_selection_mode(SemanticSelectionMode::Multiple);
        assert!(
            SemanticContribution::single(listbox)
                .validate(context)
                .is_ok()
        );

        let invalid_selection_mode = SemanticNodeContribution::primary(SemanticRole::TabList)
            .with_selection_mode(SemanticSelectionMode::Multiple);
        assert_eq!(
            SemanticContribution::single(invalid_selection_mode).validate(context),
            Err(SemanticContributionError::PropertyNotSupported {
                key: SemanticKey::PRIMARY,
                role: SemanticRole::TabList,
                property: "selection_mode",
            })
        );
    }

    #[test]
    fn popup_kind_contract_is_role_aware_and_fail_closed() {
        let context = SemanticContributionContext::default();

        let combo_menu = SemanticNodeContribution::primary(SemanticRole::ComboBox)
            .with_popup(SemanticPopupKind::Menu);
        assert_eq!(
            SemanticContribution::single(combo_menu).validate(context),
            Err(SemanticContributionError::PopupKindNotSupported {
                key: SemanticKey::PRIMARY,
                role: SemanticRole::ComboBox,
                popup: SemanticPopupKind::Menu,
            })
        );

        let menu_item_listbox = SemanticNodeContribution::primary(SemanticRole::MenuItem)
            .with_popup(SemanticPopupKind::ListBox);
        assert_eq!(
            SemanticContribution::single(menu_item_listbox).validate(context),
            Err(SemanticContributionError::PopupKindNotSupported {
                key: SemanticKey::PRIMARY,
                role: SemanticRole::MenuItem,
                popup: SemanticPopupKind::ListBox,
            })
        );

        for popup in [
            SemanticPopupKind::Menu,
            SemanticPopupKind::ListBox,
            SemanticPopupKind::Dialog,
        ] {
            let button = SemanticNodeContribution::primary(SemanticRole::Button).with_popup(popup);
            assert!(
                SemanticContribution::single(button)
                    .validate(context)
                    .is_ok()
            );
        }
    }

    #[test]
    fn scrollbar_controls_relationship_is_runtime_derived_only() {
        let context = SemanticContributionContext::default();
        let target = SemanticKey::from_static("viewport")
            .unwrap_or_else(|_| unreachable!("static test key is valid"));
        let scrollbar = SemanticNodeContribution::primary(SemanticRole::ScrollBar)
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::Controls,
                SemanticReference::Local(target.clone()),
            ))
            .with_child(SemanticNodeContribution::new(target, SemanticRole::Group));

        assert_eq!(
            SemanticContribution::single(scrollbar).validate(context),
            Err(SemanticContributionError::RelationshipNotSupported {
                key: SemanticKey::PRIMARY,
                role: SemanticRole::ScrollBar,
                kind: SemanticRelationshipKind::Controls,
            })
        );
    }

    #[test]
    fn active_descendant_relationship_is_validated() {
        let context = SemanticContributionContext::default();

        let option_key = SemanticKey::from_static("active-option")
            .unwrap_or_else(|_| unreachable!("static test key is valid"));
        let combobox = SemanticNodeContribution::primary(SemanticRole::ComboBox)
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::Controls,
                SemanticReference::Local(option_key.clone()),
            ))
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::ActiveDescendant,
                SemanticReference::Local(option_key.clone()),
            ))
            .with_child(SemanticNodeContribution::new(
                option_key,
                SemanticRole::Option,
            ));
        assert!(
            SemanticContribution::single(combobox)
                .validate(context)
                .is_ok()
        );

        let missing_controls_key = SemanticKey::from_static("missing-controls-option")
            .unwrap_or_else(|_| unreachable!("static test key is valid"));
        let missing_controls = SemanticNodeContribution::primary(SemanticRole::ComboBox)
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::ActiveDescendant,
                SemanticReference::Local(missing_controls_key.clone()),
            ))
            .with_child(SemanticNodeContribution::new(
                missing_controls_key,
                SemanticRole::Option,
            ));
        assert_eq!(
            SemanticContribution::single(missing_controls).validate(context),
            Err(
                SemanticContributionError::ActiveDescendantRequiresControls {
                    key: SemanticKey::PRIMARY,
                }
            )
        );

        let dialog_option = SemanticKey::from_static("dialog-option")
            .unwrap_or_else(|_| unreachable!("static test key is valid"));
        let dialog_popup = SemanticNodeContribution::primary(SemanticRole::ComboBox)
            .with_popup(SemanticPopupKind::Dialog)
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::Controls,
                SemanticReference::Local(dialog_option.clone()),
            ))
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::ActiveDescendant,
                SemanticReference::Local(dialog_option.clone()),
            ))
            .with_child(SemanticNodeContribution::new(
                dialog_option,
                SemanticRole::Option,
            ));
        assert_eq!(
            SemanticContribution::single(dialog_popup).validate(context),
            Err(
                SemanticContributionError::ActiveDescendantNotSupportedForDialogPopup {
                    key: SemanticKey::PRIMARY,
                }
            )
        );
    }

    #[test]
    fn editable_composite_roles_reject_secret_and_multiline_incompatibilities() {
        let source = "secret";
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(267), TextDocumentRevision::new(1));
        let position = TextPosition::new(snapshot, source, 0, TextAffinity::Downstream)
            .unwrap_or_else(|_| unreachable!("controlled position is valid"));
        let editable = SemanticEditable::new(
            snapshot,
            source,
            TextSelection::collapsed(position),
            TextSensitivity::Secret,
            false,
        )
        .unwrap_or_else(|| unreachable!("controlled secret editable is structurally valid"));

        let ordinary = SemanticNodeContribution::primary(SemanticRole::EditableText)
            .with_editable(editable.clone());
        assert!(
            SemanticContribution::single(ordinary)
                .validate(SemanticContributionContext::default())
                .is_ok()
        );

        let multiline_secret = SemanticNodeContribution::primary(SemanticRole::EditableText)
            .with_editable(editable.clone())
            .with_editable_mode(SemanticEditableMode::Multiline);
        assert_eq!(
            SemanticContribution::single(multiline_secret)
                .validate(SemanticContributionContext::default()),
            Err(SemanticContributionError::EditableCombinationNotSupported {
                key: SemanticKey::PRIMARY,
                role: SemanticRole::EditableText,
            })
        );

        let secret_combobox = SemanticNodeContribution::primary(SemanticRole::ComboBox)
            .with_editable(editable.clone());
        assert_eq!(
            SemanticContribution::single(secret_combobox)
                .validate(SemanticContributionContext::default()),
            Err(SemanticContributionError::EditableCombinationNotSupported {
                key: SemanticKey::PRIMARY,
                role: SemanticRole::ComboBox,
            })
        );

        let empty_range = SemanticRange::new(None, None, None)
            .unwrap_or_else(|_| unreachable!("open spin range is valid"));
        let secret_spin = SemanticNodeContribution::primary(SemanticRole::SpinButton)
            .with_range(empty_range)
            .with_editable(editable);
        assert_eq!(
            SemanticContribution::single(secret_spin)
                .validate(SemanticContributionContext::default()),
            Err(SemanticContributionError::EditableCombinationNotSupported {
                key: SemanticKey::PRIMARY,
                role: SemanticRole::SpinButton,
            })
        );
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
