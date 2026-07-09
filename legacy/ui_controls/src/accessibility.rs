//! File: domain/ui/ui_controls/src/accessibility.rs
//! Crate: ui_controls

use serde::{Deserialize, Serialize};

use crate::package::ids::ControlKindId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlAccessibilityRole {
    Button,
    Label,
    Checkbox,
    Slider,
    Text,
    List,
    ListItem,
    Tree,
    TreeItem,
    Table,
    Row,
    Cell,
    Menu,
    MenuItem,
    Dialog,
    Panel,
    Canvas,
    Custom,
}

impl ControlAccessibilityRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Label => "label",
            Self::Checkbox => "checkbox",
            Self::Slider => "slider",
            Self::Text => "text",
            Self::List => "list",
            Self::ListItem => "list-item",
            Self::Tree => "tree",
            Self::TreeItem => "tree-item",
            Self::Table => "table",
            Self::Row => "row",
            Self::Cell => "cell",
            Self::Menu => "menu",
            Self::MenuItem => "menu-item",
            Self::Dialog => "dialog",
            Self::Panel => "panel",
            Self::Canvas => "canvas",
            Self::Custom => "custom",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlAccessibilityLabelRequirement {
    pub label_id: String,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default)]
    pub notes: String,
}

impl ControlAccessibilityLabelRequirement {
    pub fn new(label_id: impl Into<String>) -> Self {
        Self {
            label_id: label_id.into(),
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlAccessibilityDescriptionRequirement {
    pub description_id: String,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default)]
    pub notes: String,
}

impl ControlAccessibilityDescriptionRequirement {
    pub fn new(description_id: impl Into<String>) -> Self {
        Self {
            description_id: description_id.into(),
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlSemanticHint {
    pub hint_id: String,
    #[serde(default)]
    pub notes: String,
}

impl ControlSemanticHint {
    pub fn new(hint_id: impl Into<String>) -> Self {
        Self {
            hint_id: hint_id.into(),
            notes: String::new(),
        }
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlFocusRequirement {
    #[serde(default)]
    pub focusable: bool,
    #[serde(default)]
    pub focus_order: Option<u32>,
    #[serde(default)]
    pub traps_focus: bool,
    #[serde(default)]
    pub returns_focus: bool,
}

impl ControlFocusRequirement {
    pub fn focusable() -> Self {
        Self {
            focusable: true,
            focus_order: None,
            traps_focus: false,
            returns_focus: false,
        }
    }

    pub fn with_focus_order(mut self, focus_order: u32) -> Self {
        self.focus_order = Some(focus_order);
        self
    }

    pub fn with_focus_trap(mut self) -> Self {
        self.traps_focus = true;
        self
    }

    pub fn with_focus_return(mut self) -> Self {
        self.returns_focus = true;
        self
    }

    pub fn facts(&self) -> Vec<&'static str> {
        let mut facts = Vec::new();
        if self.focusable {
            facts.push("focusable");
        }
        if self.focus_order.is_some() {
            facts.push("focus-order");
        }
        if self.traps_focus {
            facts.push("focus-trap");
        }
        if self.returns_focus {
            facts.push("focus-return");
        }
        facts
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlKeyboardActivation {
    Activate,
    Cancel,
    Commit,
    Expand,
    Collapse,
    Increment,
    Decrement,
    NavigateNext,
    NavigatePrevious,
}

impl ControlKeyboardActivation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Activate => "activate",
            Self::Cancel => "cancel",
            Self::Commit => "commit",
            Self::Expand => "expand",
            Self::Collapse => "collapse",
            Self::Increment => "increment",
            Self::Decrement => "decrement",
            Self::NavigateNext => "navigate-next",
            Self::NavigatePrevious => "navigate-previous",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlSemanticState {
    Enabled,
    Disabled,
    Selected,
    Pressed,
    Expanded,
    Collapsed,
    Checked,
    Unchecked,
    Mixed,
    Busy,
    Invalid,
    Readonly,
    Required,
}

impl ControlSemanticState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Selected => "selected",
            Self::Pressed => "pressed",
            Self::Expanded => "expanded",
            Self::Collapsed => "collapsed",
            Self::Checked => "checked",
            Self::Unchecked => "unchecked",
            Self::Mixed => "mixed",
            Self::Busy => "busy",
            Self::Invalid => "invalid",
            Self::Readonly => "readonly",
            Self::Required => "required",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlValueRangeMetadata {
    pub value_id: String,
    #[serde(default)]
    pub has_minimum: bool,
    #[serde(default)]
    pub has_maximum: bool,
    #[serde(default)]
    pub has_step: bool,
}

impl ControlValueRangeMetadata {
    pub fn new(value_id: impl Into<String>) -> Self {
        Self {
            value_id: value_id.into(),
            has_minimum: false,
            has_maximum: false,
            has_step: false,
        }
    }

    pub fn with_minimum(mut self) -> Self {
        self.has_minimum = true;
        self
    }

    pub fn with_maximum(mut self) -> Self {
        self.has_maximum = true;
        self
    }

    pub fn with_step(mut self) -> Self {
        self.has_step = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlAccessibilityDiagnosticKind {
    MissingRole,
    MissingLabel,
    MissingDescription,
    MissingFocusOrder,
    ExpectedFailure,
}

impl ControlAccessibilityDiagnosticKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingRole => "missing-role",
            Self::MissingLabel => "missing-label",
            Self::MissingDescription => "missing-description",
            Self::MissingFocusOrder => "missing-focus-order",
            Self::ExpectedFailure => "expected-failure",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlAccessibilityDiagnostic {
    pub diagnostic_id: String,
    pub kind: ControlAccessibilityDiagnosticKind,
    pub message: String,
}

impl ControlAccessibilityDiagnostic {
    pub fn new(
        diagnostic_id: impl Into<String>,
        kind: ControlAccessibilityDiagnosticKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            diagnostic_id: diagnostic_id.into(),
            kind,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlAccessibilityDescriptor {
    pub control_kind_id: ControlKindId,
    #[serde(default)]
    pub roles: Vec<ControlAccessibilityRole>,
    #[serde(default)]
    pub label_requirements: Vec<ControlAccessibilityLabelRequirement>,
    #[serde(default)]
    pub description_requirements: Vec<ControlAccessibilityDescriptionRequirement>,
    #[serde(default)]
    pub semantic_hints: Vec<ControlSemanticHint>,
    #[serde(default)]
    pub focus_requirements: Vec<ControlFocusRequirement>,
    #[serde(default)]
    pub keyboard_activations: Vec<ControlKeyboardActivation>,
    #[serde(default)]
    pub semantic_states: Vec<ControlSemanticState>,
    #[serde(default)]
    pub value_ranges: Vec<ControlValueRangeMetadata>,
    #[serde(default)]
    pub diagnostics: Vec<ControlAccessibilityDiagnostic>,
}

impl ControlAccessibilityDescriptor {
    pub fn new(control_kind_id: ControlKindId) -> Self {
        Self {
            control_kind_id,
            roles: Vec::new(),
            label_requirements: Vec::new(),
            description_requirements: Vec::new(),
            semantic_hints: Vec::new(),
            focus_requirements: Vec::new(),
            keyboard_activations: Vec::new(),
            semantic_states: Vec::new(),
            value_ranges: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn with_role(mut self, role: ControlAccessibilityRole) -> Self {
        self.roles.push(role);
        self.roles.sort();
        self.roles.dedup();
        self
    }

    pub fn with_label(mut self, label: ControlAccessibilityLabelRequirement) -> Self {
        self.label_requirements.push(label);
        self.label_requirements
            .sort_by(|left, right| left.label_id.cmp(&right.label_id));
        self.label_requirements
            .dedup_by(|left, right| left.label_id == right.label_id);
        self
    }

    pub fn with_description(
        mut self,
        description: ControlAccessibilityDescriptionRequirement,
    ) -> Self {
        self.description_requirements.push(description);
        self.description_requirements
            .sort_by(|left, right| left.description_id.cmp(&right.description_id));
        self.description_requirements
            .dedup_by(|left, right| left.description_id == right.description_id);
        self
    }

    pub fn with_hint(mut self, hint: ControlSemanticHint) -> Self {
        self.semantic_hints.push(hint);
        self.semantic_hints
            .sort_by(|left, right| left.hint_id.cmp(&right.hint_id));
        self.semantic_hints
            .dedup_by(|left, right| left.hint_id == right.hint_id);
        self
    }

    pub fn with_focus(mut self, focus: ControlFocusRequirement) -> Self {
        self.focus_requirements.push(focus);
        self
    }

    pub fn with_keyboard_activation(mut self, activation: ControlKeyboardActivation) -> Self {
        self.keyboard_activations.push(activation);
        self.keyboard_activations.sort();
        self.keyboard_activations.dedup();
        self
    }

    pub fn with_semantic_state(mut self, state: ControlSemanticState) -> Self {
        self.semantic_states.push(state);
        self.semantic_states.sort();
        self.semantic_states.dedup();
        self
    }

    pub fn with_value_range(mut self, value_range: ControlValueRangeMetadata) -> Self {
        self.value_ranges.push(value_range);
        self.value_ranges
            .sort_by(|left, right| left.value_id.cmp(&right.value_id));
        self.value_ranges
            .dedup_by(|left, right| left.value_id == right.value_id);
        self
    }

    pub fn with_diagnostic(mut self, diagnostic: ControlAccessibilityDiagnostic) -> Self {
        self.diagnostics.push(diagnostic);
        self.diagnostics
            .sort_by(|left, right| left.diagnostic_id.cmp(&right.diagnostic_id));
        self.diagnostics
            .dedup_by(|left, right| left.diagnostic_id == right.diagnostic_id);
        self
    }

    pub fn summary(&self) -> ControlAccessibilityCapabilitySummary {
        ControlAccessibilityCapabilitySummary::from_descriptor(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlAccessibilityCapabilitySummary {
    pub control_kind_id: ControlKindId,
    pub roles: Vec<String>,
    pub label_requirements: Vec<String>,
    pub description_requirements: Vec<String>,
    pub semantic_hints: Vec<String>,
    pub focus_facts: Vec<String>,
    pub keyboard_activations: Vec<String>,
    pub semantic_states: Vec<String>,
    pub value_ranges: Vec<String>,
    pub diagnostics: Vec<String>,
    pub expected_failures: Vec<String>,
    pub has_runtime_focus_behavior: bool,
}

impl ControlAccessibilityCapabilitySummary {
    pub fn from_descriptor(descriptor: &ControlAccessibilityDescriptor) -> Self {
        let mut roles = descriptor
            .roles
            .iter()
            .map(|role| role.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut label_requirements = descriptor
            .label_requirements
            .iter()
            .map(|label| label.label_id.clone())
            .collect::<Vec<_>>();
        let mut description_requirements = descriptor
            .description_requirements
            .iter()
            .map(|description| description.description_id.clone())
            .collect::<Vec<_>>();
        let mut semantic_hints = descriptor
            .semantic_hints
            .iter()
            .map(|hint| hint.hint_id.clone())
            .collect::<Vec<_>>();
        let mut focus_facts = descriptor
            .focus_requirements
            .iter()
            .flat_map(|focus| focus.facts().into_iter().map(str::to_owned))
            .collect::<Vec<_>>();
        let mut keyboard_activations = descriptor
            .keyboard_activations
            .iter()
            .map(|activation| activation.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut semantic_states = descriptor
            .semantic_states
            .iter()
            .map(|state| state.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut value_ranges = descriptor
            .value_ranges
            .iter()
            .map(|value_range| value_range.value_id.clone())
            .collect::<Vec<_>>();
        let mut diagnostics = descriptor
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut expected_failures = descriptor
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.kind == ControlAccessibilityDiagnosticKind::ExpectedFailure
            })
            .map(|diagnostic| diagnostic.diagnostic_id.clone())
            .collect::<Vec<_>>();

        sort_dedup(&mut roles);
        sort_dedup(&mut label_requirements);
        sort_dedup(&mut description_requirements);
        sort_dedup(&mut semantic_hints);
        sort_dedup(&mut focus_facts);
        sort_dedup(&mut keyboard_activations);
        sort_dedup(&mut semantic_states);
        sort_dedup(&mut value_ranges);
        sort_dedup(&mut diagnostics);
        sort_dedup(&mut expected_failures);

        Self {
            control_kind_id: descriptor.control_kind_id.clone(),
            roles,
            label_requirements,
            description_requirements,
            semantic_hints,
            focus_facts,
            keyboard_activations,
            semantic_states,
            value_ranges,
            diagnostics,
            expected_failures,
            has_runtime_focus_behavior: false,
        }
    }

    pub fn inspection_facts(&self) -> Vec<ControlAccessibilityInspectionFact> {
        vec![
            ControlAccessibilityInspectionFact::new("roles", self.roles.join(",")),
            ControlAccessibilityInspectionFact::new(
                "label_requirements",
                self.label_requirements.join(","),
            ),
            ControlAccessibilityInspectionFact::new(
                "description_requirements",
                self.description_requirements.join(","),
            ),
            ControlAccessibilityInspectionFact::new(
                "semantic_hints",
                self.semantic_hints.join(","),
            ),
            ControlAccessibilityInspectionFact::new("focus_facts", self.focus_facts.join(",")),
            ControlAccessibilityInspectionFact::new(
                "keyboard_activations",
                self.keyboard_activations.join(","),
            ),
            ControlAccessibilityInspectionFact::new(
                "semantic_states",
                self.semantic_states.join(","),
            ),
            ControlAccessibilityInspectionFact::new("value_ranges", self.value_ranges.join(",")),
            ControlAccessibilityInspectionFact::new("diagnostics", self.diagnostics.join(",")),
            ControlAccessibilityInspectionFact::new(
                "expected_failures",
                self.expected_failures.join(","),
            ),
            ControlAccessibilityInspectionFact::new(
                "has_runtime_focus_behavior",
                bool_string(self.has_runtime_focus_behavior),
            ),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlAccessibilityInspectionFact {
    pub key: String,
    pub value: String,
}

impl ControlAccessibilityInspectionFact {
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

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn bool_string(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
