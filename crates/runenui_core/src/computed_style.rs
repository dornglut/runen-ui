//! Runtime-resolved host-neutral style data.

use crate::{
    Brush, Color, DropShadow, EdgeInsets, Outline, PresentationTransform, Radius, SceneOpacity,
    Typography,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComputedStyle {
    foreground: Option<Color>,
    background: Option<Brush>,
    padding: Option<EdgeInsets>,
    radius: Option<Radius>,
    typography: Option<Typography>,
    outline: Option<Outline>,
    shadows: Vec<DropShadow>,
    opacity: SceneOpacity,
    presentation: Option<PresentationTransform>,
}

impl ComputedStyle {
    pub const EMPTY: Self = Self {
        foreground: None,
        background: None,
        padding: None,
        radius: None,
        typography: None,
        outline: None,
        shadows: Vec::new(),
        opacity: SceneOpacity::OPAQUE,
        presentation: None,
    };
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.foreground.is_none()
            && self.background.is_none()
            && self.padding.is_none()
            && self.radius.is_none()
            && self.typography.is_none()
            && self.outline.is_none()
            && self.shadows.is_empty()
            && self.opacity.get().to_bits() == SceneOpacity::OPAQUE.get().to_bits()
            && self.presentation.is_none()
    }
    #[must_use]
    pub const fn with_foreground(mut self, value: Color) -> Self {
        self.foreground = Some(value);
        self
    }
    #[must_use]
    pub fn with_background(mut self, value: impl Into<Brush>) -> Self {
        self.background = Some(value.into());
        self
    }
    #[must_use]
    pub const fn with_padding(mut self, value: EdgeInsets) -> Self {
        self.padding = Some(value);
        self
    }
    #[must_use]
    pub const fn with_radius(mut self, value: Radius) -> Self {
        self.radius = Some(value);
        self
    }
    #[must_use]
    pub fn with_typography(mut self, value: Typography) -> Self {
        self.typography = Some(value);
        self
    }
    #[must_use]
    pub fn with_outline(mut self, value: Outline) -> Self {
        self.outline = Some(value);
        self
    }
    #[must_use]
    pub fn with_shadows(mut self, value: Vec<DropShadow>) -> Self {
        self.shadows = value;
        self
    }
    #[must_use]
    pub const fn with_opacity(mut self, value: SceneOpacity) -> Self {
        self.opacity = value;
        self
    }
    #[must_use]
    pub const fn with_presentation(mut self, value: PresentationTransform) -> Self {
        self.presentation = Some(value);
        self
    }
    #[must_use]
    pub const fn foreground(&self) -> Option<Color> {
        self.foreground
    }
    #[must_use]
    pub const fn background(&self) -> Option<&Brush> {
        self.background.as_ref()
    }
    #[must_use]
    pub const fn padding(&self) -> Option<EdgeInsets> {
        self.padding
    }
    #[must_use]
    pub const fn radius(&self) -> Option<Radius> {
        self.radius
    }
    #[must_use]
    pub const fn typography(&self) -> Option<&Typography> {
        self.typography.as_ref()
    }
    #[must_use]
    pub const fn outline(&self) -> Option<&Outline> {
        self.outline.as_ref()
    }
    #[must_use]
    pub const fn shadows(&self) -> &[DropShadow] {
        self.shadows.as_slice()
    }
    #[must_use]
    pub const fn opacity(&self) -> SceneOpacity {
        self.opacity
    }
    #[must_use]
    pub const fn presentation(&self) -> Option<PresentationTransform> {
        self.presentation
    }
}
