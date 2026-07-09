//! File: domain/ui/ui_controls/src/base_control/theme_group.rs
//! Crate: ui_controls

use crate::{ControlStyleRole, ControlThemeTokenKind, ControlThemeTokenRole, ControlVisualState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlThemeGroup {
    pub group_id: String,
    pub tokens: Vec<ControlThemeTokenIntent>,
    pub styles: Vec<ControlStyleIntent>,
    pub visual_states: Vec<ControlVisualStateIntent>,
}

impl ControlThemeGroup {
    pub fn base(group_id: impl Into<String>) -> Self {
        Self::new(group_id)
            .with_token(
                "foreground",
                ControlThemeTokenKind::Color,
                ControlThemeTokenRole::Text,
            )
            .with_token(
                "spacing",
                ControlThemeTokenKind::Spacing,
                ControlThemeTokenRole::Base,
            )
            .with_style(ControlStyleRole::Label, "foreground")
            .with_visual_state(ControlVisualState::Normal)
            .with_optional_visual_state(ControlVisualState::Disabled)
    }

    pub fn surface(group_id: impl Into<String>) -> Self {
        Self::base(group_id)
            .with_token(
                "surface",
                ControlThemeTokenKind::Color,
                ControlThemeTokenRole::Surface,
            )
            .with_token(
                "border",
                ControlThemeTokenKind::Border,
                ControlThemeTokenRole::Border,
            )
            .with_token(
                "radius",
                ControlThemeTokenKind::Radius,
                ControlThemeTokenRole::Base,
            )
            .with_style(ControlStyleRole::Border, "border")
    }

    pub fn new(group_id: impl Into<String>) -> Self {
        Self {
            group_id: group_id.into(),
            tokens: Vec::new(),
            styles: Vec::new(),
            visual_states: Vec::new(),
        }
    }

    pub fn with_token(
        mut self,
        token_name: impl Into<String>,
        kind: ControlThemeTokenKind,
        role: ControlThemeTokenRole,
    ) -> Self {
        self.tokens
            .push(ControlThemeTokenIntent::required(token_name, kind, role));
        self
    }

    pub fn with_optional_token(
        mut self,
        token_name: impl Into<String>,
        kind: ControlThemeTokenKind,
        role: ControlThemeTokenRole,
    ) -> Self {
        self.tokens
            .push(ControlThemeTokenIntent::optional(token_name, kind, role));
        self
    }

    pub fn with_style(mut self, role: ControlStyleRole, token_name: impl Into<String>) -> Self {
        self.styles
            .push(ControlStyleIntent::required(role, token_name));
        self
    }

    pub fn with_optional_style(
        mut self,
        role: ControlStyleRole,
        token_name: impl Into<String>,
    ) -> Self {
        self.styles
            .push(ControlStyleIntent::optional(role, token_name));
        self
    }

    pub fn with_visual_state(mut self, state: ControlVisualState) -> Self {
        self.visual_states
            .push(ControlVisualStateIntent::required(state));
        self
    }

    pub fn with_optional_visual_state(mut self, state: ControlVisualState) -> Self {
        self.visual_states
            .push(ControlVisualStateIntent::optional(state));
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlThemeTokenIntent {
    pub token_name: String,
    pub kind: ControlThemeTokenKind,
    pub role: ControlThemeTokenRole,
    pub required: bool,
}

impl ControlThemeTokenIntent {
    pub fn required(
        token_name: impl Into<String>,
        kind: ControlThemeTokenKind,
        role: ControlThemeTokenRole,
    ) -> Self {
        Self {
            token_name: token_name.into(),
            kind,
            role,
            required: true,
        }
    }

    pub fn optional(
        token_name: impl Into<String>,
        kind: ControlThemeTokenKind,
        role: ControlThemeTokenRole,
    ) -> Self {
        Self {
            token_name: token_name.into(),
            kind,
            role,
            required: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlStyleIntent {
    pub role: ControlStyleRole,
    pub token_name: String,
    pub required: bool,
}

impl ControlStyleIntent {
    pub fn required(role: ControlStyleRole, token_name: impl Into<String>) -> Self {
        Self {
            role,
            token_name: token_name.into(),
            required: true,
        }
    }

    pub fn optional(role: ControlStyleRole, token_name: impl Into<String>) -> Self {
        Self {
            role,
            token_name: token_name.into(),
            required: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlVisualStateIntent {
    pub state: ControlVisualState,
    pub required: bool,
}

impl ControlVisualStateIntent {
    pub const fn required(state: ControlVisualState) -> Self {
        Self {
            state,
            required: true,
        }
    }

    pub const fn optional(state: ControlVisualState) -> Self {
        Self {
            state,
            required: false,
        }
    }
}
