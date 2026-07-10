//! Core host-neutral style value vocabulary.
//!
//! This module owns primitive style values, token references, token-backed value
//! unions, and authored style intent only. It does not resolve themes, recipes,
//! selectors, renderer materials, or computed styles.

use crate::Px;

/// Stable identifier for a named design token.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TokenId(String);

impl TokenId {
    /// Creates a token identifier.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the token identifier string.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for TokenId {
    fn from(id: &str) -> Self {
        Self::new(id)
    }
}

impl From<String> for TokenId {
    fn from(id: String) -> Self {
        Self::new(id)
    }
}

macro_rules! define_token_ref {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(TokenId);

        impl $name {
            /// Creates a typed token reference.
            #[must_use]
            pub fn new(id: impl Into<TokenId>) -> Self {
                Self(id.into())
            }

            /// Returns the untyped token identifier.
            #[must_use]
            pub const fn id(&self) -> &TokenId {
                &self.0
            }

            /// Returns the token identifier string.
            #[must_use]
            pub const fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl From<TokenId> for $name {
            fn from(id: TokenId) -> Self {
                Self::new(id)
            }
        }

        impl From<&str> for $name {
            fn from(id: &str) -> Self {
                Self::new(id)
            }
        }

        impl From<String> for $name {
            fn from(id: String) -> Self {
                Self::new(id)
            }
        }
    };
}

define_token_ref!(ColorToken, "Typed reference to a color design token.");
define_token_ref!(LengthToken, "Typed reference to a length design token.");
define_token_ref!(SpacingToken, "Typed reference to a spacing design token.");
define_token_ref!(RadiusToken, "Typed reference to a radius design token.");

/// Host-neutral sRGB color with straight alpha.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl Color {
    /// Fully transparent black.
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    /// Opaque black.
    pub const BLACK: Self = Self::rgb(0, 0, 0);

    /// Opaque white.
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    /// Creates an opaque sRGB color.
    #[must_use]
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, 255)
    }

    /// Creates an sRGB color with straight alpha.
    #[must_use]
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Returns the red channel.
    #[must_use]
    pub const fn red(self) -> u8 {
        self.red
    }

    /// Returns the green channel.
    #[must_use]
    pub const fn green(self) -> u8 {
        self.green
    }

    /// Returns the blue channel.
    #[must_use]
    pub const fn blue(self) -> u8 {
        self.blue
    }

    /// Returns the alpha channel.
    #[must_use]
    pub const fn alpha(self) -> u8 {
        self.alpha
    }
}

/// Color style value that may be literal or token-backed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ColorValue {
    Literal(Color),
    Token(ColorToken),
}

impl ColorValue {
    /// Creates a literal color value.
    #[must_use]
    pub const fn literal(value: Color) -> Self {
        Self::Literal(value)
    }

    /// Creates a token-backed color value.
    #[must_use]
    pub fn token(token: impl Into<ColorToken>) -> Self {
        Self::Token(token.into())
    }

    /// Returns the literal color value, if this value is literal.
    #[must_use]
    pub const fn as_literal(&self) -> Option<&Color> {
        match self {
            Self::Literal(value) => Some(value),
            Self::Token(_) => None,
        }
    }

    /// Returns the color token reference, if this value is token-backed.
    #[must_use]
    pub const fn as_token(&self) -> Option<&ColorToken> {
        match self {
            Self::Literal(_) => None,
            Self::Token(token) => Some(token),
        }
    }
}

impl From<Color> for ColorValue {
    fn from(value: Color) -> Self {
        Self::literal(value)
    }
}

impl From<ColorToken> for ColorValue {
    fn from(token: ColorToken) -> Self {
        Self::Token(token)
    }
}

/// Host-neutral logical length used by visual style values.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Length(f32);

impl Length {
    /// Zero logical pixels.
    pub const ZERO: Self = Self(0.0);

