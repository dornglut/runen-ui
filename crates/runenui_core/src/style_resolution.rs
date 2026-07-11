//! Pure style-resolution helpers.

use crate::{
    ColorToken, ColorValue, ComputedStyle, RadiusToken, RadiusValue, SpacingToken, SpacingValue,
    StyleIntent, StyleTokens,
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
    let tokens = StyleTokens::new();
    resolve_style(intent, &tokens)
}

#[must_use]
pub fn resolve_style(intent: &StyleIntent, tokens: &StyleTokens) -> StyleResolution {
    let mut computed_style = ComputedStyle::EMPTY;
    let mut unresolved_tokens = Vec::new();

    match intent.foreground() {
        Some(ColorValue::Literal(color)) => {
            computed_style = computed_style.with_foreground(*color);
        }
        Some(ColorValue::Token(token)) => match tokens.color(token) {
            Some(color) => {
                computed_style = computed_style.with_foreground(color);
            }
            None => unresolved_tokens.push(UnresolvedStyleToken::Foreground(token.clone())),
        },
        None => {}
    }

    match intent.background() {
        Some(ColorValue::Literal(color)) => {
            computed_style = computed_style.with_background(*color);
        }
        Some(ColorValue::Token(token)) => match tokens.color(token) {
            Some(color) => {
                computed_style = computed_style.with_background(color);
            }
            None => unresolved_tokens.push(UnresolvedStyleToken::Background(token.clone())),
        },
        None => {}
    }

    match intent.padding() {
        Some(SpacingValue::Literal(padding)) => {
            computed_style = computed_style.with_padding(*padding);
        }
        Some(SpacingValue::Token(token)) => match tokens.spacing(token) {
            Some(padding) => {
                computed_style = computed_style.with_padding(padding);
            }
            None => unresolved_tokens.push(UnresolvedStyleToken::Padding(token.clone())),
        },
        None => {}
    }

    match intent.radius() {
        Some(RadiusValue::Literal(radius)) => {
            computed_style = computed_style.with_radius(*radius);
        }
        Some(RadiusValue::Token(token)) => match tokens.radius(token) {
            Some(radius) => {
                computed_style = computed_style.with_radius(radius);
            }
            None => unresolved_tokens.push(UnresolvedStyleToken::Radius(token.clone())),
        },
        None => {}
    }

    StyleResolution::new(computed_style, unresolved_tokens)
}
