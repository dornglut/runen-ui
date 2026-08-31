//! Runtime-resolved host-neutral style data.

use crate::{Color, EdgeInsets, Radius, Typography};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComputedStyle {
    foreground: Option<Color>,
    background: Option<Color>,
    padding: Option<EdgeInsets>,
    radius: Option<Radius>,
    typography: Option<Typography>,
}

impl ComputedStyle {
    pub const EMPTY: Self = Self {
        foreground: None,
        background: None,
        padding: None,
        radius: None,
        typography: None,
    };
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.foreground.is_none()
            && self.background.is_none()
            && self.padding.is_none()
            && self.radius.is_none()
            && self.typography.is_none()
    }
    #[must_use]
    pub const fn with_foreground(mut self, value: Color) -> Self {
        self.foreground = Some(value);
        self
    }
    #[must_use]
    pub const fn with_background(mut self, value: Color) -> Self {
        self.background = Some(value);
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
    pub const fn foreground(&self) -> Option<Color> {
        self.foreground
    }
    #[must_use]
    pub const fn background(&self) -> Option<Color> {
        self.background
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

    pub(crate) fn from_parts(
        foreground: Option<Color>,
        background: Option<Color>,
        padding: Option<EdgeInsets>,
        radius: Option<Radius>,
        typography: Option<Typography>,
    ) -> Self {
        Self {
            foreground,
            background,
            padding,
            radius,
            typography,
        }
    }
}