    /// Creates a length in logical pixels.
    #[must_use]
    pub const fn px(value: f32) -> Self {
        Self(value)
    }

    /// Returns the logical pixel value.
    #[must_use]
    pub const fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for Length {
    fn from(value: f32) -> Self {
        Self::px(value)
    }
}

impl From<u16> for Length {
    fn from(value: u16) -> Self {
        Self::px(f32::from(value))
    }
}

impl From<Px> for Length {
    fn from(value: Px) -> Self {
        Self::px(value.value())
    }
}

impl From<Length> for Px {
    fn from(value: Length) -> Self {
        Self::new(value.value())
    }
}

/// Length style value that may be literal or token-backed.
#[derive(Clone, Debug, PartialEq)]
pub enum LengthValue {
    Literal(Length),
    Token(LengthToken),
}

impl LengthValue {
    /// Creates a literal length value.
    #[must_use]
    pub const fn literal(value: Length) -> Self {
        Self::Literal(value)
    }

    /// Creates a token-backed length value.
    #[must_use]
    pub fn token(token: impl Into<LengthToken>) -> Self {
        Self::Token(token.into())
    }

    /// Returns the literal length value, if this value is literal.
    #[must_use]
    pub const fn as_literal(&self) -> Option<&Length> {
        match self {
            Self::Literal(value) => Some(value),
            Self::Token(_) => None,
        }
    }

    /// Returns the length token reference, if this value is token-backed.
    #[must_use]
    pub const fn as_token(&self) -> Option<&LengthToken> {
        match self {
            Self::Literal(_) => None,
            Self::Token(token) => Some(token),
        }
    }
}

impl From<Length> for LengthValue {
    fn from(value: Length) -> Self {
        Self::literal(value)
    }
}

impl From<LengthToken> for LengthValue {
    fn from(token: LengthToken) -> Self {
        Self::Token(token)
    }
}

/// Edge spacing used by padding-like and margin-like style values.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EdgeInsets {
    top: Length,
    right: Length,
    bottom: Length,
    left: Length,
}

impl EdgeInsets {
    /// Zero spacing on every edge.
    pub const ZERO: Self = Self::all(Length::ZERO);

    /// Creates edge spacing from top, right, bottom, and left values.
    #[must_use]
    pub const fn new(top: Length, right: Length, bottom: Length, left: Length) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Creates equal spacing on every edge.
    #[must_use]
    pub const fn all(value: Length) -> Self {
        Self::new(value, value, value, value)
    }

    /// Creates spacing from horizontal and vertical values.
    #[must_use]
    pub const fn symmetric(horizontal: Length, vertical: Length) -> Self {
        Self::new(vertical, horizontal, vertical, horizontal)
    }

    /// Returns the top edge spacing.
    #[must_use]
    pub const fn top(self) -> Length {
        self.top
    }

    /// Returns the right edge spacing.
    #[must_use]
    pub const fn right(self) -> Length {
        self.right
    }

    /// Returns the bottom edge spacing.
    #[must_use]
    pub const fn bottom(self) -> Length {
        self.bottom
    }

    /// Returns the left edge spacing.
    #[must_use]
    pub const fn left(self) -> Length {
        self.left
    }
}

impl From<Length> for EdgeInsets {
    fn from(value: Length) -> Self {
        Self::all(value)
    }
}

/// Alias used when a style API describes spacing rather than physical edge insets.
pub type Spacing = EdgeInsets;

/// Spacing style value that may be literal or token-backed.
#[derive(Clone, Debug, PartialEq)]
pub enum SpacingValue {
    Literal(EdgeInsets),
    Token(SpacingToken),
}

impl SpacingValue {
    /// Creates a literal spacing value.
    #[must_use]
    pub const fn literal(value: EdgeInsets) -> Self {
        Self::Literal(value)
    }

    /// Creates a token-backed spacing value.
    #[must_use]
    pub fn token(token: impl Into<SpacingToken>) -> Self {
        Self::Token(token.into())
    }

