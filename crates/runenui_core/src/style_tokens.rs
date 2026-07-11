//! In-memory style token values.

use std::collections::BTreeMap;

use crate::{Color, ColorToken, EdgeInsets, Radius, RadiusToken, SpacingToken};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleTokens {
    colors: BTreeMap<ColorToken, Color>,
    spacing: BTreeMap<SpacingToken, EdgeInsets>,
    radii: BTreeMap<RadiusToken, Radius>,
}

impl StyleTokens {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_color(mut self, token: impl Into<ColorToken>, value: Color) -> Self {
        self.colors.insert(token.into(), value);
        self
    }

    #[must_use]
    pub fn with_spacing(mut self, token: impl Into<SpacingToken>, value: EdgeInsets) -> Self {
        self.spacing.insert(token.into(), value);
        self
    }

    #[must_use]
    pub fn with_radius(mut self, token: impl Into<RadiusToken>, value: Radius) -> Self {
        self.radii.insert(token.into(), value);
        self
    }

    pub fn insert_color(&mut self, token: impl Into<ColorToken>, value: Color) -> Option<Color> {
        self.colors.insert(token.into(), value)
    }

    pub fn insert_spacing(
        &mut self,
        token: impl Into<SpacingToken>,
        value: EdgeInsets,
    ) -> Option<EdgeInsets> {
        self.spacing.insert(token.into(), value)
    }

    pub fn insert_radius(
        &mut self,
        token: impl Into<RadiusToken>,
        value: Radius,
    ) -> Option<Radius> {
        self.radii.insert(token.into(), value)
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
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty() && self.spacing.is_empty() && self.radii.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::StyleTokens;
    use crate::{Color, ColorToken, EdgeInsets, Length, Radius, RadiusToken, SpacingToken};

    #[test]
    fn token_map_defaults_to_empty() {
        let tokens = StyleTokens::default();

        assert!(tokens.is_empty());
        assert_eq!(tokens.color(&ColorToken::new("color.text.primary")), None);
        assert_eq!(tokens.spacing(&SpacingToken::new("space.2")), None);
    }

    #[test]
    fn token_map_stores_typed_values() {
        let spacing = EdgeInsets::all(Length::px(8.0));
        let radius = Radius::all(Length::px(4.0));
        let tokens = StyleTokens::new()
            .with_color("color.text.primary", Color::WHITE)
            .with_spacing("space.2", spacing)
            .with_radius("radius.control", radius);

        assert!(!tokens.is_empty());
        assert_eq!(
            tokens.color(&ColorToken::new("color.text.primary")),
            Some(Color::WHITE)
        );
        assert_eq!(tokens.spacing(&SpacingToken::new("space.2")), Some(spacing));
        assert_eq!(
            tokens.radius(&RadiusToken::new("radius.control")),
            Some(radius)
        );
    }

    #[test]
    fn insert_returns_replaced_value() {
        let mut tokens = StyleTokens::new();

        assert_eq!(
            tokens.insert_color("color.text.primary", Color::WHITE),
            None
        );
        assert_eq!(
            tokens.insert_color("color.text.primary", Color::BLACK),
            Some(Color::WHITE)
        );
        assert_eq!(
            tokens.color(&ColorToken::new("color.text.primary")),
            Some(Color::BLACK)
        );
    }
}
