//! Host-neutral production style environment and layered theme vocabulary.

use core::{error::Error, fmt};
use std::collections::{BTreeMap, btree_map::Entry};

use crate::{StyleProperties, StyleRecipeId, StyleTokens, StyleVariantId};

/// Canonical transient interaction facts consumed by style resolution.
///
/// These are values only. `runenui_runtime` remains the sole live authority for
/// hover, focus, press/active, and widget enablement state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StyleInteractionFacts {
    hovered: bool,
    focused: bool,
    active: bool,
    disabled: bool,
}

impl StyleInteractionFacts {
    #[must_use]
    pub const fn new(hovered: bool, focused: bool, active: bool, disabled: bool) -> Self {
        Self {
            hovered,
            focused,
            active,
            disabled,
        }
    }
    #[must_use]
    pub const fn hovered(self) -> bool {
        self.hovered
    }
    #[must_use]
    pub const fn focused(self) -> bool {
        self.focused
    }
    #[must_use]
    pub const fn active(self) -> bool {
        self.active
    }
    #[must_use]
    pub const fn disabled(self) -> bool {
        self.disabled
    }
}

/// Framework-ordered interaction layer.
///
/// Resolution order is hover, focus, active, disabled; later active layers win
/// property-by-property.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StyleInteractionState {
    Hover,
    Focus,
    Active,
    Disabled,
}

impl StyleInteractionState {
    pub const ORDERED: [Self; 4] = [Self::Hover, Self::Focus, Self::Active, Self::Disabled];

    #[must_use]
    pub const fn is_active(self, facts: StyleInteractionFacts) -> bool {
        match self {
            Self::Hover => facts.hovered(),
            Self::Focus => facts.focused(),
            Self::Active => facts.active(),
            Self::Disabled => facts.disabled(),
        }
    }
}

/// Explicit user/platform preference facts supplied to style resolution.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StylePreferences {
    high_contrast: bool,
    reduced_motion: bool,
}

impl StylePreferences {
    #[must_use]
    pub const fn new(high_contrast: bool, reduced_motion: bool) -> Self {
        Self {
            high_contrast,
            reduced_motion,
        }
    }
    #[must_use]
    pub const fn high_contrast(self) -> bool {
        self.high_contrast
    }
    #[must_use]
    pub const fn reduced_motion(self) -> bool {
        self.reduced_motion
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StylePreferenceKind {
    ReducedMotion,
    HighContrast,
}

/// Mandatory preference policy applied above authored overrides.
///
/// M8A has no animation property family, so reduced motion is currently an
/// explicit cache/invalidation fact without a style-property override. M9 may
/// extend this policy without changing preference ownership.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StylePreferencePolicy {
    high_contrast: StyleProperties,
}

impl StylePreferencePolicy {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            high_contrast: StyleProperties::EMPTY,
        }
    }
    #[must_use]
    pub fn with_high_contrast(mut self, properties: StyleProperties) -> Self {
        self.high_contrast = properties;
        self
    }
    #[must_use]
    pub const fn high_contrast(&self) -> &StyleProperties {
        &self.high_contrast
    }
}

/// Theme-owned recipe base, ordered-variant definitions, and interaction layers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleRecipe {
    base: StyleProperties,
    variants: BTreeMap<StyleVariantId, StyleProperties>,
    interactions: BTreeMap<StyleInteractionState, StyleProperties>,
}

