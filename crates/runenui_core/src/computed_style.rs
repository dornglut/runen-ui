//! Resolved host-neutral style data.

use crate::{Color, EdgeInsets, Radius};

/// Concrete resolved visual style values for an element.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComputedStyle {
    foreground: Option<Color>,
    background: Option<Color>,
    padding: Option<EdgeInsets>,
    radius: Option<Radius>,
}

impl ComputedStyle {
    /// Empty computed style.
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
    pub const fn with_foreground(mut self, foreground: Color) -> Self {
        self.foreground = Some(foreground);
        self
    }

    #[must_use]
    pub const fn with_background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }

    #[must_use]
    pub const fn with_padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = Some(padding);
        self
    }

    #[must_use]
    pub const fn with_radius(mut self, radius: Radius) -> Self {
        self.radius = Some(radius);
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
}

#[cfg(test)]
mod tests {
    use super::ComputedStyle;
    use crate::{Color, EdgeInsets, Length, Radius};

    #[test]
    fn computed_style_defaults_to_empty() {
        let style = ComputedStyle::default();

        assert_eq!(style, ComputedStyle::EMPTY);
        assert!(style.is_empty());
        assert_eq!(style.foreground(), None);
        assert_eq!(style.background(), None);
        assert_eq!(style.padding(), None);
        assert_eq!(style.radius(), None);
    }

    #[test]
    fn computed_style_stores_resolved_values() {
        let padding = EdgeInsets::all(Length::px(8.0));
        let radius = Radius::all(Length::px(4.0));
        let style = ComputedStyle::EMPTY
            .with_foreground(Color::WHITE)
            .with_background(Color::BLACK)
            .with_padding(padding)
            .with_radius(radius);

        assert!(!style.is_empty());
        assert_eq!(style.foreground(), Some(Color::WHITE));
        assert_eq!(style.background(), Some(Color::BLACK));
        assert_eq!(style.padding(), Some(padding));
        assert_eq!(style.radius(), Some(radius));
    }
}
