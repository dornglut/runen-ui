//! Reusable control interaction declarations.
//!
//! This module describes what interaction a control supports. It does not
//! execute interaction, collect OS input, mutate product state, open overlays,
//! or edit text. Runtime crates consume these declarations to form reusable
//! interaction evidence.

use serde::{Deserialize, Serialize};

use crate::package::ids::ControlKindId;

/// Canonical reusable interaction states that a control package may expose.
///
/// These states are descriptor vocabulary only. Runtime code may report that a
/// mounted control entered one of these states, but declaring the state never
/// grants host command execution, product mutation, overlay behavior, or text
/// editing authority to `ui_controls`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlInteractionState {
    /// Control is enabled.
    Enabled,

    /// Control is disabled and should suppress reusable interaction.
    Disabled,

    /// Control is read-only; text intent may be observed but not edited.
    ReadOnly,

    /// Pointer hover state.
    Hovered,

    /// Pointer press state.
    Pressed,

    /// Active/navigation state.
    Active,

    /// Focused state.
    Focused,

    /// Keyboard-visible focus state.
    FocusVisible,

    /// Pointer capture state.
    Captured,

    /// Suppressed interaction state.
    Suppressed,
}

impl ControlInteractionState {
    /// Returns the stable descriptor/catalog label for this state.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::ReadOnly => "read-only",
            Self::Hovered => "hovered",
            Self::Pressed => "pressed",
            Self::Active => "active",
            Self::Focused => "focused",
            Self::FocusVisible => "focus-visible",
            Self::Captured => "captured",
            Self::Suppressed => "suppressed",
        }
    }
}

/// A stable, deduplicated set of reusable interaction states for a control.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlInteractionStateSet {
    /// Sorted, deduplicated reusable interaction states.
    #[serde(default)]
    pub states: Vec<ControlInteractionState>,
}

impl ControlInteractionStateSet {
    /// Creates a sorted, deduplicated state set.
    pub fn new(states: impl IntoIterator<Item = ControlInteractionState>) -> Self {
        let mut states = states.into_iter().collect::<Vec<_>>();
        states.sort();
        states.dedup();
        Self { states }
    }

    /// Returns true when the set declares the provided state.
    pub fn contains(&self, state: ControlInteractionState) -> bool {
        self.states.contains(&state)
    }
}

/// Reusable input/runtime triggers that a control descriptor can require.
///
/// Pointer activation is split across press, release, activate, and cancel so
/// runtime replay can prove press visual state separately from semantic
/// activation. Button-like controls should emit activation on `PointerActivate`,
/// normally after a release inside a previously pressed target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlInteractionTrigger {
    /// Pointer hover or move over a control.
    PointerHover,

    /// Pointer down/press on a control.
    PointerPress,

    /// Pointer release after a prior press/capture.
    PointerRelease,

    /// Pointer activation, normally release-inside after a prior press.
    PointerActivate,

    /// Pointer cancel, release outside, or lost capture.
    PointerCancel,

    /// Focus resolution or traversal.
    Focus,

    /// Keyboard activation such as Enter or Space.
    KeyboardActivate,

    /// Keyboard navigation such as arrow-key movement.
    KeyboardNavigate,

    /// Semantic action fact resolved by a renderer/input source.
    SemanticAction,

    /// Text intent observed as a probe, without text editing.
    TextIntent,
}

impl ControlInteractionTrigger {
    /// Returns the stable descriptor/catalog label for this trigger.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PointerHover => "pointer-hover",
            Self::PointerPress => "pointer-press",
            Self::PointerRelease => "pointer-release",
            Self::PointerActivate => "pointer-activate",
            Self::PointerCancel => "pointer-cancel",
            Self::Focus => "focus",
            Self::KeyboardActivate => "keyboard-activate",
            Self::KeyboardNavigate => "keyboard-navigate",
            Self::SemanticAction => "semantic-action",
            Self::TextIntent => "text-intent",
        }
    }
}

/// Reusable semantic outcomes that hosts may consume after runtime formation.
///
/// Outcomes remain intents. They do not execute app, editor, game, overlay, or
/// text-editing behavior inside reusable control declarations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlInteractionOutcome {
    /// Runtime requests activation but does not execute a host command.
    ActivationRequested,

    /// Runtime emits action intent for a host/product layer to decide later.
    ActionIntent,

    /// Runtime emits list active-item intent.
    ActiveItemIntent,

    /// Runtime emits tree node intent.
    NodeIntent,

    /// Runtime emits table cell or row intent.
    CellOrRowIntent,

    /// Runtime observed text intent as a probe.
    TextIntentSeen,

    /// Runtime emits inspection intent only.
    InspectionRequested,

    /// Runtime emits open intent only.
    OpenRequested,
}

