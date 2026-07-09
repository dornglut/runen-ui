//! Reusable generic text-display declarations for control packages.
//!
//! `ui_controls` may describe renderer-neutral text display needs for reusable
//! controls. It does not own renderer backends, font discovery, GPU atlas upload,
//! editing, clipboard, product buffers, authored UI mutation, or undo stacks.

use serde::{Deserialize, Serialize};

use crate::package::ids::ControlKindId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlGenericTextSemanticRole {
    Label,
    Heading,
    Body,
    Helper,
    Badge,
    TabLabel,
    MenuItem,
    InspectorLabel,
    InspectorValue,
}

impl ControlGenericTextSemanticRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Label => "label",
            Self::Heading => "heading",
            Self::Body => "body",
            Self::Helper => "helper",
            Self::Badge => "badge",
            Self::TabLabel => "tab-label",
            Self::MenuItem => "menu-item",
            Self::InspectorLabel => "inspector-label",
            Self::InspectorValue => "inspector-value",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlGenericTextWrapPolicy {
    NoWrap,
    Word,
    Character,
}

impl ControlGenericTextWrapPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoWrap => "no-wrap",
            Self::Word => "word",
            Self::Character => "character",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlGenericTextOverflowPolicy {
    Clip,
    EndEllipsis,
    StartEllipsisModeled,
    MiddleEllipsisModeled,
}

impl ControlGenericTextOverflowPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clip => "clip",
            Self::EndEllipsis => "end-ellipsis",
            Self::StartEllipsisModeled => "start-ellipsis-modeled",
            Self::MiddleEllipsisModeled => "middle-ellipsis-modeled",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlGenericTextAlignmentPolicy {
    Start,
    Center,
    End,
}

impl ControlGenericTextAlignmentPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlGenericTextRoleDescriptor {
    pub role_id: String,
    pub semantic_role: ControlGenericTextSemanticRole,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub inline_spans_allowed: bool,
    #[serde(default)]
    pub max_lines: Option<u32>,
}

impl ControlGenericTextRoleDescriptor {
    pub fn new(role_id: impl Into<String>, semantic_role: ControlGenericTextSemanticRole) -> Self {
        Self {
            role_id: role_id.into(),
            semantic_role,
            required: true,
            inline_spans_allowed: false,
            max_lines: None,
        }
    }

    pub fn with_inline_spans(mut self) -> Self {
        self.inline_spans_allowed = true;
        self
    }

