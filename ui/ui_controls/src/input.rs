//! File: domain/ui/ui_controls/src/input.rs
//! Crate: ui_controls

use serde::{Deserialize, Serialize};

use crate::package::ids::ControlKindId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlInputMode {
    Pointer,
    Wheel,
    Keyboard,
    SemanticAction,
    TextInput,
    TouchReady,
    Controller,
    StylusTablet,
}

impl ControlInputMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pointer => "pointer",
            Self::Wheel => "wheel",
            Self::Keyboard => "keyboard",
            Self::SemanticAction => "semantic-action",
            Self::TextInput => "text-input",
            Self::TouchReady => "touch-ready",
            Self::Controller => "controller",
            Self::StylusTablet => "stylus-tablet",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlInputModeSet {
    #[serde(default)]
    pub modes: Vec<ControlInputMode>,
}

impl ControlInputModeSet {
    pub fn new(modes: impl IntoIterator<Item = ControlInputMode>) -> Self {
        let mut modes = modes.into_iter().collect::<Vec<_>>();
        modes.sort();
        modes.dedup();
        Self { modes }
    }

    pub fn contains(&self, mode: ControlInputMode) -> bool {
        self.modes.contains(&mode)
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.modes.iter().map(|mode| mode.as_str()).collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlGestureKind {
    Hover,
    Press,
    Drag,
    MarqueeSelect,
    MultiClick,
    Cancel,
    Commit,
    Rollback,
    PointerCapture,
    LostCapture,
}

impl ControlGestureKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hover => "hover",
            Self::Press => "press",
            Self::Drag => "drag",
            Self::MarqueeSelect => "marquee-select",
            Self::MultiClick => "multi-click",
            Self::Cancel => "cancel",
            Self::Commit => "commit",
            Self::Rollback => "rollback",
            Self::PointerCapture => "pointer-capture",
            Self::LostCapture => "lost-capture",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlGestureRequirement {
    pub kind: ControlGestureKind,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default)]
    pub notes: String,
}

