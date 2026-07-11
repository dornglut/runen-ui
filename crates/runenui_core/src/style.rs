//! Validated host-neutral authored style vocabulary.

use crate::{IdentifierError, LogicalLength, identity::validate_identifier};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TokenId(TokenText);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum TokenText {
    Static(&'static str),
    Owned(Box<str>),
}

impl TokenId {
    /// Validates a dynamic token identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierError`] when the identifier text is invalid.
    pub fn new(id: impl Into<String>) -> Result<Self, IdentifierError> {
        let id = id.into();
        validate_identifier(&id)?;
        Ok(Self(TokenText::Owned(id.into_boxed_str())))
    }

    /// Validates a static token identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierError`] when the identifier text is invalid.
    pub const fn from_static(id: &'static str) -> Result<Self, IdentifierError> {
        match validate_identifier(id) {
            Ok(()) => Ok(Self(TokenText::Static(id))),
            Err(error) => Err(error),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.0 {
            TokenText::Static(value) => value,
            TokenText::Owned(value) => value,
        }
    }
}

macro_rules! define_token_ref {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(TokenId);

        impl $name {
            #[must_use]
            pub const fn new(id: TokenId) -> Self {
                Self(id)
            }
            /// Validates a dynamic typed token reference.
            ///
            /// # Errors
            ///
            /// Returns [`IdentifierError`] when the identifier text is invalid.
            pub fn parse(id: impl Into<String>) -> Result<Self, IdentifierError> {
                TokenId::new(id).map(Self)
            }
            #[must_use]
            pub const fn id(&self) -> &TokenId {
                &self.0
            }
            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl From<TokenId> for $name {
            fn from(id: TokenId) -> Self {
                Self(id)
            }
        }
    };
}

define_token_ref!(ColorToken, "Typed color-token reference.");
define_token_ref!(SpacingToken, "Typed edge-spacing-token reference.");
define_token_ref!(RadiusToken, "Typed corner-radius-token reference.");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    #[must_use]
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, 255)
    }
    #[must_use]
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
    #[must_use]
    pub const fn red(self) -> u8 {
        self.red
    }
    #[must_use]
    pub const fn green(self) -> u8 {
        self.green
    }
    #[must_use]
    pub const fn blue(self) -> u8 {
        self.blue
    }
    #[must_use]
    pub const fn alpha(self) -> u8 {
        self.alpha
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ColorValue {
    Literal(Color),
    Token(ColorToken),
}

impl ColorValue {
    #[must_use]
    pub const fn literal(value: Color) -> Self {
        Self::Literal(value)
    }
    #[must_use]
    pub const fn token(token: ColorToken) -> Self {
        Self::Token(token)
    }
    #[must_use]
    pub const fn as_literal(&self) -> Option<&Color> {
        if let Self::Literal(value) = self {
            Some(value)
        } else {
            None
        }
    }
    #[must_use]
    pub const fn as_token(&self) -> Option<&ColorToken> {
        if let Self::Token(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl From<Color> for ColorValue {
    fn from(value: Color) -> Self {
        Self::Literal(value)
    }
}
impl From<ColorToken> for ColorValue {
    fn from(value: ColorToken) -> Self {
        Self::Token(value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EdgeInsets {
    top: LogicalLength,
    right: LogicalLength,
    bottom: LogicalLength,
    left: LogicalLength,
}

impl EdgeInsets {
    pub const ZERO: Self = Self::all(LogicalLength::ZERO);
    #[must_use]
    pub const fn new(
        top: LogicalLength,
        right: LogicalLength,
        bottom: LogicalLength,
        left: LogicalLength,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
    #[must_use]
    pub const fn all(value: LogicalLength) -> Self {
        Self::new(value, value, value, value)
    }
    #[must_use]
    pub const fn symmetric(horizontal: LogicalLength, vertical: LogicalLength) -> Self {
        Self::new(vertical, horizontal, vertical, horizontal)
    }
    #[must_use]
    pub const fn top(self) -> LogicalLength {
        self.top
    }
    #[must_use]
    pub const fn right(self) -> LogicalLength {
        self.right
    }
    #[must_use]
    pub const fn bottom(self) -> LogicalLength {
        self.bottom
    }
    #[must_use]
    pub const fn left(self) -> LogicalLength {
        self.left
    }
}

impl From<LogicalLength> for EdgeInsets {
    fn from(value: LogicalLength) -> Self {
        Self::all(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpacingValue {
    Literal(EdgeInsets),
    Token(SpacingToken),
}

impl SpacingValue {
    #[must_use]
    pub const fn literal(value: EdgeInsets) -> Self {
        Self::Literal(value)
    }
    #[must_use]
    pub const fn token(token: SpacingToken) -> Self {
        Self::Token(token)
    }
    #[must_use]
    pub const fn as_literal(&self) -> Option<&EdgeInsets> {
        if let Self::Literal(value) = self {
            Some(value)
        } else {
            None
        }
    }
    #[must_use]
    pub const fn as_token(&self) -> Option<&SpacingToken> {
        if let Self::Token(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl From<EdgeInsets> for SpacingValue {
    fn from(value: EdgeInsets) -> Self {
        Self::Literal(value)
    }
}
impl From<SpacingToken> for SpacingValue {
    fn from(value: SpacingToken) -> Self {
        Self::Token(value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Radius {
    top_left: LogicalLength,
    top_right: LogicalLength,
    bottom_right: LogicalLength,
    bottom_left: LogicalLength,
}

impl Radius {
    pub const ZERO: Self = Self::all(LogicalLength::ZERO);
    #[must_use]
    pub const fn new(
        top_left: LogicalLength,
        top_right: LogicalLength,
        bottom_right: LogicalLength,
        bottom_left: LogicalLength,
    ) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }
    #[must_use]
    pub const fn all(value: LogicalLength) -> Self {
        Self::new(value, value, value, value)
    }
    #[must_use]
    pub const fn top_left(self) -> LogicalLength {
        self.top_left
    }
    #[must_use]
    pub const fn top_right(self) -> LogicalLength {
        self.top_right
    }
    #[must_use]
    pub const fn bottom_right(self) -> LogicalLength {
        self.bottom_right
    }
    #[must_use]
    pub const fn bottom_left(self) -> LogicalLength {
        self.bottom_left
    }
}

impl From<LogicalLength> for Radius {
    fn from(value: LogicalLength) -> Self {
        Self::all(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RadiusValue {
    Literal(Radius),
    Token(RadiusToken),
}

impl RadiusValue {
    #[must_use]
    pub const fn literal(value: Radius) -> Self {
        Self::Literal(value)
    }
    #[must_use]
    pub const fn token(token: RadiusToken) -> Self {
        Self::Token(token)
    }
    #[must_use]
    pub const fn as_literal(&self) -> Option<&Radius> {
        if let Self::Literal(value) = self {
            Some(value)
        } else {
            None
        }
    }
    #[must_use]
    pub const fn as_token(&self) -> Option<&RadiusToken> {
        if let Self::Token(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl From<Radius> for RadiusValue {
    fn from(value: Radius) -> Self {
        Self::Literal(value)
    }
}
impl From<RadiusToken> for RadiusValue {
    fn from(value: RadiusToken) -> Self {
        Self::Token(value)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleIntent {
    foreground: Option<ColorValue>,
    background: Option<ColorValue>,
    padding: Option<SpacingValue>,
    radius: Option<RadiusValue>,
}

impl StyleIntent {
    pub const EMPTY: Self = Self {
        foreground: None,
        background: None,
        padding: None,
        radius: None,
    };
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.foreground.is_none()
            && self.background.is_none()
            && self.padding.is_none()
            && self.radius.is_none()
    }
    #[must_use]
    pub fn with_foreground(mut self, value: impl Into<ColorValue>) -> Self {
        self.foreground = Some(value.into());
        self
    }
    #[must_use]
    pub fn with_background(mut self, value: impl Into<ColorValue>) -> Self {
        self.background = Some(value.into());
        self
    }
    #[must_use]
    pub fn with_padding(mut self, value: impl Into<SpacingValue>) -> Self {
        self.padding = Some(value.into());
        self
    }
    #[must_use]
    pub fn with_radius(mut self, value: impl Into<RadiusValue>) -> Self {
        self.radius = Some(value.into());
        self
    }
    #[must_use]
    pub const fn foreground(&self) -> Option<&ColorValue> {
        self.foreground.as_ref()
    }
    #[must_use]
    pub const fn background(&self) -> Option<&ColorValue> {
        self.background.as_ref()
    }
    #[must_use]
    pub const fn padding(&self) -> Option<&SpacingValue> {
        self.padding.as_ref()
    }
    #[must_use]
    pub const fn radius(&self) -> Option<&RadiusValue> {
        self.radius.as_ref()
    }
}
