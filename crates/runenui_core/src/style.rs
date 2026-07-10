//! Core host-neutral style value vocabulary.
//!
//! This module owns primitive style values only. It does not resolve themes,
//! recipes, selectors, renderer materials, or computed styles.

use crate::Px;

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

#[cfg(test)]
mod tests {
    use super::{Color, EdgeInsets, Length, Radius, Spacing};
    use crate::Px;

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
}
