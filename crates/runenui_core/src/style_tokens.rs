//! Validated in-memory style-token definitions.

use core::{error::Error, fmt};
use std::collections::{BTreeMap, btree_map::Entry};

use crate::{
    Color, ColorToken, EdgeInsets, Radius, RadiusToken, SpacingToken, TokenId, Typography,
    TypographyToken,
};

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Current style-token family.
///
/// This enum is non-exhaustive because later style milestones may add families
/// such as borders, shadows, or opacity.
pub enum TokenFamily {
    Color,
    Spacing,
    Radius,
    Typography,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateTokenDefinition {
    family: TokenFamily,
    token: TokenId,
}

impl DuplicateTokenDefinition {
    #[must_use]
    pub const fn family(&self) -> TokenFamily {
        self.family
    }
    #[must_use]
    pub const fn token(&self) -> &TokenId {
        &self.token
    }
}

impl fmt::Display for DuplicateTokenDefinition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "duplicate {:?} token definition: {}",
            self.family,
            self.token.as_str()
        )
    }
}

impl Error for DuplicateTokenDefinition {}

#[derive(Clone, Debug, Default)]
pub struct StyleTokens {
    colors: BTreeMap<ColorToken, Color>,
    spacing: BTreeMap<SpacingToken, EdgeInsets>,
    radii: BTreeMap<RadiusToken, Radius>,
    typography: BTreeMap<TypographyToken, Typography>,
    revision: u64,
}

impl PartialEq for StyleTokens {
    fn eq(&self, other: &Self) -> bool {
        self.colors == other.colors
            && self.spacing == other.spacing
            && self.radii == other.radii
            && self.typography == other.typography
    }
}

impl StyleTokens {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Defines a color token without replacement.
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateTokenDefinition`] if the color token already exists.
    pub fn define_color(
        &mut self,
        token: ColorToken,
        value: Color,
    ) -> Result<(), DuplicateTokenDefinition> {
        define(
            &mut self.colors,
            token,
            value,
            TokenFamily::Color,
            ColorToken::id,
        )?;
        self.advance_revision();
        Ok(())
    }

    /// Defines a spacing token without replacement.
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateTokenDefinition`] if the spacing token already exists.
    pub fn define_spacing(
        &mut self,
        token: SpacingToken,
        value: EdgeInsets,
    ) -> Result<(), DuplicateTokenDefinition> {
        define(
            &mut self.spacing,
            token,
            value,
            TokenFamily::Spacing,
            SpacingToken::id,
        )?;
        self.advance_revision();
        Ok(())
    }

    /// Defines a radius token without replacement.
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateTokenDefinition`] if the radius token already exists.
    pub fn define_radius(
        &mut self,
        token: RadiusToken,
        value: Radius,
    ) -> Result<(), DuplicateTokenDefinition> {
        define(
            &mut self.radii,
            token,
            value,
            TokenFamily::Radius,
            RadiusToken::id,
        )?;
        self.advance_revision();
        Ok(())
    }

    /// Defines a metric typography token without replacement.
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateTokenDefinition`] if the typography token already exists.
    pub fn define_typography(
        &mut self,
        token: TypographyToken,
        value: Typography,
    ) -> Result<(), DuplicateTokenDefinition> {
        define(
            &mut self.typography,
            token,
            value,
            TokenFamily::Typography,
            TypographyToken::id,
        )?;
        self.advance_revision();
        Ok(())
    }

    #[must_use]
    pub fn color(&self, token: &ColorToken) -> Option<Color> {
        self.colors.get(token).copied()
    }
    #[must_use]
    pub fn spacing(&self, token: &SpacingToken) -> Option<EdgeInsets> {
        self.spacing.get(token).copied()
    }
    #[must_use]
    pub fn radius(&self, token: &RadiusToken) -> Option<Radius> {
        self.radii.get(token).copied()
    }
    #[must_use]
    pub fn typography(&self, token: &TypographyToken) -> Option<&Typography> {
        self.typography.get(token)
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
            && self.spacing.is_empty()
            && self.radii.is_empty()
            && self.typography.is_empty()
    }

    /// Monotonic diagnostic revision for callers that want a cheap change hint.
    ///
    /// Cache correctness must compare token content because independent token
    /// sets and saturated revisions can share this value.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    const fn advance_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub const fn __seed_revision_for_test(&mut self, revision: u64) {
        self.revision = revision;
    }
}

fn define<Key: Ord, Value>(
    map: &mut BTreeMap<Key, Value>,
    key: Key,
    value: Value,
    family: TokenFamily,
    token_id: fn(&Key) -> &TokenId,
) -> Result<(), DuplicateTokenDefinition> {
    let token = token_id(&key).clone();
    match map.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert(value);
            Ok(())
        }
        Entry::Occupied(_) => Err(DuplicateTokenDefinition { family, token }),
    }
}