impl StyleRecipe {
    #[must_use]
    pub fn new(base: StyleProperties) -> Self {
        Self {
            base,
            variants: BTreeMap::new(),
            interactions: BTreeMap::new(),
        }
    }
    #[must_use]
    pub const fn base(&self) -> &StyleProperties {
        &self.base
    }
    pub fn define_variant(
        &mut self,
        id: StyleVariantId,
        properties: StyleProperties,
    ) -> Result<(), DuplicateStyleDefinition> {
        match self.variants.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(properties);
                Ok(())
            }
            Entry::Occupied(entry) => Err(DuplicateStyleDefinition::Variant(entry.key().clone())),
        }
    }
    pub fn define_interaction(
        &mut self,
        state: StyleInteractionState,
        properties: StyleProperties,
    ) -> Result<(), DuplicateStyleDefinition> {
        match self.interactions.entry(state) {
            Entry::Vacant(entry) => {
                entry.insert(properties);
                Ok(())
            }
            Entry::Occupied(_) => Err(DuplicateStyleDefinition::Interaction(state)),
        }
    }
    #[must_use]
    pub fn variant(&self, id: &StyleVariantId) -> Option<&StyleProperties> {
        self.variants.get(id)
    }
    #[must_use]
    pub fn interaction(&self, state: StyleInteractionState) -> Option<&StyleProperties> {
        self.interactions.get(&state)
    }
}

/// One explicit theme: exact token content plus typed recipes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleTheme {
    tokens: StyleTokens,
    recipes: BTreeMap<StyleRecipeId, StyleRecipe>,
}

impl StyleTheme {
    #[must_use]
    pub fn new(tokens: StyleTokens) -> Self {
        Self {
            tokens,
            recipes: BTreeMap::new(),
        }
    }
    #[must_use]
    pub const fn tokens(&self) -> &StyleTokens {
        &self.tokens
    }
    pub fn define_recipe(
        &mut self,
        id: StyleRecipeId,
        recipe: StyleRecipe,
    ) -> Result<(), DuplicateStyleDefinition> {
        match self.recipes.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(recipe);
                Ok(())
            }
            Entry::Occupied(entry) => Err(DuplicateStyleDefinition::Recipe(entry.key().clone())),
        }
    }
    #[must_use]
    pub fn recipe(&self, id: &StyleRecipeId) -> Option<&StyleRecipe> {
        self.recipes.get(id)
    }
}

/// Complete host-neutral style input for one publication attempt.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleEnvironment {
    framework_defaults: StyleProperties,
    theme: StyleTheme,
    preferences: StylePreferences,
    preference_policy: StylePreferencePolicy,
}

impl StyleEnvironment {
    #[must_use]
    pub fn new(theme: StyleTheme) -> Self {
        Self {
            framework_defaults: StyleProperties::EMPTY,
            theme,
            preferences: StylePreferences::default(),
            preference_policy: StylePreferencePolicy::default(),
        }
    }
    #[must_use]
    pub fn from_tokens(tokens: StyleTokens) -> Self {
        Self::new(StyleTheme::new(tokens))
    }
    #[must_use]
    pub fn with_framework_defaults(mut self, properties: StyleProperties) -> Self {
        self.framework_defaults = properties;
        self
    }
    #[must_use]
    pub const fn framework_defaults(&self) -> &StyleProperties {
        &self.framework_defaults
    }
    #[must_use]
    pub const fn theme(&self) -> &StyleTheme {
        &self.theme
    }
    #[must_use]
    pub const fn preferences(&self) -> StylePreferences {
        self.preferences
    }
    #[must_use]
    pub const fn preference_policy(&self) -> &StylePreferencePolicy {
        &self.preference_policy
    }
    #[must_use]
    pub const fn with_preferences(mut self, preferences: StylePreferences) -> Self {
        self.preferences = preferences;
        self
    }
    #[must_use]
    pub fn with_preference_policy(mut self, policy: StylePreferencePolicy) -> Self {
        self.preference_policy = policy;
        self
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DuplicateStyleDefinition {
    Recipe(StyleRecipeId),
    Variant(StyleVariantId),
    Interaction(StyleInteractionState),
}

impl fmt::Display for DuplicateStyleDefinition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Recipe(id) => write!(formatter, "duplicate style recipe: {}", id.as_str()),
            Self::Variant(id) => write!(formatter, "duplicate style variant: {}", id.as_str()),
            Self::Interaction(state) => write!(formatter, "duplicate interaction style: {state:?}"),
        }
    }
}

impl Error for DuplicateStyleDefinition {}
