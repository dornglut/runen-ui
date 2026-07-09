//! Normalized UI input facts.
//!
//! Facts are device/input data prepared for runtime resolution. They do not
//! decide control behavior and do not execute product commands.

use crate::{
    FocusChange, FocusDirection, FocusTargetId, Key, KeyState, Modifiers, PointerButton,
    PointerDelta, PointerEventKind, PointerPacket, PointerPosition, SemanticActionEvent,
    TextCompositionFact, TextEditFact, TextSelectionFact,
};

/// Normalized input fact delivered to runtime interaction formation.
///
/// Facts carry device, keyboard, focus, semantic, and text-intent data without
/// executing reusable control behavior or product commands. `ui_runtime`
/// resolves these facts against mounted structure and descriptor declarations.
#[derive(Debug, Clone, PartialEq)]
pub enum NormalizedInputFact {
    /// Pointer input such as hover, press, release, or scroll.
    Pointer(PointerInputFact),

    /// Keyboard input with stable key/state data.
    Keyboard(KeyboardInputFact),

    /// Focus input such as explicit target, clear, or traversal.
    Focus(FocusInputFact),

    /// Semantic input already resolved by an input source.
    Semantic(SemanticInputFact),

    /// Text intent input observed before full text editing exists.
    TextIntent(TextIntentFact),

    /// Editable-text input intent fact. Runtime resolves this against package descriptors.
    TextEdit(TextEditFact),

    /// IME/preedit composition fact. Runtime keeps it separate from committed text.
    TextComposition(TextCompositionFact),

    /// Text caret or range selection fact.
    TextSelection(TextSelectionFact),
}

impl NormalizedInputFact {
    /// Returns the stable fact family for this normalized fact.
    pub const fn kind(&self) -> NormalizedInputFactKind {
        match self {
            Self::Pointer(_) => NormalizedInputFactKind::Pointer,
            Self::Keyboard(_) => NormalizedInputFactKind::Keyboard,
            Self::Focus(_) => NormalizedInputFactKind::Focus,
            Self::Semantic(_) => NormalizedInputFactKind::Semantic,
            Self::TextIntent(_) => NormalizedInputFactKind::TextIntent,
            Self::TextEdit(_) => NormalizedInputFactKind::TextEdit,
            Self::TextComposition(_) => NormalizedInputFactKind::TextComposition,
            Self::TextSelection(_) => NormalizedInputFactKind::TextSelection,
        }
    }
}

/// Stable discriminator for normalized input fact families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NormalizedInputFactKind {
    /// Pointer fact family.
    Pointer,

    /// Keyboard fact family.
    Keyboard,

    /// Focus fact family.
    Focus,

    /// Semantic fact family.
    Semantic,

    /// Text-intent fact family.
    TextIntent,

    /// Editable-text edit fact family.
    TextEdit,

    /// Editable-text composition fact family.
    TextComposition,

    /// Editable-text selection fact family.
    TextSelection,
}

impl NormalizedInputFactKind {
    /// Returns the stable label for this fact family.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pointer => "pointer",
            Self::Keyboard => "keyboard",
            Self::Focus => "focus",
            Self::Semantic => "semantic",
            Self::TextIntent => "text-intent",
            Self::TextEdit => "text-edit",
            Self::TextComposition => "text-composition",
            Self::TextSelection => "text-selection",
        }
    }
}

/// Pointer fact formed from mouse, touch, stylus, or equivalent pointer input.
///
/// This is normalized input data only. Pointer capture, activation, and
/// suppression semantics are formed later by `ui_runtime` against descriptors.
#[derive(Debug, Clone, PartialEq)]
pub struct PointerInputFact {
    /// Normalized pointer event kind.
    pub kind: PointerEventKind,

    /// Normalized pointer position.
    pub position: PointerPosition,

    /// Normalized pointer delta.
    pub delta: PointerDelta,

    /// Optional pointer button associated with the fact.
    pub button: Option<PointerButton>,

    /// Keyboard modifiers active for the input.
    pub modifiers: Modifiers,

    /// Normalized click count.
    pub click_count: u8,

    /// Raw packet data retained for lower-level inspection.
    pub packet: PointerPacket,
}

