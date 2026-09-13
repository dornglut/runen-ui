use crate::{Brush, StrokeStyle};

/// Optional paint-only node outline resolved through computed style.
///
/// The outline carries only neutral paint vocabulary. Runtime remains responsible
/// for selecting the node's authoritative outline geometry and composing it into
/// immutable publication; renderers may only realize the resolved brush/stroke.
#[derive(Clone, Debug, PartialEq)]
pub struct Outline {
    brush: Brush,
    style: StrokeStyle,
}

impl Outline {
    /// Creates one outline from its complete brush and centered stroke policy.
    #[must_use]
    pub const fn new(brush: Brush, style: StrokeStyle) -> Self {
        Self { brush, style }
    }

    /// Returns the complete outline brush without projecting it to a color.
    #[must_use]
    pub const fn brush(&self) -> &Brush {
        &self.brush
    }

    /// Returns the centered outline stroke policy.
    #[must_use]
    pub const fn style(&self) -> StrokeStyle {
        self.style
    }
}