    /// Returns the literal spacing value, if this value is literal.
    #[must_use]
    pub const fn as_literal(&self) -> Option<&EdgeInsets> {
        match self {
            Self::Literal(value) => Some(value),
            Self::Token(_) => None,
        }
    }

    /// Returns the spacing token reference, if this value is token-backed.
    #[must_use]
    pub const fn as_token(&self) -> Option<&SpacingToken> {
        match self {
            Self::Literal(_) => None,
            Self::Token(token) => Some(token),
        }
    }
}

impl From<EdgeInsets> for SpacingValue {
    fn from(value: EdgeInsets) -> Self {
        Self::literal(value)
    }
}

impl From<SpacingToken> for SpacingValue {
    fn from(token: SpacingToken) -> Self {
        Self::Token(token)
    }
}

/// Corner radius style value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Radius {
    top_left: Length,
    top_right: Length,
    bottom_right: Length,
    bottom_left: Length,
}

impl Radius {
    /// Zero radius on every corner.
    pub const ZERO: Self = Self::all(Length::ZERO);

    /// Creates a radius from top-left, top-right, bottom-right, and bottom-left values.
    #[must_use]
    pub const fn new(
        top_left: Length,
        top_right: Length,
        bottom_right: Length,
        bottom_left: Length,
    ) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    /// Creates equal radius on every corner.
    #[must_use]
    pub const fn all(value: Length) -> Self {
        Self::new(value, value, value, value)
    }

    /// Returns the top-left corner radius.
    #[must_use]
    pub const fn top_left(self) -> Length {
        self.top_left
    }

    /// Returns the top-right corner radius.
    #[must_use]
    pub const fn top_right(self) -> Length {
        self.top_right
    }

    /// Returns the bottom-right corner radius.
    #[must_use]
    pub const fn bottom_right(self) -> Length {
        self.bottom_right
    }

    /// Returns the bottom-left corner radius.
    #[must_use]
    pub const fn bottom_left(self) -> Length {
        self.bottom_left
    }
}

impl From<Length> for Radius {
    fn from(value: Length) -> Self {
        Self::all(value)
    }
}

/// Radius style value that may be literal or token-backed.
#[derive(Clone, Debug, PartialEq)]
pub enum RadiusValue {
    Literal(Radius),
    Token(RadiusToken),
}

impl RadiusValue {
    /// Creates a literal radius value.
    #[must_use]
    pub const fn literal(value: Radius) -> Self {
        Self::Literal(value)
    }

    /// Creates a token-backed radius value.
    #[must_use]
    pub fn token(token: impl Into<RadiusToken>) -> Self {
        Self::Token(token.into())
    }

    /// Returns the literal radius value, if this value is literal.
    #[must_use]
    pub const fn as_literal(&self) -> Option<&Radius> {
        match self {
            Self::Literal(value) => Some(value),
            Self::Token(_) => None,
        }
    }

    /// Returns the radius token reference, if this value is token-backed.
    #[must_use]
    pub const fn as_token(&self) -> Option<&RadiusToken> {
        match self {
            Self::Literal(_) => None,
            Self::Token(token) => Some(token),
        }
    }
}

impl From<Radius> for RadiusValue {
    fn from(value: Radius) -> Self {
        Self::literal(value)
    }
}

impl From<RadiusToken> for RadiusValue {
    fn from(token: RadiusToken) -> Self {
        Self::Token(token)
    }
}

/// Authored local visual style intent attached to an element.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StyleIntent {
    foreground: Option<Color>,
    background: Option<Color>,
    padding: Option<EdgeInsets>,
    radius: Option<Radius>,
}

impl StyleIntent {
    /// Empty style intent.
    pub const EMPTY: Self = Self {
        foreground: None,
        background: None,
        padding: None,
        radius: None,
    };

