//! Theme token activation request types.

use serde::{Deserialize, Serialize};

use super::{
    ThemeAccessibilityModeId, ThemeModeId, ThemePlatformId, ThemeStateVariantId,
    ThemeTargetProfileId, ThemeTokenSourceId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeTokenActivationMode {
    Preview,
    Activate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeTokenResolveRequest {
    pub target_profile: ThemeTargetProfileId,
    pub component: Option<ThemeTokenSourceId>,
    pub state: Option<ThemeStateVariantId>,
    pub mode: Option<ThemeModeId>,
    pub platform: Option<ThemePlatformId>,
    pub accessibility: Option<ThemeAccessibilityModeId>,
    pub activation: ThemeTokenActivationMode,
}

impl ThemeTokenResolveRequest {
    pub fn activate(target_profile: impl Into<ThemeTargetProfileId>) -> Self {
        Self {
            target_profile: target_profile.into(),
            component: None,
            state: None,
            mode: None,
            platform: None,
            accessibility: None,
            activation: ThemeTokenActivationMode::Activate,
        }
    }

    pub fn preview(target_profile: impl Into<ThemeTargetProfileId>) -> Self {
        Self {
            activation: ThemeTokenActivationMode::Preview,
            ..Self::activate(target_profile)
        }
    }
}
