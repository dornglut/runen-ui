//! Host-neutral keyboard, committed-text, and composition protocol values.

use core::{
    fmt,
    hash::{Hash, Hasher},
};

use crate::runtime_protocol::RuntimeNamespace;
use crate::{InputDeviceId, KeyModifiers};

/// Layout-independent physical keyboard identity.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PhysicalKey {
    Enter,
    Space,
    Tab,
    Escape,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    /// An owned host-neutral physical code.
    Code(String),
}

/// Interpreted keyboard meaning. Character values are never committed text.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum LogicalKey {
    Enter,
    Space,
    Tab,
    Escape,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Character(String),
    Named(String),
}

/// Keyboard transition reported by a host.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KeyboardPhase {
    Down,
    Up,
    Cancel,
}

/// Physical location of a keyboard key.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KeyLocation {
    Standard,
    Left,
    Right,
    Numpad,
}

/// Whether the host associates the keyboard event with composition.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KeyboardCompositionState {
    Inactive,
    Active,
}

/// One host-neutral raw keyboard event.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct KeyboardEvent {
    phase: KeyboardPhase,
    physical_key: PhysicalKey,
    logical_key: LogicalKey,
    modifiers: KeyModifiers,
    repeat: bool,
    location: KeyLocation,
    composition: KeyboardCompositionState,
    device_id: Option<InputDeviceId>,
}

impl KeyboardEvent {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        phase: KeyboardPhase,
        physical_key: PhysicalKey,
        logical_key: LogicalKey,
        modifiers: KeyModifiers,
        repeat: bool,
        location: KeyLocation,
        composition: KeyboardCompositionState,
        device_id: Option<InputDeviceId>,
    ) -> Self {
        Self {
            phase,
            physical_key,
            logical_key,
            modifiers,
            repeat,
            location,
            composition,
            device_id,
        }
    }
    #[must_use]
    pub const fn phase(&self) -> KeyboardPhase {
        self.phase
    }
    #[must_use]
    pub const fn physical_key(&self) -> &PhysicalKey {
        &self.physical_key
    }
    #[must_use]
    pub const fn logical_key(&self) -> &LogicalKey {
        &self.logical_key
    }
    #[must_use]
    pub const fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }
    #[must_use]
    pub const fn is_repeat(&self) -> bool {
        self.repeat
    }
    #[must_use]
    pub const fn location(&self) -> KeyLocation {
        self.location
    }
    #[must_use]
    pub const fn composition_state(&self) -> KeyboardCompositionState {
        self.composition
    }
    #[must_use]
    pub const fn device_id(&self) -> Option<InputDeviceId> {
        self.device_id
    }
}

/// Error returned when committed text is empty.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommittedTextError;
impl fmt::Display for CommittedTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("committed text must not be empty")
    }
}
impl std::error::Error for CommittedTextError {}

/// Unicode text committed by the host, distinct from keyboard key meaning.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CommittedTextEvent {
    text: String,
    device_id: Option<InputDeviceId>,
}
impl CommittedTextEvent {
    /// Creates committed text.
    ///
    /// # Errors
    ///
    /// Returns [`CommittedTextError`] when the text is empty.
    pub fn new(
        text: impl Into<String>,
        device_id: Option<InputDeviceId>,
    ) -> Result<Self, CommittedTextError> {
        let text = text.into();
        if text.is_empty() {
            Err(CommittedTextError)
        } else {
            Ok(Self { text, device_id })
        }
    }
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
    #[must_use]
    pub const fn device_id(&self) -> Option<InputDeviceId> {
        self.device_id
    }
}

/// Opaque, runtime-local identity for one non-reusable composition lifetime.
#[derive(Clone)]
pub struct CompositionGeneration {
    namespace: RuntimeNamespace,
    generation: u64,
}
impl CompositionGeneration {
    #[must_use]
    pub const fn get(&self) -> u64 {
        self.generation
    }
}
impl PartialEq for CompositionGeneration {
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation && self.namespace.__runtime_same_as(&other.namespace)
    }
}
impl Eq for CompositionGeneration {}
impl Hash for CompositionGeneration {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.namespace.__runtime_hash(state);
        self.generation.hash(state);
    }
}
impl fmt::Debug for CompositionGeneration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CompositionGeneration(..)")
    }
}