    /// Returns whether this style intent contains no local visual values.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.foreground.is_none()
            && self.background.is_none()
            && self.padding.is_none()
            && self.radius.is_none()
    }

    /// Sets local foreground color intent.
    #[must_use]
    pub const fn with_foreground(mut self, foreground: Color) -> Self {
        self.foreground = Some(foreground);
        self
    }

    /// Sets local background color intent.
    #[must_use]
    pub const fn with_background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }

    /// Sets local padding intent.
    #[must_use]
    pub fn with_padding(mut self, padding: impl Into<EdgeInsets>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Sets local corner radius intent.
    #[must_use]
    pub fn with_radius(mut self, radius: impl Into<Radius>) -> Self {
        self.radius = Some(radius.into());
        self
    }

    /// Returns local foreground color intent, if present.
    #[must_use]
    pub const fn foreground(self) -> Option<Color> {
        self.foreground
    }

    /// Returns local background color intent, if present.
    #[must_use]
    pub const fn background(self) -> Option<Color> {
        self.background
    }

    /// Returns local padding intent, if present.
    #[must_use]
    pub const fn padding(self) -> Option<EdgeInsets> {
        self.padding
    }

    /// Returns local corner radius intent, if present.
    #[must_use]
    pub const fn radius(self) -> Option<Radius> {
        self.radius
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Color, ColorToken, ColorValue, EdgeInsets, Length, LengthToken, LengthValue, Radius,
        RadiusToken, RadiusValue, Spacing, SpacingToken, SpacingValue, StyleIntent, TokenId,
    };
    use crate::Px;

    #[test]
    fn token_id_preserves_identifier_text() {
        let id = TokenId::new("color.text.primary");

        assert_eq!(id.as_str(), "color.text.primary");
        assert_eq!(TokenId::from("color.text.primary"), id);
    }

    #[test]
    fn typed_token_references_preserve_token_ids() {
        let color = ColorToken::new("color.text.primary");
        let length = LengthToken::new("length.control.height");
        let spacing = SpacingToken::new("space.2");
        let radius = RadiusToken::new("radius.control");

        assert_eq!(color.as_str(), "color.text.primary");
        assert_eq!(length.as_str(), "length.control.height");
        assert_eq!(spacing.as_str(), "space.2");
        assert_eq!(radius.as_str(), "radius.control");
        assert_eq!(color.id(), &TokenId::new("color.text.primary"));
    }

    #[test]
    fn typed_tokens_do_not_compare_across_token_families() {
        let color = ColorToken::new("color.surface");
        let other_color = ColorToken::new("color.surface");
        let spacing = SpacingToken::new("color.surface");

        assert_eq!(color, other_color);
        assert_eq!(spacing.as_str(), color.as_str());
    }

    #[test]
    fn color_value_preserves_literal_and_token_forms() {
        let literal = ColorValue::literal(Color::WHITE);
        let token = ColorValue::token("color.text.primary");

        assert_eq!(literal.as_literal(), Some(&Color::WHITE));
        assert_eq!(literal.as_token(), None);
        assert_eq!(token.as_literal(), None);
        assert_eq!(
            token.as_token(),
            Some(&ColorToken::new("color.text.primary"))
        );
    }

    #[test]
    fn length_value_preserves_literal_and_token_forms() {
        let literal = LengthValue::literal(Length::px(24.0));
        let token = LengthValue::token("length.control.height");

        assert_eq!(literal.as_literal(), Some(&Length::px(24.0)));
        assert_eq!(literal.as_token(), None);
        assert_eq!(token.as_literal(), None);
        assert_eq!(
            token.as_token(),
            Some(&LengthToken::new("length.control.height"))
        );
    }

    #[test]
    fn spacing_value_preserves_literal_and_token_forms() {
        let spacing = EdgeInsets::all(Length::px(8.0));
        let literal = SpacingValue::literal(spacing);
        let token = SpacingValue::token("space.2");

        assert_eq!(literal.as_literal(), Some(&spacing));
        assert_eq!(literal.as_token(), None);
        assert_eq!(token.as_literal(), None);
        assert_eq!(token.as_token(), Some(&SpacingToken::new("space.2")));
    }

    #[test]
    fn radius_value_preserves_literal_and_token_forms() {
        let radius = Radius::all(Length::px(4.0));
        let literal = RadiusValue::literal(radius);
        let token = RadiusValue::token("radius.control");

        assert_eq!(literal.as_literal(), Some(&radius));
        assert_eq!(literal.as_token(), None);
        assert_eq!(token.as_literal(), None);
        assert_eq!(token.as_token(), Some(&RadiusToken::new("radius.control")));
    }

    #[test]
    fn color_builders_preserve_channels() {
        let opaque = Color::rgb(10, 20, 30);
        assert_eq!(opaque.red(), 10);
        assert_eq!(opaque.green(), 20);
        assert_eq!(opaque.blue(), 30);
        assert_eq!(opaque.alpha(), 255);

        let translucent = Color::rgba(10, 20, 30, 40);
        assert_eq!(translucent.alpha(), 40);
    }

    #[test]
    fn length_converts_from_existing_px_type() {
        let length = Length::from(Px::new(12.0));
        assert_eq!(length, Length::px(12.0));
        assert_eq!(Px::from(length), Px::new(12.0));
    }

    #[test]
    fn edge_insets_support_uniform_and_axis_values() {
        let uniform = EdgeInsets::all(Length::px(8.0));
        assert_eq!(uniform.top(), Length::px(8.0));
        assert_eq!(uniform.right(), Length::px(8.0));
        assert_eq!(uniform.bottom(), Length::px(8.0));
        assert_eq!(uniform.left(), Length::px(8.0));

        let axis = EdgeInsets::symmetric(Length::px(4.0), Length::px(12.0));
        assert_eq!(axis.top(), Length::px(12.0));
        assert_eq!(axis.right(), Length::px(4.0));
        assert_eq!(axis.bottom(), Length::px(12.0));
        assert_eq!(axis.left(), Length::px(4.0));
    }

    #[test]
    fn spacing_alias_uses_edge_insets_model() {
        let spacing: Spacing = EdgeInsets::all(Length::px(6.0));
        assert_eq!(spacing.left(), Length::px(6.0));
    }

    #[test]
    fn radius_supports_uniform_and_per_corner_values() {
        let uniform = Radius::all(Length::px(3.0));
        assert_eq!(uniform.top_left(), Length::px(3.0));
        assert_eq!(uniform.top_right(), Length::px(3.0));
        assert_eq!(uniform.bottom_right(), Length::px(3.0));
        assert_eq!(uniform.bottom_left(), Length::px(3.0));

        let corners = Radius::new(
            Length::px(1.0),
            Length::px(2.0),
            Length::px(3.0),
            Length::px(4.0),
        );
        assert_eq!(corners.top_left(), Length::px(1.0));
        assert_eq!(corners.top_right(), Length::px(2.0));
        assert_eq!(corners.bottom_right(), Length::px(3.0));
        assert_eq!(corners.bottom_left(), Length::px(4.0));
    }

    #[test]
    fn style_intent_stores_unresolved_local_visual_values() {
        let intent = StyleIntent::EMPTY
            .with_foreground(Color::WHITE)
            .with_background(Color::BLACK)
            .with_padding(EdgeInsets::all(Length::px(8.0)))
            .with_radius(Radius::all(Length::px(4.0)));

        assert!(!intent.is_empty());
        assert_eq!(intent.foreground(), Some(Color::WHITE));
        assert_eq!(intent.background(), Some(Color::BLACK));
        assert_eq!(intent.padding(), Some(EdgeInsets::all(Length::px(8.0))));
        assert_eq!(intent.radius(), Some(Radius::all(Length::px(4.0))));
    }
}
