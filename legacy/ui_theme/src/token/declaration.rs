//! Theme token declarations, values, and selectors.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::UiColor;

use super::activation::ThemeTokenResolveRequest;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ThemeTokenId(String);

impl ThemeTokenId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ThemeTokenId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ThemeTokenId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ThemeTokenSourceId(String);

impl ThemeTokenSourceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ThemeTokenSourceId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ThemeTokenSourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ThemeTokenSourcePath(String);

impl ThemeTokenSourcePath {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ThemeTokenSourcePath {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ThemeTokenSourcePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub type ThemeTargetProfileId = ThemeTokenSourceId;
pub type ThemeModeId = ThemeTokenSourceId;
pub type ThemeStateVariantId = ThemeTokenSourceId;
pub type ThemePlatformId = ThemeTokenSourceId;
pub type ThemeAccessibilityModeId = ThemeTokenSourceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ThemeTokenLayer {
    Primitive,
    Semantic,
    Component,
    State,
    Mode,
    Theme,
    Skin,
    Platform,
    Accessibility,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ThemeTokenFamily {
    Color,
    Spacing,
    Radius,
    Typography,
    Opacity,
    Elevation,
    BorderWidth,
    Duration,
    Easing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThemeTokenValue {
    Color(UiColor),
    Number(f32),
    Text(String),
}

impl ThemeTokenValue {
    pub fn family(&self) -> ThemeTokenFamily {
        match self {
            Self::Color(_) => ThemeTokenFamily::Color,
            Self::Number(_) => ThemeTokenFamily::Spacing,
            Self::Text(_) => ThemeTokenFamily::Easing,
        }
    }

    fn is_valid_for_family(&self, family: ThemeTokenFamily) -> bool {
        match (family, self) {
            (ThemeTokenFamily::Color, Self::Color(value)) => [value.r, value.g, value.b, value.a]
                .into_iter()
                .all(|component| component.is_finite() && (0.0..=1.0).contains(&component)),
            (
                ThemeTokenFamily::Spacing
                | ThemeTokenFamily::Radius
                | ThemeTokenFamily::Typography
                | ThemeTokenFamily::Opacity
                | ThemeTokenFamily::Elevation
                | ThemeTokenFamily::BorderWidth
                | ThemeTokenFamily::Duration,
                Self::Number(value),
            ) => value.is_finite() && *value >= 0.0,
            (ThemeTokenFamily::Easing, Self::Text(value)) => !value.trim().is_empty(),
            _ => false,
        }
    }

    pub(super) fn validation_error_code(&self, family: ThemeTokenFamily) -> Option<&'static str> {
        let kind_matches = matches!(
            (family, self),
            (ThemeTokenFamily::Color, Self::Color(_))
                | (
                    ThemeTokenFamily::Spacing
                        | ThemeTokenFamily::Radius
                        | ThemeTokenFamily::Typography
                        | ThemeTokenFamily::Opacity
                        | ThemeTokenFamily::Elevation
                        | ThemeTokenFamily::BorderWidth
                        | ThemeTokenFamily::Duration,
                    Self::Number(_),
                )
                | (ThemeTokenFamily::Easing, Self::Text(_))
        );

        if !kind_matches {
            Some("ui.theme.token.family_mismatch")
        } else if !self.is_valid_for_family(family) {
            Some("ui.theme.token.malformed_value")
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThemeTokenValueSource {
    Value(ThemeTokenValue),
    Alias(ThemeTokenId),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ThemeTokenSelector {
    pub component: Option<ThemeTokenSourceId>,
    pub state: Option<ThemeStateVariantId>,
    pub mode: Option<ThemeModeId>,
    pub platform: Option<ThemePlatformId>,
    pub accessibility: Option<ThemeAccessibilityModeId>,
}

impl ThemeTokenSelector {
    pub fn matches(&self, request: &ThemeTokenResolveRequest) -> bool {
        selector_matches(&self.component, &request.component)
            && selector_matches(&self.state, &request.state)
            && selector_matches(&self.mode, &request.mode)
            && selector_matches(&self.platform, &request.platform)
            && selector_matches(&self.accessibility, &request.accessibility)
    }
}

fn selector_matches<T: PartialEq>(selector: &Option<T>, request: &Option<T>) -> bool {
    selector.is_none() || selector == request
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeTokenDeclaration {
    pub id: ThemeTokenId,
    pub family: ThemeTokenFamily,
    pub layer: ThemeTokenLayer,
    pub value: ThemeTokenValueSource,
    pub source: ThemeTokenSourceId,
    pub source_path: Option<ThemeTokenSourcePath>,
    pub target_profiles: Vec<ThemeTargetProfileId>,
    pub selector: ThemeTokenSelector,
    pub preview_only: bool,
}

impl ThemeTokenDeclaration {
    pub fn value(
        id: impl Into<ThemeTokenId>,
        family: ThemeTokenFamily,
        layer: ThemeTokenLayer,
        value: ThemeTokenValue,
        source: impl Into<ThemeTokenSourceId>,
    ) -> Self {
        Self {
            id: id.into(),
            family,
            layer,
            value: ThemeTokenValueSource::Value(value),
            source: source.into(),
            source_path: None,
            target_profiles: Vec::new(),
            selector: ThemeTokenSelector::default(),
            preview_only: false,
        }
    }

    pub fn alias(
        id: impl Into<ThemeTokenId>,
        family: ThemeTokenFamily,
        layer: ThemeTokenLayer,
        alias: impl Into<ThemeTokenId>,
        source: impl Into<ThemeTokenSourceId>,
    ) -> Self {
        Self {
            id: id.into(),
            family,
            layer,
            value: ThemeTokenValueSource::Alias(alias.into()),
            source: source.into(),
            source_path: None,
            target_profiles: Vec::new(),
            selector: ThemeTokenSelector::default(),
            preview_only: false,
        }
    }

    pub fn for_target_profiles(
        mut self,
        profiles: impl IntoIterator<Item = ThemeTargetProfileId>,
    ) -> Self {
        self.target_profiles = profiles.into_iter().collect();
        self
    }

    pub fn with_selector(mut self, selector: ThemeTokenSelector) -> Self {
        self.selector = selector;
        self
    }

    pub fn with_source_path(mut self, source_path: impl Into<ThemeTokenSourcePath>) -> Self {
        self.source_path = Some(source_path.into());
        self
    }

    pub fn preview_only(mut self) -> Self {
        self.preview_only = true;
        self
    }

    pub(super) fn supports_profile(&self, profile: &ThemeTargetProfileId) -> bool {
        self.target_profiles.is_empty() || self.target_profiles.contains(profile)
    }
}