impl PointerInputFact {
    /// Creates a pointer fact with no button and no click count.
    pub fn new(kind: PointerEventKind, position: PointerPosition) -> Self {
        Self {
            kind,
            position,
            delta: PointerDelta::ZERO,
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            packet: PointerPacket::default(),
        }
    }

    /// Attaches a pointer button to the fact.
    pub fn with_button(mut self, button: PointerButton) -> Self {
        self.button = Some(button);
        self
    }

    /// Attaches a normalized click count to the fact.
    pub fn with_click_count(mut self, click_count: u8) -> Self {
        self.click_count = click_count;
        self
    }
}

/// Keyboard key fact with stable key, state, and modifier data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardInputFact {
    /// Stable key value.
    pub key: Key,

    /// Pressed/released key state.
    pub state: KeyState,

    /// Keyboard modifiers active for the input.
    pub modifiers: Modifiers,
}

impl KeyboardInputFact {
    /// Creates a keyboard fact with default modifiers.
    pub fn new(key: Key, state: KeyState) -> Self {
        Self {
            key,
            state,
            modifiers: Modifiers::default(),
        }
    }
}

/// Focus fact for explicit focus changes or traversal requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusInputFact {
    /// Explicit focus change request.
    pub change: FocusChange,

    /// Optional focus traversal direction.
    pub direction: Option<FocusDirection>,

    /// Whether resulting focus should be focus-visible.
    pub focus_visible: bool,
}

impl FocusInputFact {
    /// Creates a focus fact from an explicit focus change.
    pub const fn change(change: FocusChange) -> Self {
        Self {
            change,
            direction: None,
            focus_visible: false,
        }
    }

    /// Creates a focus traversal fact with focus-visible enabled.
    pub const fn traversal(direction: FocusDirection) -> Self {
        Self {
            change: FocusChange::None,
            direction: Some(direction),
            focus_visible: true,
        }
    }

    /// Creates an explicit focus-target fact with focus-visible enabled.
    pub const fn target(target: FocusTargetId) -> Self {
        Self {
            change: FocusChange::Set(target),
            direction: None,
            focus_visible: true,
        }
    }
}

/// Semantic action fact for renderer/input sources that already resolved intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticInputFact {
    /// Semantic action event supplied by the input source.
    pub event: SemanticActionEvent,
}

impl SemanticInputFact {
    /// Creates a semantic input fact.
    pub const fn new(event: SemanticActionEvent) -> Self {
        Self { event }
    }
}

/// Text-intent input fact used by Phase 12 as a read-only probe.
///
/// A text intent represents input at the runtime seam, not text editing. Phase
/// 12 replay may observe the fact and emit probe evidence, but it must not
/// mutate a text buffer, create selections, own caret layout, or start edit
/// transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextIntentFact {
    /// Minimal text-intent kind.
    pub kind: TextIntentKind,

    /// Text payload for insert-text intent.
    pub text: String,
}

impl TextIntentFact {
    /// Creates an insert-text intent fact.
    pub fn insert_text(text: impl Into<String>) -> Self {
        Self {
            kind: TextIntentKind::InsertText,
            text: text.into(),
        }
    }
}

/// Minimal text-intent kinds required before full text editing exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextIntentKind {
    /// Insert text intent; Phase 12 only observes this as a probe.
    InsertText,

    /// Commit intent placeholder before full text editing exists.
    Commit,

    /// Cancel intent placeholder before full text editing exists.
    Cancel,
}

/// Deterministic sample of normalized input facts for runtime replay.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedInputSample {
    /// Stable sample id used by replay scripts and reports.
    pub sample_id: String,

    /// Ordered normalized input facts in this sample.
    pub facts: Vec<NormalizedInputFact>,
}

impl NormalizedInputSample {
    /// Creates an empty input sample.
    pub fn new(sample_id: impl Into<String>) -> Self {
        Self {
            sample_id: sample_id.into(),
            facts: Vec::new(),
        }
    }

    /// Appends one normalized fact to the sample.
    pub fn with_fact(mut self, fact: NormalizedInputFact) -> Self {
        self.facts.push(fact);
        self
    }

    /// Returns stable labels for each fact family in the sample.
    pub fn fact_kinds(&self) -> Vec<&'static str> {
        self.facts.iter().map(|fact| fact.kind().as_str()).collect()
    }
}
