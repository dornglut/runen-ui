//! Text-editing report evidence types.

use crate::WidgetId;

#[derive(Debug, Clone, PartialEq)]
pub struct TextEditingReport {
    pub replay_id: String,
    pub fixture_id: String,
    pub input_steps: Vec<String>,
    pub descriptor_evidence: Vec<TextEditingDescriptorEvidence>,
    pub lifecycle_transitions: Vec<TextEditingLifecycleTransition>,
    pub caret_evidence: Vec<TextEditingCaretEvidence>,
    pub selection_evidence: Vec<TextEditingSelectionEvidence>,
    pub composition_evidence: Vec<TextEditingCompositionEvidence>,
    pub value_evidence: Vec<TextEditingValueEvidence>,
    pub accepted_edit_intents: Vec<TextEditingEditIntentEvidence>,
    pub suppressed_edit_intents: Vec<TextEditingSuppressionEvidence>,
    pub boundary_assertions: TextEditingBoundaryAssertions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditingDescriptorEvidence {
    pub target_id: String,
    pub widget_id: WidgetId,
    pub control_kind_id: String,
    pub mode: String,
    pub supported_intents: Vec<String>,
    pub selection_policy: String,
    pub composition_policy: String,
    pub host_owned_mutation: bool,
    pub proof_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditingLifecycleTransition {
    pub step_id: String,
    pub target_id: String,
    pub from: TextEditingLifecycleState,
    pub to: TextEditingLifecycleState,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEditingLifecycleState {
    Unfocused,
    Focused,
    Editing,
    Composing,
    Selecting,
    Submitting,
    Cancelled,
    Suppressed,
}

impl TextEditingLifecycleState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unfocused => "unfocused",
            Self::Focused => "focused",
            Self::Editing => "editing",
            Self::Composing => "composing",
            Self::Selecting => "selecting",
            Self::Submitting => "submitting",
            Self::Cancelled => "cancelled",
            Self::Suppressed => "suppressed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditingCaretEvidence {
    pub step_id: String,
    pub target_id: String,
    pub position: String,
    pub reason: String,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditingSelectionEvidence {
    pub step_id: String,
    pub target_id: String,
    pub anchor: String,
    pub extent: String,
    pub reason: String,
    pub collapsed: bool,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditingCompositionEvidence {
    pub step_id: String,
    pub target_id: String,
    pub kind: String,
    pub text: String,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditingValueEvidence {
    pub step_id: String,
    pub target_id: String,
    pub committed_text: String,
    pub composition_text: Option<String>,
    pub rendered_value: String,
    pub caret: String,
    pub selection: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditingEditIntentEvidence {
    pub step_id: String,
    pub input_sample_id: String,
    pub target_id: String,
    pub intent: String,
    pub text: String,
    pub host_owned_source: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditingSuppressionEvidence {
    pub step_id: String,
    pub input_sample_id: String,
    pub target_id: Option<String>,
    pub intent: String,
    pub reason: String,
    pub host_commands_executed: u32,
    pub product_mutations: u32,
    pub authored_ui_edits: u32,
    pub product_undo_redo_operations: u32,
    pub plugin_framework_operations: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextEditingBoundaryAssertions {
    pub host_commands_executed: u32,
    pub product_mutations: u32,
    pub authored_ui_edits: u32,
    pub product_undo_redo_operations: u32,
    pub plugin_framework_operations: u32,
    pub accepted_edit_intents: u32,
    pub suppressed_edit_intents: u32,
    pub lifecycle_transitions: u32,
    pub caret_moves: u32,
    pub selection_changes: u32,
    pub composition_events: u32,
}

impl TextEditingBoundaryAssertions {
    pub const fn no_bypass_evidence(self) -> bool {
        self.host_commands_executed == 0
            && self.product_mutations == 0
            && self.authored_ui_edits == 0
            && self.product_undo_redo_operations == 0
            && self.plugin_framework_operations == 0
    }
}
