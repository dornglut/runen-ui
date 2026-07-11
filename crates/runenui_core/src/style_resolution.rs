//! Pure style-resolution helpers.

use crate::{
    ColorToken, ColorValue, ComputedStyle, RadiusToken, RadiusValue, SpacingToken, SpacingValue,
    StyleIntent, StyleTokens,
};

/// Resolution provenance for one authored style field.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum StyleFieldProvenance<Token> {
    /// No value was authored for this field.
    #[default]
    Absent,
    /// The field was authored as a literal value.
    Literal,
    /// The field was authored as a token and the token resolved successfully.
    ResolvedToken(Token),
    /// The field was authored as a token, but the token was missing.
    MissingToken(Token),
}

/// Per-field provenance produced by style resolution.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StyleProvenance {
    foreground: StyleFieldProvenance<ColorToken>,
    background: StyleFieldProvenance<ColorToken>,
    padding: StyleFieldProvenance<SpacingToken>,
    radius: StyleFieldProvenance<RadiusToken>,
}

impl StyleProvenance {
    /// Provenance for an empty style intent.
    pub const EMPTY: Self = Self {
        foreground: StyleFieldProvenance::Absent,
        background: StyleFieldProvenance::Absent,
        padding: StyleFieldProvenance::Absent,
        radius: StyleFieldProvenance::Absent,
    };

    /// Creates a complete per-field provenance product.
    #[must_use]
    pub const fn new(
        foreground: StyleFieldProvenance<ColorToken>,
        background: StyleFieldProvenance<ColorToken>,
        padding: StyleFieldProvenance<SpacingToken>,
        radius: StyleFieldProvenance<RadiusToken>,
    ) -> Self {
        Self {
            foreground,
            background,
            padding,
            radius,
        }
    }

    /// Returns foreground resolution provenance.
    #[must_use]
    pub const fn foreground(&self) -> &StyleFieldProvenance<ColorToken> {
        &self.foreground
    }

    /// Returns background resolution provenance.
    #[must_use]
    pub const fn background(&self) -> &StyleFieldProvenance<ColorToken> {
        &self.background
    }

    /// Returns padding resolution provenance.
    #[must_use]
    pub const fn padding(&self) -> &StyleFieldProvenance<SpacingToken> {
        &self.padding
    }

    /// Returns corner-radius resolution provenance.
    #[must_use]
    pub const fn radius(&self) -> &StyleFieldProvenance<RadiusToken> {
        &self.radius
    }
}

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
    provenance: StyleProvenance,
    unresolved_tokens: Vec<UnresolvedStyleToken>,
}

impl StyleResolution {
    const fn new(
        computed_style: ComputedStyle,
        provenance: StyleProvenance,
        unresolved_tokens: Vec<UnresolvedStyleToken>,
    ) -> Self {
        Self {
            computed_style,
            provenance,
            unresolved_tokens,
        }
    }

    #[must_use]
    pub const fn computed_style(&self) -> ComputedStyle {
        self.computed_style
    }

    /// Returns per-field resolution provenance.
    #[must_use]
    pub const fn provenance(&self) -> &StyleProvenance {
        &self.provenance
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
    let mut provenance = StyleProvenance::EMPTY;
    let mut unresolved_tokens = Vec::new();

    match intent.foreground() {
        Some(ColorValue::Literal(color)) => {
            computed_style = computed_style.with_foreground(*color);
            provenance.foreground = StyleFieldProvenance::Literal;
        }
        Some(ColorValue::Token(token)) => {
            if let Some(color) = tokens.color(token) {
                computed_style = computed_style.with_foreground(color);
                provenance.foreground = StyleFieldProvenance::ResolvedToken(token.clone());
            } else {
                provenance.foreground = StyleFieldProvenance::MissingToken(token.clone());
                unresolved_tokens.push(UnresolvedStyleToken::Foreground(token.clone()));
            }
        }
        None => {}
    }

    match intent.background() {
        Some(ColorValue::Literal(color)) => {
            computed_style = computed_style.with_background(*color);
            provenance.background = StyleFieldProvenance::Literal;
        }
        Some(ColorValue::Token(token)) => {
            if let Some(color) = tokens.color(token) {
                computed_style = computed_style.with_background(color);
                provenance.background = StyleFieldProvenance::ResolvedToken(token.clone());
            } else {
                provenance.background = StyleFieldProvenance::MissingToken(token.clone());
                unresolved_tokens.push(UnresolvedStyleToken::Background(token.clone()));
            }
        }
        None => {}
    }

    match intent.padding() {
        Some(SpacingValue::Literal(padding)) => {
            computed_style = computed_style.with_padding(*padding);
            provenance.padding = StyleFieldProvenance::Literal;
        }
        Some(SpacingValue::Token(token)) => {
            if let Some(padding) = tokens.spacing(token) {
                computed_style = computed_style.with_padding(padding);
                provenance.padding = StyleFieldProvenance::ResolvedToken(token.clone());
            } else {
                provenance.padding = StyleFieldProvenance::MissingToken(token.clone());
                unresolved_tokens.push(UnresolvedStyleToken::Padding(token.clone()));
            }
        }
        None => {}
    }

    match intent.radius() {
        Some(RadiusValue::Literal(radius)) => {
            computed_style = computed_style.with_radius(*radius);
            provenance.radius = StyleFieldProvenance::Literal;
        }
        Some(RadiusValue::Token(token)) => {
            if let Some(radius) = tokens.radius(token) {
                computed_style = computed_style.with_radius(radius);
                provenance.radius = StyleFieldProvenance::ResolvedToken(token.clone());
            } else {
                provenance.radius = StyleFieldProvenance::MissingToken(token.clone());
                unresolved_tokens.push(UnresolvedStyleToken::Radius(token.clone()));
            }
        }
        None => {}
    }

    StyleResolution::new(computed_style, provenance, unresolved_tokens)
}