impl ControlInteractionOutcome {
    /// Returns the stable descriptor/catalog label for this outcome.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ActivationRequested => "activation-requested",
            Self::ActionIntent => "action-intent",
            Self::ActiveItemIntent => "active-item-intent",
            Self::NodeIntent => "node-intent",
            Self::CellOrRowIntent => "cell-or-row-intent",
            Self::TextIntentSeen => "text-intent-seen",
            Self::InspectionRequested => "inspection-requested",
            Self::OpenRequested => "open-requested",
        }
    }
}

/// A single trigger declaration and the reusable outcomes it may produce.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlInteractionRequirement {
    /// Trigger that runtime replay may match against normalized input.
    pub trigger: ControlInteractionTrigger,

    /// Reusable semantic outcomes allowed for this trigger.
    #[serde(default)]
    pub outcomes: Vec<ControlInteractionOutcome>,

    /// Whether runtime must have focus on the target before forming outcomes.
    #[serde(default)]
    pub requires_focus: bool,

    /// Whether a disabled target suppresses this trigger.
    #[serde(default = "default_suppresses_when_disabled")]
    pub suppresses_when_disabled: bool,
}

impl ControlInteractionRequirement {
    /// Creates a requirement with no outcomes and disabled suppression enabled.
    pub fn new(trigger: ControlInteractionTrigger) -> Self {
        Self {
            trigger,
            outcomes: Vec::new(),
            requires_focus: false,
            suppresses_when_disabled: true,
        }
    }

    /// Adds a reusable semantic outcome to this requirement.
    pub fn with_outcome(mut self, outcome: ControlInteractionOutcome) -> Self {
        self.outcomes.push(outcome);
        self.outcomes.sort();
        self.outcomes.dedup();
        self
    }

    /// Marks this requirement as focus-dependent.
    pub fn requiring_focus(mut self) -> Self {
        self.requires_focus = true;
        self
    }
}

/// Package-owned interaction declaration for one control kind.
///
/// This descriptor is the authoritative reusable interaction contract that the
/// catalog, inspection projection, and runtime replay fixture consume. Compiled
/// controls may keep a copy for convenience, but the package descriptor is the
/// durable public path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlInteractionDescriptor {
    /// Control kind this interaction declaration belongs to.
    pub control_kind_id: ControlKindId,

    /// Reusable states that may be observed or formed for this control.
    pub states: ControlInteractionStateSet,

    /// Trigger requirements that runtime replay may match against input facts.
    #[serde(default)]
    pub requirements: Vec<ControlInteractionRequirement>,

    /// Whether text-intent input may be observed as a probe.
    ///
    /// This does not imply editable text, caret ownership, selection,
    /// clipboard, undo/redo, IME, or text-buffer mutation.
    #[serde(default)]
    pub text_intent_probe: bool,
}

impl ControlInteractionDescriptor {
    /// Creates a descriptor with the default reusable state vocabulary.
    pub fn new(control_kind_id: ControlKindId) -> Self {
        Self {
            control_kind_id,
            states: ControlInteractionStateSet::new([
                ControlInteractionState::Enabled,
                ControlInteractionState::Disabled,
                ControlInteractionState::ReadOnly,
                ControlInteractionState::Hovered,
                ControlInteractionState::Pressed,
                ControlInteractionState::Active,
                ControlInteractionState::Focused,
                ControlInteractionState::FocusVisible,
                ControlInteractionState::Captured,
                ControlInteractionState::Suppressed,
            ]),
            requirements: Vec::new(),
            text_intent_probe: false,
        }
    }

    /// Replaces the descriptor state set.
    pub fn with_states(
        mut self,
        states: impl IntoIterator<Item = ControlInteractionState>,
    ) -> Self {
        self.states = ControlInteractionStateSet::new(states);
        self
    }

