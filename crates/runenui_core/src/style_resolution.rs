//! Pure style-resolution helpers.

use crate::{
    ColorToken, ColorValue, ComputedStyle, RadiusToken, RadiusValue, SpacingToken, SpacingValue,
    StyleIntent,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnresolvedStyleToken {
    Foreground(ColorToken),
    Background(ColorToken),
    Padding(SpacingToken),
    Radius(RadiusToken),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleResolution {
    computed_style: ComputedStyle,
    unresolved_tokens: Vec<UnresolvedStyleToken>,
}

impl StyleResolution {
    #[must_use]
    pub const fn new(
        computed_style: ComputedStyle,
        unresolved_tokens: Vec<UnresolvedStyleToken>,
    ) -> Self {
        Self {
            computed_style,
            unresolved_tokens,
        }
    }

    #[must_use]
    pub const fn computed_style(&self) -> ComputedStyle {
        self.computed_style
    }

    #[must_use]
    pub const fn unresolved_tokens(&self) -> &[UnresolvedStyleToken] {
        self.unresolved_tokens.as_slice()
    }

    #[must_use]
    pub const fn is_fully_resolved(&self) -> bool {
        self.unresolved_tokens.is_empty()
    }
}

#[must_use]
pub fn resolve_literal_style(intent: &StyleIntent) -> StyleResolution {
    let mut computed_style = ComputedStyle::EMPTY;
    let mut unresolved_tokens = Vec::new();

    match intent.foreground() {
        Some(ColorValue::Literal(color)) => computed_style = computed_style.with_foreground(*color),
        Some(ColorValue::Token(token)) => {
            unresolved_tokens.push(UnresolvedStyleToken::Foreground(token.clone()));
        }
        None => {}
    }

    match intent.background() {
        Some(ColorValue::Literal(color)) => computed_style = computed_style.with_background(*color),
        Some(ColorValue::Token(token)) => {
            unresolved_tokens.push(UnresolvedStyleToken::Background(token.clone()));
        }
        None => {}
    }

    match intent.padding() {
        Some(SpacingValue::Literal(padding)) => {
            computed_style = computed_style.with_padding(*padding);
        }
        Some(SpacingValue::Token(token)) => {
            unresolved_tokens.push(UnresolvedStyleToken::Padding(token.clone()));
        }
        None => {}
    }

    match intent.radius() {
        Some(RadiusValue::Literal(radius)) => computed_style = computed_style.with_radius(*radius),
        Some(RadiusValue::Token(token)) => {
            unresolved_tokens.push(UnresolvedStyleToken::Radius(token.clone()));
        }
        None => {}
    }

    StyleResolution::new(computed_style, unresolved_tokens)
}

#[cfg(test)]
mod tests {
    use super::{StyleResolution, UnresolvedStyleToken, resolve_literal_style};
    use crate::{
        Color, ColorToken, ColorValue, ComputedStyle, EdgeInsets, Length, Radius, RadiusToken,
        RadiusValue, SpacingToken, SpacingValue, StyleIntent,
    };

    #[test]
    fn empty_intent_resolves_to_empty_computed_style() {
        let resolution = resolve_literal_style(&StyleIntent::EMPTY);

        assert_eq!(resolution.computed_style(), ComputedStyle::EMPTY);
        assert!(resolution.is_fully_resolved());
        assert_eq!(resolution.unresolved_tokens(), []);
    }

    #[test]
    fn literal_intent_resolves_to_computed_style() {
        let padding = EdgeInsets::all(Length::px(8.0));
        let radius = Radius::all(Length::px(4.0));
        let intent = StyleIntent::EMPTY
            .with_foreground(Color::WHITE)
            .with_background(Color::BLACK)
            .with_padding(padding)
            .with_radius(radius);

        let resolution = resolve_literal_style(&intent);

        assert_eq!(
            resolution.computed_style(),
            ComputedStyle::EMPTY
                .with_foreground(Color::WHITE)
                .with_background(Color::BLACK)
                .with_padding(padding)
                .with_radius(radius)
        );
        assert!(resolution.is_fully_resolved());
        assert_eq!(resolution.unresolved_tokens(), []);
    }

    #[test]
    fn token_backed_intent_reports_unresolved_tokens() {
        let intent = StyleIntent::EMPTY
            .with_foreground(ColorValue::token("color.text.primary"))
            .with_background(ColorToken::new("color.surface"))
            .with_padding(SpacingToken::new("space.2"))
            .with_radius(RadiusValue::token("radius.control"));

        let resolution = resolve_literal_style(&intent);

        assert_eq!(resolution.computed_style(), ComputedStyle::EMPTY);
        assert!(!resolution.is_fully_resolved());
        assert_eq!(
            resolution.unresolved_tokens(),
            [
                UnresolvedStyleToken::Foreground(ColorToken::new("color.text.primary")),
                UnresolvedStyleToken::Background(ColorToken::new("color.surface")),
                UnresolvedStyleToken::Padding(SpacingToken::new("space.2")),
                UnresolvedStyleToken::Radius(RadiusToken::new("radius.control")),
            ]
        );
    }

    #[test]
    fn mixed_intent_resolves_literals_and_reports_tokens() {
        let padding = EdgeInsets::all(Length::px(12.0));
        let intent = StyleIntent::EMPTY
            .with_foreground(Color::WHITE)
            .with_background(ColorToken::new("color.surface"))
            .with_padding(SpacingValue::literal(padding))
            .with_radius(RadiusToken::new("radius.control"));

        let resolution = resolve_literal_style(&intent);

        assert_eq!(
            resolution.computed_style(),
            ComputedStyle::EMPTY
                .with_foreground(Color::WHITE)
                .with_padding(padding)
        );
        assert_eq!(
            resolution.unresolved_tokens(),
            [
                UnresolvedStyleToken::Background(ColorToken::new("color.surface")),
                UnresolvedStyleToken::Radius(RadiusToken::new("radius.control")),
            ]
        );
    }

    #[test]
    fn style_resolution_can_be_constructed_explicitly() {
        let resolution = StyleResolution::new(ComputedStyle::EMPTY, Vec::new());

        assert_eq!(resolution.computed_style(), ComputedStyle::EMPTY);
        assert!(resolution.is_fully_resolved());
    }
}
