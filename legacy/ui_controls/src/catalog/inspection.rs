use serde::{Deserialize, Serialize};

use super::{
    ControlCatalogEntryDescriptor, ControlDiagnosticBadge, ControlStoryProofBadge,
    diagnostic_badges,
};
use crate::accessibility::ControlAccessibilityCapabilitySummary;
use crate::editable_text::ControlEditableTextSupportSummary;
use crate::generic_text::ControlGenericTextSupportSummary;
use crate::input::ControlInputCapabilitySummary;
use crate::interaction::ControlInteractionSupportSummary;
use crate::overlay::ControlOverlaySupportSummary;
use crate::package::descriptor::{ControlKindDescriptor, ControlPackageDescriptor};
use crate::package::story_proof::ControlStoryProofSummary;
use crate::state::ControlStateCapabilitySummary;
use crate::theme::ControlThemeCapabilitySummary;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlInspectionDescriptor {
    pub package_id: String,
    pub control_kind_id: String,
    pub display_name: String,
    pub facts: Vec<ControlInspectionFact>,
    pub diagnostic_badges: Vec<ControlDiagnosticBadge>,
    #[serde(default)]
    pub story_proof_badge: Option<ControlStoryProofBadge>,
}
impl ControlInspectionDescriptor {
    pub fn from_control_kind(
        package: &ControlPackageDescriptor,
        kind: &ControlKindDescriptor,
    ) -> Self {
        let entry = ControlCatalogEntryDescriptor::from_control_kind(package, kind);
        let mut facts = Vec::new();
        push_fact(
            &mut facts,
            ControlInspectionSection::Identity,
            "package_id",
            &entry.package_id,
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Identity,
            "control_kind_id",
            &entry.control_kind_id,
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Identity,
            "display_name",
            &entry.display_name,
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Metadata,
            "category",
            &entry.category,
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Metadata,
            "tags",
            &entry.tags.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Compatibility,
            "supports_runtime_mount",
            bool_string(entry.compatibility.supports_runtime_mount),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Schemas,
            "properties",
            &schema_ref(&kind.property_schema),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Schemas,
            "state",
            &schema_ref(&kind.state_schema),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Schemas,
            "event_payload",
            &schema_ref(&kind.event_payload_schema),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Kernels,
            "layout",
            kind.kernels.layout.as_str(),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Kernels,
            "interaction",
            kind.kernels.interaction.as_str(),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Kernels,
            "visual",
            kind.kernels.visual.as_str(),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Routes,
            "routes",
            &entry.route_ids.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Fixtures,
            "fixtures",
            &entry.fixture_ids.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Stories,
            "stories",
            &entry.story_ids.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Diagnostics,
            "diagnostics",
            &entry.diagnostic_ids.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::MountEligibility,
            "eligible",
            bool_string(entry.mount_eligible),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Layering,
            "overlay.kinds",
            &entry.overlay_kinds.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Layering,
            "overlay.triggers",
            &entry.overlay_triggers.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Layering,
            "overlay.layers",
            &entry.overlay_layers.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Layering,
            "overlay.supported",
            bool_string(entry.overlay_supported),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextEditing,
            "text_editing.supported",
            bool_string(entry.editable_text_supported),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.supported",
            bool_string(entry.generic_text_supported),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.roles",
            &entry.text_roles.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.semantic_roles",
            &entry.text_semantic_roles.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.wrap",
            &entry.text_wrap_policies.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.overflow",
            &entry.text_overflow_policies.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.alignment",
            &entry.text_alignment_policies.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.inline_spans_supported",
            bool_string(entry.inline_spans_supported),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.line_metrics_supported",
            bool_string(entry.line_metrics_supported),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.glyph_evidence_supported",
            bool_string(entry.glyph_evidence_supported),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.fallback_evidence_supported",
            bool_string(entry.fallback_evidence_supported),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.renderer_backend_required",
            bool_string(entry.renderer_backend_required),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.executes_host_commands",
            bool_string(entry.executes_host_commands),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.mutates_product_state",
            bool_string(entry.mutates_product_state),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.authored_ui_edits",
            "false",
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::TextDisplay,
            "text_display.product_undo_redo",
            "false",
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Surface2D,
            "surface2d.supported",
            bool_string(entry.surface2d_supported),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Surface2D,
            "surface2d.input_modes",
            &entry.surface2d_input_modes.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Surface2D,
            "surface2d.layers",
            &entry.surface2d_layers.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Surface2D,
            "surface2d.budget_evidence",
            &entry.surface2d_budget_evidence.join(","),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Surface2D,
            "surface2d.accessibility_complete",
            bool_string(entry.surface2d_accessibility_complete),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Surface2D,
            "surface2d.interaction_complete",
            bool_string(entry.surface2d_interaction_complete),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Surface2D,
            "surface2d.renderer_backend_required",
            bool_string(entry.renderer_backend_required),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Surface2D,
            "surface2d.executes_host_commands",
            bool_string(entry.executes_host_commands),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Surface2D,
            "surface2d.mutates_product_state",
            bool_string(entry.mutates_product_state),
        );
        push_fact(
            &mut facts,
            ControlInspectionSection::Surface2D,
            "surface2d.graph_or_timeline_semantics",
            bool_string(entry.surface2d_graph_or_timeline_semantics),
        );
        Self {
            package_id: entry.package_id,
            control_kind_id: entry.control_kind_id,
            display_name: entry.display_name,
            facts,
            diagnostic_badges: diagnostic_badges(package, kind),
            story_proof_badge: None,
        }
    }
    pub fn with_story_proof_summary(mut self, summary: &ControlStoryProofSummary) -> Self {
        self.story_proof_badge = Some(ControlStoryProofBadge::from_summary(summary));
        self
    }
    pub fn with_input_summary(mut self, summary: &ControlInputCapabilitySummary) -> Self {
        for fact in summary.inspection_facts() {
            push_fact(
                &mut self.facts,
                ControlInspectionSection::Input,
                &fact.key,
                &fact.value,
            );
        }
        self
    }
    pub fn with_interaction_summary(mut self, summary: &ControlInteractionSupportSummary) -> Self {
        for fact in summary.inspection_facts() {
            push_fact(
                &mut self.facts,
                ControlInspectionSection::Interaction,
                &fact.key,
                &fact.value,
            );
        }
        self
    }
    pub fn with_layering_summary(mut self, summary: &ControlOverlaySupportSummary) -> Self {
        for fact in summary.inspection_facts() {
            push_fact(
                &mut self.facts,
                ControlInspectionSection::Layering,
                &fact.key,
                &fact.value,
            );
        }
        self
    }
    pub fn with_editable_text_summary(
        mut self,
        summary: &ControlEditableTextSupportSummary,
    ) -> Self {
        for fact in summary.inspection_facts() {
            push_fact(
                &mut self.facts,
                ControlInspectionSection::TextEditing,
                &fact.key,
                &fact.value,
            );
        }
        self
    }
    pub fn with_generic_text_summary(mut self, summary: &ControlGenericTextSupportSummary) -> Self {
        for fact in summary.inspection_facts() {
            push_fact(
                &mut self.facts,
                ControlInspectionSection::TextDisplay,
                &fact.key,
                &fact.value,
            );
        }
        self
    }
    pub fn with_state_summary(mut self, summary: &ControlStateCapabilitySummary) -> Self {
        for fact in summary.inspection_facts() {
            push_fact(
                &mut self.facts,
                ControlInspectionSection::State,
                &fact.key,
                &fact.value,
            );
        }
        self
    }
    pub fn with_theme_summary(mut self, summary: &ControlThemeCapabilitySummary) -> Self {
        for fact in summary.inspection_facts() {
            push_fact(
                &mut self.facts,
                ControlInspectionSection::Theme,
                &fact.key,
                &fact.value,
            );
        }
        self
    }
    pub fn with_accessibility_summary(
        mut self,
        summary: &ControlAccessibilityCapabilitySummary,
    ) -> Self {
        for fact in summary.inspection_facts() {
            push_fact(
                &mut self.facts,
                ControlInspectionSection::Accessibility,
                &fact.key,
                &fact.value,
            );
        }
        self
    }
    pub fn fact(&self, section: ControlInspectionSection, key: &str) -> Option<&str> {
        self.facts
            .iter()
            .find(|fact| fact.section == section && fact.key == key)
            .map(|fact| fact.value.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlInspectionSection {
    Identity,
    Metadata,
    Compatibility,
    Schemas,
    Kernels,
    Routes,
    Fixtures,
    Stories,
    StoryProof,
    Input,
    Interaction,
    Layering,
    TextEditing,
    TextDisplay,
    Surface2D,
    State,
    Theme,
    Accessibility,
    Diagnostics,
    MountEligibility,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlInspectionFact {
    pub section: ControlInspectionSection,
    pub key: String,
    pub value: String,
}
fn push_fact(
    facts: &mut Vec<ControlInspectionFact>,
    section: ControlInspectionSection,
    key: &str,
    value: &str,
) {
    facts.push(ControlInspectionFact {
        section,
        key: key.to_owned(),
        value: value.to_owned(),
    });
}
fn bool_string(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
fn schema_ref(schema: &ui_schema::UiSchemaRef) -> String {
    format!("{}@{}", schema.id.as_str(), schema.version.value())
}