    /// Adds or replaces one trigger requirement.
    pub fn with_requirement(mut self, requirement: ControlInteractionRequirement) -> Self {
        self.requirements.push(requirement);
        self.requirements
            .sort_by_key(|requirement| requirement.trigger);
        self.requirements
            .dedup_by_key(|requirement| requirement.trigger);
        self
    }

    /// Marks whether runtime may observe text intent as a probe.
    pub fn with_text_intent_probe(mut self, text_intent_probe: bool) -> Self {
        self.text_intent_probe = text_intent_probe;
        self
    }

    /// Projects this descriptor into read-only catalog/inspection summary data.
    pub fn summary(&self) -> ControlInteractionSupportSummary {
        ControlInteractionSupportSummary::from_descriptor(self)
    }
}

/// Read-only catalog/inspection projection for a control interaction descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlInteractionSupportSummary {
    /// Control kind this support summary describes.
    pub control_kind_id: ControlKindId,

    /// Stable reusable state labels.
    pub states: Vec<String>,

    /// Stable reusable trigger labels.
    pub triggers: Vec<String>,

    /// Stable reusable outcome labels.
    pub outcomes: Vec<String>,

    /// Whether any declared requirement needs focus.
    pub requires_focus: bool,

    /// Whether text intent may be observed as a probe.
    pub text_intent_probe: bool,

    /// Whether reusable runtime interaction is supported by declarations.
    pub runtime_interaction_supported: bool,

    /// Whether this control owns runtime behavior itself.
    pub control_owned_runtime_behavior: bool,

    /// Whether this descriptor executes host commands.
    pub executes_host_commands: bool,

    /// Whether this descriptor mutates product state.
    pub mutates_product_state: bool,
}

impl ControlInteractionSupportSummary {
    /// Builds read-only support summary data from a package descriptor.
    pub fn from_descriptor(descriptor: &ControlInteractionDescriptor) -> Self {
        let mut triggers = descriptor
            .requirements
            .iter()
            .map(|requirement| requirement.trigger.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut outcomes = descriptor
            .requirements
            .iter()
            .flat_map(|requirement| requirement.outcomes.iter())
            .map(|outcome| outcome.as_str().to_owned())
            .collect::<Vec<_>>();

        triggers.sort();
        triggers.dedup();
        outcomes.sort();
        outcomes.dedup();

        Self {
            control_kind_id: descriptor.control_kind_id.clone(),
            states: descriptor
                .states
                .states
                .iter()
                .map(|state| state.as_str().to_owned())
                .collect(),
            triggers,
            outcomes,
            requires_focus: descriptor
                .requirements
                .iter()
                .any(|requirement| requirement.requires_focus),
            text_intent_probe: descriptor.text_intent_probe,
            runtime_interaction_supported: !descriptor.requirements.is_empty(),
            control_owned_runtime_behavior: false,
            executes_host_commands: false,
            mutates_product_state: false,
        }
    }

    /// Projects this summary into read-only inspection facts.
    pub fn inspection_facts(&self) -> Vec<ControlInteractionInspectionFact> {
        vec![
            ControlInteractionInspectionFact::new("states", self.states.join(",")),
            ControlInteractionInspectionFact::new("triggers", self.triggers.join(",")),
            ControlInteractionInspectionFact::new("outcomes", self.outcomes.join(",")),
            ControlInteractionInspectionFact::new(
                "requires_focus",
                bool_string(self.requires_focus),
            ),
            ControlInteractionInspectionFact::new(
                "text_intent_probe",
                bool_string(self.text_intent_probe),
            ),
            ControlInteractionInspectionFact::new(
                "runtime_interaction_supported",
                bool_string(self.runtime_interaction_supported),
            ),
            ControlInteractionInspectionFact::new(
                "control_owned_runtime_behavior",
                bool_string(self.control_owned_runtime_behavior),
            ),
            ControlInteractionInspectionFact::new(
                "executes_host_commands",
                bool_string(self.executes_host_commands),
            ),
            ControlInteractionInspectionFact::new(
                "mutates_product_state",
                bool_string(self.mutates_product_state),
            ),
        ]
    }
}

/// One read-only interaction fact projected into control inspection output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlInteractionInspectionFact {
    /// Stable inspection fact key.
    pub key: String,

    /// Stable inspection fact value.
    pub value: String,
}

impl ControlInteractionInspectionFact {
    /// Creates one read-only inspection fact.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

fn default_suppresses_when_disabled() -> bool {
    true
}

fn bool_string(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
