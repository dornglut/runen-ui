//! Typed authored values for common M9 node-visual style properties.

use crate::{
    DropShadow, OpacityToken, Outline, OutlineToken, PresentationToken, PresentationTransform,
    SceneOpacity, ShadowToken,
};

/// Literal-or-token authored value for the optional node outline.
#[derive(Clone, Debug, PartialEq)]
pub enum OutlineValue {
    Literal(Outline),
    Token(OutlineToken),
}

impl OutlineValue {
    #[must_use]
    pub const fn literal(value: Outline) -> Self {
        Self::Literal(value)
    }

    #[must_use]
    pub const fn token(token: OutlineToken) -> Self {
        Self::Token(token)
    }

    #[must_use]
    pub const fn as_literal(&self) -> Option<&Outline> {
        if let Self::Literal(value) = self {
            Some(value)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_token(&self) -> Option<&OutlineToken> {
        if let Self::Token(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl From<Outline> for OutlineValue {
    fn from(value: Outline) -> Self {
        Self::Literal(value)
    }
}

impl From<OutlineToken> for OutlineValue {
    fn from(value: OutlineToken) -> Self {
        Self::Token(value)
    }
}

/// Literal-or-token authored value for one complete ordered drop-shadow list.
///
/// A later cascade layer replaces the complete list rather than appending to a
/// lower-precedence list, preserving ordinary property-local style precedence.
#[derive(Clone, Debug, PartialEq)]
pub enum ShadowValue {
    Literal(Vec<DropShadow>),
    Token(ShadowToken),
}

impl ShadowValue {
    #[must_use]
    pub const fn literal(value: Vec<DropShadow>) -> Self {
        Self::Literal(value)
    }

    #[must_use]
    pub const fn token(token: ShadowToken) -> Self {
        Self::Token(token)
    }

    #[must_use]
    pub const fn as_literal(&self) -> Option<&[DropShadow]> {
        if let Self::Literal(value) = self {
            Some(value.as_slice())
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_token(&self) -> Option<&ShadowToken> {
        if let Self::Token(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl From<Vec<DropShadow>> for ShadowValue {
    fn from(value: Vec<DropShadow>) -> Self {
        Self::Literal(value)
    }
}

impl From<DropShadow> for ShadowValue {
    fn from(value: DropShadow) -> Self {
        Self::Literal(vec![value])
    }
}

impl From<ShadowToken> for ShadowValue {
    fn from(value: ShadowToken) -> Self {
        Self::Token(value)
    }
}

/// Literal-or-token authored value for effective node opacity.
#[derive(Clone, Debug, PartialEq)]
pub enum OpacityValue {
    Literal(SceneOpacity),
    Token(OpacityToken),
}

impl OpacityValue {
    #[must_use]
    pub const fn literal(value: SceneOpacity) -> Self {
        Self::Literal(value)
    }

    #[must_use]
    pub const fn token(token: OpacityToken) -> Self {
        Self::Token(token)
    }

    #[must_use]
    pub const fn as_literal(&self) -> Option<SceneOpacity> {
        if let Self::Literal(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_token(&self) -> Option<&OpacityToken> {
        if let Self::Token(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl From<SceneOpacity> for OpacityValue {
    fn from(value: SceneOpacity) -> Self {
        Self::Literal(value)
    }
}

impl From<OpacityToken> for OpacityValue {
    fn from(value: OpacityToken) -> Self {
        Self::Token(value)
    }
}

/// Literal-or-token authored value for the node-wide presentation transform.
#[derive(Clone, Debug, PartialEq)]
pub enum PresentationValue {
    Literal(PresentationTransform),
    Token(PresentationToken),
}

impl PresentationValue {
    #[must_use]
    pub const fn literal(value: PresentationTransform) -> Self {
        Self::Literal(value)
    }

    #[must_use]
    pub const fn token(token: PresentationToken) -> Self {
        Self::Token(token)
    }

    #[must_use]
    pub const fn as_literal(&self) -> Option<PresentationTransform> {
        if let Self::Literal(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_token(&self) -> Option<&PresentationToken> {
        if let Self::Token(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl From<PresentationTransform> for PresentationValue {
    fn from(value: PresentationTransform) -> Self {
        Self::Literal(value)
    }
}

impl From<PresentationToken> for PresentationValue {
    fn from(value: PresentationToken) -> Self {
        Self::Token(value)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VisualStyleProperties {
    outline: Option<OutlineValue>,
    shadows: Option<ShadowValue>,
    opacity: Option<OpacityValue>,
    presentation: Option<PresentationValue>,
}

impl VisualStyleProperties {
    pub const EMPTY: Self = Self {
        outline: None,
        shadows: None,
        opacity: None,
        presentation: None,
    };

    pub const fn is_empty(&self) -> bool {
        self.outline.is_none()
            && self.shadows.is_none()
            && self.opacity.is_none()
            && self.presentation.is_none()
    }

    pub fn with_outline(mut self, value: impl Into<OutlineValue>) -> Self {
        self.outline = Some(value.into());
        self
    }

    pub fn with_shadows(mut self, value: impl Into<ShadowValue>) -> Self {
        self.shadows = Some(value.into());
        self
    }

    pub fn with_opacity(mut self, value: impl Into<OpacityValue>) -> Self {
        self.opacity = Some(value.into());
        self
    }

    pub fn with_presentation(mut self, value: impl Into<PresentationValue>) -> Self {
        self.presentation = Some(value.into());
        self
    }

    pub const fn outline(&self) -> Option<&OutlineValue> {
        self.outline.as_ref()
    }

    pub const fn shadows(&self) -> Option<&ShadowValue> {
        self.shadows.as_ref()
    }

    pub const fn opacity(&self) -> Option<&OpacityValue> {
        self.opacity.as_ref()
    }

    pub const fn presentation(&self) -> Option<&PresentationValue> {
        self.presentation.as_ref()
    }
}