    pub fn with_max_lines(mut self, value: u32) -> Self {
        self.max_lines = Some(value);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlGenericTextLayoutSupport {
    pub wrap_policies: Vec<ControlGenericTextWrapPolicy>,
    pub overflow_policies: Vec<ControlGenericTextOverflowPolicy>,
    pub alignment_policies: Vec<ControlGenericTextAlignmentPolicy>,
    pub inline_spans: bool,
    pub line_metrics: bool,
    pub glyph_evidence: bool,
    pub fallback_evidence: bool,
    pub text_direction_policy: bool,
    pub renderer_backend_required: bool,
}

impl ControlGenericTextLayoutSupport {
    pub fn renderer_neutral_display() -> Self {
        Self {
            wrap_policies: vec![
                ControlGenericTextWrapPolicy::NoWrap,
                ControlGenericTextWrapPolicy::Word,
                ControlGenericTextWrapPolicy::Character,
            ],
            overflow_policies: vec![
                ControlGenericTextOverflowPolicy::Clip,
                ControlGenericTextOverflowPolicy::EndEllipsis,
                ControlGenericTextOverflowPolicy::StartEllipsisModeled,
                ControlGenericTextOverflowPolicy::MiddleEllipsisModeled,
            ],
            alignment_policies: vec![
                ControlGenericTextAlignmentPolicy::Start,
                ControlGenericTextAlignmentPolicy::Center,
                ControlGenericTextAlignmentPolicy::End,
            ],
            inline_spans: true,
            line_metrics: true,
            glyph_evidence: true,
            fallback_evidence: true,
            text_direction_policy: true,
            renderer_backend_required: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlGenericTextDescriptor {
    pub control_kind_id: ControlKindId,
    pub roles: Vec<ControlGenericTextRoleDescriptor>,
    pub layout_support: ControlGenericTextLayoutSupport,
    #[serde(default = "default_true")]
    pub proof_required: bool,
}

impl ControlGenericTextDescriptor {
    pub fn new(control_kind_id: ControlKindId) -> Self {
        Self {
            control_kind_id,
            roles: Vec::new(),
            layout_support: ControlGenericTextLayoutSupport::renderer_neutral_display(),
            proof_required: true,
        }
    }

    pub fn with_role(mut self, role: ControlGenericTextRoleDescriptor) -> Self {
        self.roles.push(role);
        self
    }

    pub fn summary(&self) -> ControlGenericTextSupportSummary {
        ControlGenericTextSupportSummary::from_descriptor(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlGenericTextSupportSummary {
    pub control_kind_id: ControlKindId,
    pub generic_text_supported: bool,
    pub roles: Vec<String>,
    pub semantic_roles: Vec<String>,
    pub wrap_policies: Vec<String>,
    pub overflow_policies: Vec<String>,
    pub alignment_policies: Vec<String>,
    pub inline_span_support: bool,
    pub line_metrics_support: bool,
    pub glyph_evidence_support: bool,
    pub fallback_evidence_support: bool,
    pub text_direction_policy_support: bool,
    pub renderer_backend_required: bool,
    pub executes_host_commands: bool,
    pub mutates_product_state: bool,
    pub authored_ui_edits: bool,
    pub product_undo_redo: bool,
}

impl ControlGenericTextSupportSummary {
    pub fn from_descriptor(descriptor: &ControlGenericTextDescriptor) -> Self {
        let mut roles = descriptor
            .roles
            .iter()
            .map(|role| role.role_id.clone())
            .collect::<Vec<_>>();
        roles.sort();
        roles.dedup();
        let mut semantic_roles = descriptor
            .roles
            .iter()
            .map(|role| role.semantic_role.as_str().to_owned())
            .collect::<Vec<_>>();
        semantic_roles.sort();
        semantic_roles.dedup();
        Self {
            control_kind_id: descriptor.control_kind_id.clone(),
            generic_text_supported: !descriptor.roles.is_empty(),
            roles,
            semantic_roles,
            wrap_policies: descriptor
                .layout_support
                .wrap_policies
                .iter()
                .map(|policy| policy.as_str().to_owned())
                .collect(),
            overflow_policies: descriptor
                .layout_support
                .overflow_policies
                .iter()
                .map(|policy| policy.as_str().to_owned())
                .collect(),
            alignment_policies: descriptor
                .layout_support
                .alignment_policies
                .iter()
                .map(|policy| policy.as_str().to_owned())
                .collect(),
            inline_span_support: descriptor.layout_support.inline_spans,
            line_metrics_support: descriptor.layout_support.line_metrics,
            glyph_evidence_support: descriptor.layout_support.glyph_evidence,
            fallback_evidence_support: descriptor.layout_support.fallback_evidence,
            text_direction_policy_support: descriptor.layout_support.text_direction_policy,
            renderer_backend_required: descriptor.layout_support.renderer_backend_required,
            executes_host_commands: false,
            mutates_product_state: false,
            authored_ui_edits: false,
            product_undo_redo: false,
        }
    }

    pub fn inspection_facts(&self) -> Vec<ControlGenericTextInspectionFact> {
        vec![
            ControlGenericTextInspectionFact::new(
                "text_display.supported",
                bool_string(self.generic_text_supported),
            ),
            ControlGenericTextInspectionFact::new("text_display.roles", self.roles.join(",")),
            ControlGenericTextInspectionFact::new(
                "text_display.semantic_roles",
                self.semantic_roles.join(","),
            ),
            ControlGenericTextInspectionFact::new(
                "text_display.wrap_policies",
                self.wrap_policies.join(","),
            ),
            ControlGenericTextInspectionFact::new(
                "text_display.overflow_policies",
                self.overflow_policies.join(","),
            ),
            ControlGenericTextInspectionFact::new(
                "text_display.alignment_policies",
                self.alignment_policies.join(","),
            ),
            ControlGenericTextInspectionFact::new(
                "text_display.inline_spans",
                bool_string(self.inline_span_support),
            ),
            ControlGenericTextInspectionFact::new(
                "text_display.line_metrics",
                bool_string(self.line_metrics_support),
            ),
            ControlGenericTextInspectionFact::new(
                "text_display.glyph_evidence",
                bool_string(self.glyph_evidence_support),
            ),
            ControlGenericTextInspectionFact::new(
                "text_display.fallback_evidence",
                bool_string(self.fallback_evidence_support),
            ),
            ControlGenericTextInspectionFact::new(
                "text_display.text_direction_policy",
                bool_string(self.text_direction_policy_support),
            ),
            ControlGenericTextInspectionFact::new(
                "text_display.renderer_backend_required",
                bool_string(self.renderer_backend_required),
            ),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlGenericTextInspectionFact {
    pub key: String,
    pub value: String,
}

impl ControlGenericTextInspectionFact {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn bool_string(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