impl RuntimeNamespace {
    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_composition_generation(&self, generation: u64) -> CompositionGeneration {
        CompositionGeneration {
            namespace: self.clone(),
            generation,
        }
    }
    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_composition_generation_is_local(
        &self,
        generation: &CompositionGeneration,
    ) -> bool {
        self.__runtime_same_as(&generation.namespace)
    }
}

/// Checked UTF-8 byte range into one composition preedit string.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CompositionRange {
    start: usize,
    end: usize,
}
/// Structured invalid composition-range input.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CompositionRangeError {
    Reversed,
    OutOfBounds,
    NotScalarBoundary,
}
impl fmt::Display for CompositionRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid UTF-8 composition range")
    }
}
impl std::error::Error for CompositionRangeError {}
impl CompositionRange {
    /// Creates a checked range into `preedit`.
    ///
    /// # Errors
    ///
    /// Returns [`CompositionRangeError`] when the range is reversed, out of
    /// bounds, or does not align to Unicode scalar boundaries.
    pub const fn new(
        preedit: &str,
        start: usize,
        end: usize,
    ) -> Result<Self, CompositionRangeError> {
        if start > end {
            return Err(CompositionRangeError::Reversed);
        }
        if end > preedit.len() {
            return Err(CompositionRangeError::OutOfBounds);
        }
        if !preedit.is_char_boundary(start) || !preedit.is_char_boundary(end) {
            return Err(CompositionRangeError::NotScalarBoundary);
        }
        Ok(Self { start, end })
    }
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }
}

/// Why a composition lifetime was cancelled.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CompositionCancelReason {
    FocusTransfer,
    Removal,
    Replacement,
    Disablement,
    Explicit,
    Shutdown,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CompositionStart {
    generation: CompositionGeneration,
    device_id: Option<InputDeviceId>,
}
impl CompositionStart {
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(
        generation: CompositionGeneration,
        device_id: Option<InputDeviceId>,
    ) -> Self {
        Self {
            generation,
            device_id,
        }
    }
    #[must_use]
    pub const fn generation(&self) -> &CompositionGeneration {
        &self.generation
    }
    #[must_use]
    pub const fn device_id(&self) -> Option<InputDeviceId> {
        self.device_id
    }
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CompositionUpdate {
    generation: CompositionGeneration,
    preedit: String,
    range: Option<CompositionRange>,
}
impl CompositionUpdate {
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(
        generation: CompositionGeneration,
        preedit: String,
        range: Option<CompositionRange>,
    ) -> Self {
        Self {
            generation,
            preedit,
            range,
        }
    }
    #[must_use]
    pub const fn generation(&self) -> &CompositionGeneration {
        &self.generation
    }
    #[must_use]
    pub fn preedit(&self) -> &str {
        &self.preedit
    }
    #[must_use]
    pub const fn range(&self) -> Option<CompositionRange> {
        self.range
    }
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CompositionEnd {
    generation: CompositionGeneration,
}
impl CompositionEnd {
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(generation: CompositionGeneration) -> Self {
        Self { generation }
    }
    #[must_use]
    pub const fn generation(&self) -> &CompositionGeneration {
        &self.generation
    }
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CompositionCancel {
    generation: CompositionGeneration,
    reason: CompositionCancelReason,
}
impl CompositionCancel {
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(
        generation: CompositionGeneration,
        reason: CompositionCancelReason,
    ) -> Self {
        Self { generation, reason }
    }
    #[must_use]
    pub const fn generation(&self) -> &CompositionGeneration {
        &self.generation
    }
    #[must_use]
    pub const fn reason(&self) -> CompositionCancelReason {
        self.reason
    }
}

/// One routed composition lifecycle event.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CompositionEvent {
    Start(CompositionStart),
    Update(CompositionUpdate),
    End(CompositionEnd),
    Cancel(CompositionCancel),
}
impl CompositionEvent {
    #[must_use]
    pub const fn generation(&self) -> &CompositionGeneration {
        match self {
            Self::Start(v) => v.generation(),
            Self::Update(v) => v.generation(),
            Self::End(v) => v.generation(),
            Self::Cancel(v) => v.generation(),
        }
    }
}