impl ControlGestureRequirement {
    pub fn new(kind: ControlGestureKind) -> Self {
        Self {
            kind,
            required: true,
            notes: String::new(),
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlDeviceKind {
    Pressure,
    Tilt,
    Twist,
    TangentialPressure,
    Eraser,
    BarrelButton,
    CoalescedSamples,
    PredictedSamples,
}

impl ControlDeviceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pressure => "pressure",
            Self::Tilt => "tilt",
            Self::Twist => "twist",
            Self::TangentialPressure => "tangential-pressure",
            Self::Eraser => "eraser",
            Self::BarrelButton => "barrel-button",
            Self::CoalescedSamples => "coalesced-samples",
            Self::PredictedSamples => "predicted-samples",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlDeviceRequirement {
    pub kind: ControlDeviceKind,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default)]
    pub notes: String,
}

impl ControlDeviceRequirement {
    pub fn new(kind: ControlDeviceKind) -> Self {
        Self {
            kind,
            required: true,
            notes: String::new(),
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPointerRequirement {
    #[serde(default)]
    pub requires_capture: bool,
    #[serde(default)]
    pub requires_lost_capture: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlKeyboardRequirement {
    #[serde(default)]
    pub requires_focus: bool,
    #[serde(default)]
    pub requires_shortcuts: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlWheelRequirement {
    #[serde(default)]
    pub requires_scroll_delta: bool,
    #[serde(default)]
    pub requires_zoom_delta: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlTextInputRequirement {
    #[serde(default)]
    pub requires_text_entry: bool,
    #[serde(default)]
    pub requires_composition: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlSemanticActionRequirement {
    pub action: String,
    #[serde(default = "default_required")]
    pub required: bool,
}

impl ControlSemanticActionRequirement {
    pub fn new(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            required: true,
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlInputDescriptor {
    pub control_kind_id: ControlKindId,
    pub modes: ControlInputModeSet,
    #[serde(default)]
    pub gestures: Vec<ControlGestureRequirement>,
    #[serde(default)]
    pub devices: Vec<ControlDeviceRequirement>,
    #[serde(default)]
    pub pointer: ControlPointerRequirement,
    #[serde(default)]
    pub keyboard: ControlKeyboardRequirement,
    #[serde(default)]
    pub wheel: ControlWheelRequirement,
    #[serde(default)]
    pub text_input: ControlTextInputRequirement,
    #[serde(default)]
    pub semantic_actions: Vec<ControlSemanticActionRequirement>,
}

impl ControlInputDescriptor {
    pub fn new(control_kind_id: ControlKindId) -> Self {
        Self {
            control_kind_id,
            modes: ControlInputModeSet::default(),
            gestures: Vec::new(),
            devices: Vec::new(),
            pointer: ControlPointerRequirement::default(),
            keyboard: ControlKeyboardRequirement::default(),
            wheel: ControlWheelRequirement::default(),
            text_input: ControlTextInputRequirement::default(),
            semantic_actions: Vec::new(),
        }
    }

    pub fn with_modes(mut self, modes: impl IntoIterator<Item = ControlInputMode>) -> Self {
        self.modes = ControlInputModeSet::new(modes);
        self
    }

    pub fn with_gesture(mut self, gesture: ControlGestureRequirement) -> Self {
        self.gestures.push(gesture);
        self.gestures.sort_by_key(|gesture| gesture.kind);
        self.gestures.dedup_by_key(|gesture| gesture.kind);
        self
    }

    pub fn with_device(mut self, device: ControlDeviceRequirement) -> Self {
        self.devices.push(device);
        self.devices.sort_by_key(|device| device.kind);
        self.devices.dedup_by_key(|device| device.kind);
        self
    }

    pub fn with_pointer(mut self, pointer: ControlPointerRequirement) -> Self {
        self.pointer = pointer;
        self
    }

    pub fn with_keyboard(mut self, keyboard: ControlKeyboardRequirement) -> Self {
        self.keyboard = keyboard;
        self
    }

    pub fn with_wheel(mut self, wheel: ControlWheelRequirement) -> Self {
        self.wheel = wheel;
        self
    }

    pub fn with_text_input(mut self, text_input: ControlTextInputRequirement) -> Self {
        self.text_input = text_input;
        self
    }

    pub fn with_semantic_action(mut self, action: ControlSemanticActionRequirement) -> Self {
        self.semantic_actions.push(action);
        self.semantic_actions
            .sort_by(|left, right| left.action.cmp(&right.action));
        self.semantic_actions
            .dedup_by(|left, right| left.action.as_str() == right.action.as_str());
        self
    }

    pub fn summary(&self) -> ControlInputCapabilitySummary {
        ControlInputCapabilitySummary::from_descriptor(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlInputCapabilitySummary {
    pub control_kind_id: ControlKindId,
    pub modes: Vec<String>,
    pub required_gestures: Vec<String>,
    pub optional_gestures: Vec<String>,
    pub required_device_facts: Vec<String>,
    pub optional_device_facts: Vec<String>,
    pub semantic_actions: Vec<String>,
    pub requires_pointer_capture: bool,
    pub requires_lost_capture: bool,
    pub requires_keyboard_focus: bool,
    pub requires_text_entry: bool,
    pub has_runtime_behavior: bool,
}

impl ControlInputCapabilitySummary {
    pub fn from_descriptor(descriptor: &ControlInputDescriptor) -> Self {
        let mut required_gestures = descriptor
            .gestures
            .iter()
            .filter(|gesture| gesture.required)
            .map(|gesture| gesture.kind.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut optional_gestures = descriptor
            .gestures
            .iter()
            .filter(|gesture| !gesture.required)
            .map(|gesture| gesture.kind.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut required_device_facts = descriptor
            .devices
            .iter()
            .filter(|device| device.required)
            .map(|device| device.kind.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut optional_device_facts = descriptor
            .devices
            .iter()
            .filter(|device| !device.required)
            .map(|device| device.kind.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut semantic_actions = descriptor
            .semantic_actions
            .iter()
            .map(|action| action.action.clone())
            .collect::<Vec<_>>();

        required_gestures.sort();
        optional_gestures.sort();
        required_device_facts.sort();
        optional_device_facts.sort();
        semantic_actions.sort();

        Self {
            control_kind_id: descriptor.control_kind_id.clone(),
            modes: descriptor
                .modes
                .modes
                .iter()
                .map(|mode| mode.as_str().to_owned())
                .collect(),
            required_gestures,
            optional_gestures,
            required_device_facts,
            optional_device_facts,
            semantic_actions,
            requires_pointer_capture: descriptor.pointer.requires_capture,
            requires_lost_capture: descriptor.pointer.requires_lost_capture,
            requires_keyboard_focus: descriptor.keyboard.requires_focus,
            requires_text_entry: descriptor.text_input.requires_text_entry,
            has_runtime_behavior: false,
        }
    }

    pub fn inspection_facts(&self) -> Vec<ControlInputInspectionFact> {
        vec![
            ControlInputInspectionFact::new("modes", self.modes.join(",")),
            ControlInputInspectionFact::new("required_gestures", self.required_gestures.join(",")),
            ControlInputInspectionFact::new("optional_gestures", self.optional_gestures.join(",")),
            ControlInputInspectionFact::new(
                "required_device_facts",
                self.required_device_facts.join(","),
            ),
            ControlInputInspectionFact::new(
                "optional_device_facts",
                self.optional_device_facts.join(","),
            ),
            ControlInputInspectionFact::new("semantic_actions", self.semantic_actions.join(",")),
            ControlInputInspectionFact::new(
                "requires_pointer_capture",
                bool_string(self.requires_pointer_capture),
            ),
            ControlInputInspectionFact::new(
                "requires_lost_capture",
                bool_string(self.requires_lost_capture),
            ),
            ControlInputInspectionFact::new(
                "requires_keyboard_focus",
                bool_string(self.requires_keyboard_focus),
            ),
            ControlInputInspectionFact::new(
                "requires_text_entry",
                bool_string(self.requires_text_entry),
            ),
            ControlInputInspectionFact::new(
                "has_runtime_behavior",
                bool_string(self.has_runtime_behavior),
            ),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlInputInspectionFact {
    pub key: String,
    pub value: String,
}

impl ControlInputInspectionFact {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

fn default_required() -> bool {
    true
}

fn bool_string(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
