//! File: domain/ui/ui_controls/src/theme.rs
//! Crate: ui_controls

use serde::{Deserialize, Serialize};

use crate::package::ids::ControlKindId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlThemeTokenKind {
    Color,
    Spacing,
    Typography,
    Radius,
    Border,
    Opacity,
    Elevation,
}

impl ControlThemeTokenKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Color => "color",
            Self::Spacing => "spacing",
            Self::Typography => "typography",
            Self::Radius => "radius",
            Self::Border => "border",
            Self::Opacity => "opacity",
            Self::Elevation => "elevation",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlThemeTokenRole {
    Base,
    Accent,
    Surface,
    Text,
    Border,
    FocusRing,
    Overlay,
    Feedback,
}

impl ControlThemeTokenRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Accent => "accent",
            Self::Surface => "surface",
            Self::Text => "text",
            Self::Border => "border",
            Self::FocusRing => "focus-ring",
            Self::Overlay => "overlay",
            Self::Feedback => "feedback",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlThemeTokenRequirement {
    pub token_id: String,
    pub kind: ControlThemeTokenKind,
    pub role: ControlThemeTokenRole,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default)]
    pub fallback_token_id: Option<String>,
    #[serde(default)]
    pub notes: String,
}

impl ControlThemeTokenRequirement {
    pub fn new(
        token_id: impl Into<String>,
        kind: ControlThemeTokenKind,
        role: ControlThemeTokenRole,
    ) -> Self {
        Self {
            token_id: token_id.into(),
            kind,
            role,
            required: true,
            fallback_token_id: None,
            notes: String::new(),
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_fallback(mut self, fallback_token_id: impl Into<String>) -> Self {
        self.fallback_token_id = Some(fallback_token_id.into());
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlVisualState {
    Normal,
    Hover,
    Pressed,
    Focused,
    Selected,
    Disabled,
    Error,
    Warning,
    Info,
    Active,
    Loading,
    ReadOnly,
}

impl ControlVisualState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Hover => "hover",
            Self::Pressed => "pressed",
            Self::Focused => "focused",
            Self::Selected => "selected",
            Self::Disabled => "disabled",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Active => "active",
            Self::Loading => "loading",
            Self::ReadOnly => "read-only",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlVisualStateRequirement {
    pub state: ControlVisualState,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default)]
    pub notes: String,
}

impl ControlVisualStateRequirement {
    pub fn new(state: ControlVisualState) -> Self {
        Self {
            state,
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
pub enum ControlStyleRole {
    Container,
    Label,
    Icon,
    Value,
    Background,
    Foreground,
    Border,
    Accent,
    FocusRing,
    Overlay,
}

impl ControlStyleRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Container => "container",
            Self::Label => "label",
            Self::Icon => "icon",
            Self::Value => "value",
            Self::Background => "background",
            Self::Foreground => "foreground",
            Self::Border => "border",
            Self::Accent => "accent",
            Self::FocusRing => "focus-ring",
            Self::Overlay => "overlay",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStyleRequirement {
    pub role: ControlStyleRole,
    pub token_id: String,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default)]
    pub notes: String,
}

impl ControlStyleRequirement {
    pub fn new(role: ControlStyleRole, token_id: impl Into<String>) -> Self {
        Self {
            role,
            token_id: token_id.into(),
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
pub enum ControlStyleDiagnosticKind {
    FallbackToken,
    MissingToken,
    ExpectedFailure,
}

impl ControlStyleDiagnosticKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FallbackToken => "fallback-token",
            Self::MissingToken => "missing-token-diagnostic",
            Self::ExpectedFailure => "expected-failure",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStyleFallback {
    pub token_id: String,
    pub fallback_token_id: String,
    pub diagnostic: ControlStyleDiagnosticKind,
}

impl ControlStyleFallback {
    pub fn new(token_id: impl Into<String>, fallback_token_id: impl Into<String>) -> Self {
        Self {
            token_id: token_id.into(),
            fallback_token_id: fallback_token_id.into(),
            diagnostic: ControlStyleDiagnosticKind::FallbackToken,
        }
    }

    pub fn with_diagnostic(mut self, diagnostic: ControlStyleDiagnosticKind) -> Self {
        self.diagnostic = diagnostic;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStyleDiagnostic {
    pub diagnostic_id: String,
    pub kind: ControlStyleDiagnosticKind,
    pub token_id: String,
    pub message: String,
}

impl ControlStyleDiagnostic {
    pub fn new(
        diagnostic_id: impl Into<String>,
        kind: ControlStyleDiagnosticKind,
        token_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            diagnostic_id: diagnostic_id.into(),
            kind,
            token_id: token_id.into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlThemeDescriptor {
    pub control_kind_id: ControlKindId,
    #[serde(default)]
    pub token_requirements: Vec<ControlThemeTokenRequirement>,
    #[serde(default)]
    pub visual_states: Vec<ControlVisualStateRequirement>,
    #[serde(default)]
    pub style_requirements: Vec<ControlStyleRequirement>,
    #[serde(default)]
    pub fallbacks: Vec<ControlStyleFallback>,
    #[serde(default)]
    pub diagnostics: Vec<ControlStyleDiagnostic>,
}

impl ControlThemeDescriptor {
    pub fn new(control_kind_id: ControlKindId) -> Self {
        Self {
            control_kind_id,
            token_requirements: Vec::new(),
            visual_states: Vec::new(),
            style_requirements: Vec::new(),
            fallbacks: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn with_token(mut self, token: ControlThemeTokenRequirement) -> Self {
        self.token_requirements.push(token);
        self.token_requirements
            .sort_by(|left, right| left.token_id.cmp(&right.token_id));
        self.token_requirements
            .dedup_by(|left, right| left.token_id == right.token_id);
        self
    }

    pub fn with_visual_state(mut self, state: ControlVisualStateRequirement) -> Self {
        self.visual_states.push(state);
        self.visual_states.sort_by_key(|state| state.state);
        self.visual_states.dedup_by_key(|state| state.state);
        self
    }

    pub fn with_style(mut self, style: ControlStyleRequirement) -> Self {
        self.style_requirements.push(style);
        self.style_requirements.sort_by_key(|style| style.role);
        self.style_requirements.dedup_by_key(|style| style.role);
        self
    }

    pub fn with_fallback(mut self, fallback: ControlStyleFallback) -> Self {
        self.fallbacks.push(fallback);
        self.fallbacks
            .sort_by(|left, right| left.token_id.cmp(&right.token_id));
        self.fallbacks
            .dedup_by(|left, right| left.token_id == right.token_id);
        self
    }

    pub fn with_diagnostic(mut self, diagnostic: ControlStyleDiagnostic) -> Self {
        self.diagnostics.push(diagnostic);
        self.diagnostics
            .sort_by(|left, right| left.diagnostic_id.cmp(&right.diagnostic_id));
        self.diagnostics
            .dedup_by(|left, right| left.diagnostic_id == right.diagnostic_id);
        self
    }

    pub fn summary(&self) -> ControlThemeCapabilitySummary {
        ControlThemeCapabilitySummary::from_descriptor(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlThemeCapabilitySummary {
    pub control_kind_id: ControlKindId,
    pub required_token_kinds: Vec<String>,
    pub optional_token_kinds: Vec<String>,
    pub token_roles: Vec<String>,
    pub visual_states: Vec<String>,
    pub style_roles: Vec<String>,
    pub fallback_tokens: Vec<String>,
    pub diagnostics: Vec<String>,
    pub expected_failures: Vec<String>,
    pub has_runtime_style_behavior: bool,
}

impl ControlThemeCapabilitySummary {
    pub fn from_descriptor(descriptor: &ControlThemeDescriptor) -> Self {
        let mut required_token_kinds = descriptor
            .token_requirements
            .iter()
            .filter(|token| token.required)
            .map(|token| token.kind.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut optional_token_kinds = descriptor
            .token_requirements
            .iter()
            .filter(|token| !token.required)
            .map(|token| token.kind.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut token_roles = descriptor
            .token_requirements
            .iter()
            .map(|token| token.role.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut visual_states = descriptor
            .visual_states
            .iter()
            .map(|state| state.state.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut style_roles = descriptor
            .style_requirements
            .iter()
            .map(|style| style.role.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut fallback_tokens = descriptor
            .fallbacks
            .iter()
            .map(|fallback| fallback.fallback_token_id.clone())
            .chain(
                descriptor
                    .token_requirements
                    .iter()
                    .filter_map(|token| token.fallback_token_id.clone()),
            )
            .collect::<Vec<_>>();
        let mut diagnostics = descriptor
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.as_str().to_owned())
            .chain(
                descriptor
                    .fallbacks
                    .iter()
                    .map(|fallback| fallback.diagnostic.as_str().to_owned()),
            )
            .collect::<Vec<_>>();
        let mut expected_failures = descriptor
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == ControlStyleDiagnosticKind::ExpectedFailure)
            .map(|diagnostic| diagnostic.diagnostic_id.clone())
            .collect::<Vec<_>>();

        sort_dedup(&mut required_token_kinds);
        sort_dedup(&mut optional_token_kinds);
        sort_dedup(&mut token_roles);
        sort_dedup(&mut visual_states);
        sort_dedup(&mut style_roles);
        sort_dedup(&mut fallback_tokens);
        sort_dedup(&mut diagnostics);
        sort_dedup(&mut expected_failures);

        Self {
            control_kind_id: descriptor.control_kind_id.clone(),
            required_token_kinds,
            optional_token_kinds,
            token_roles,
            visual_states,
            style_roles,
            fallback_tokens,
            diagnostics,
            expected_failures,
            has_runtime_style_behavior: false,
        }
    }

    pub fn inspection_facts(&self) -> Vec<ControlThemeInspectionFact> {
        vec![
            ControlThemeInspectionFact::new(
                "required_token_kinds",
                self.required_token_kinds.join(","),
            ),
            ControlThemeInspectionFact::new(
                "optional_token_kinds",
                self.optional_token_kinds.join(","),
            ),
            ControlThemeInspectionFact::new("token_roles", self.token_roles.join(",")),
            ControlThemeInspectionFact::new("visual_states", self.visual_states.join(",")),
            ControlThemeInspectionFact::new("style_roles", self.style_roles.join(",")),
            ControlThemeInspectionFact::new("fallback_tokens", self.fallback_tokens.join(",")),
            ControlThemeInspectionFact::new("diagnostics", self.diagnostics.join(",")),
            ControlThemeInspectionFact::new("expected_failures", self.expected_failures.join(",")),
            ControlThemeInspectionFact::new(
                "has_runtime_style_behavior",
                bool_string(self.has_runtime_style_behavior),
            ),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlThemeInspectionFact {
    pub key: String,
    pub value: String,
}

impl ControlThemeInspectionFact {
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
