//! Renderer-neutral owner-local paint contribution vocabulary.

use crate::{Color, ComputedStyle, LogicalLength, LogicalRect, LogicalSize};

/// Read-only facts supplied while one mounted widget contributes paint.
///
/// The context deliberately contains no mounted identity, surface origin,
/// raster scale, renderer/backend object, resource provider, semantic data, or
/// publication history.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintContributionContext {
    local_size: LogicalSize,
    computed_style: ComputedStyle,
}

impl PaintContributionContext {
    /// Returns the owner's final local logical size.
    #[must_use]
    pub const fn local_size(self) -> LogicalSize {
        self.local_size
    }

    /// Returns the owner's resolved style facts.
    #[must_use]
    pub const fn computed_style(self) -> ComputedStyle {
        self.computed_style
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(local_size: LogicalSize, computed_style: ComputedStyle) -> Self {
        Self {
            local_size,
            computed_style,
        }
    }
}

/// Ordered immutable paint fragment authored in one widget's local logical space.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintContribution {
    items: Vec<PaintContributionItem>,
}

impl PaintContribution {
    /// Empty contribution.
    #[must_use]
    pub const fn empty() -> Self {
        Self { items: Vec::new() }
    }

    /// Creates one contribution from already validated items in local order.
    #[must_use]
    pub const fn new(items: Vec<PaintContributionItem>) -> Self {
        Self { items }
    }

    /// Creates a one-item contribution.
    #[must_use]
    pub fn single(item: PaintContributionItem) -> Self {
        Self { items: vec![item] }
    }

    /// Returns contribution items in exact authored order.
    #[must_use]
    pub const fn items(&self) -> &[PaintContributionItem] {
        self.items.as_slice()
    }

    /// Returns whether this widget contributes no paint.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// One owner-local renderer-neutral paint item.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintContributionItem {
    primitive: PaintPrimitive,
}

impl PaintContributionItem {
    /// Creates a filled logical rectangle using one literal core color.
    #[must_use]
    pub const fn fill_rect(rect: LogicalRect, color: Color) -> Self {
        Self {
            primitive: PaintPrimitive::FillRect { rect, color },
        }
    }

    /// Creates a centered logical rectangle stroke.
    ///
    /// [`LogicalLength`] guarantees a finite non-negative width. Width zero is
    /// retained literally and means no stroke coverage; it is never a backend
    /// hairline request.
    #[must_use]
    pub const fn stroke_rect(rect: LogicalRect, color: Color, width: LogicalLength) -> Self {
        Self {
            primitive: PaintPrimitive::StrokeRect { rect, color, width },
        }
    }

    /// Returns the renderer-neutral primitive.
    #[must_use]
    pub const fn primitive(&self) -> &PaintPrimitive {
        &self.primitive
    }
}

/// Minimum M6B renderer-neutral paint primitive vocabulary.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PaintPrimitive {
    /// Filled logical rectangle.
    FillRect { rect: LogicalRect, color: Color },
    /// Centered mitered logical rectangle stroke.
    StrokeRect {
        rect: LogicalRect,
        color: Color,
        width: LogicalLength,
    },
}

impl PaintPrimitive {
    /// Returns the primitive's owner-local rectangle.
    #[must_use]
    pub const fn rect(&self) -> LogicalRect {
        match self {
            Self::FillRect { rect, .. } | Self::StrokeRect { rect, .. } => *rect,
        }
    }

    /// Returns the primitive's literal unpremultiplied sRGB8 core color.
    #[must_use]
    pub const fn color(&self) -> Color {
        match self {
            Self::FillRect { color, .. } | Self::StrokeRect { color, .. } => *color,
        }
    }

    /// Returns stroke width when this is a stroke primitive.
    #[must_use]
    pub const fn stroke_width(&self) -> Option<LogicalLength> {
        match self {
            Self::FillRect { .. } => None,
            Self::StrokeRect { width, .. } => Some(*width),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PaintContribution, PaintContributionItem, PaintPrimitive};
    use crate::{Color, LogicalLength, LogicalRect};

    #[test]
    fn contribution_preserves_literal_color_geometry_and_order() {
        let first_rect = LogicalRect::try_new(0.0, 0.0, 10.0, 20.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let second_rect = LogicalRect::try_new(1.0, 2.0, 3.0, 4.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let stroke =
            LogicalLength::new(2.0).unwrap_or_else(|_| unreachable!("test stroke width is valid"));
        let contribution = PaintContribution::new(vec![
            PaintContributionItem::fill_rect(first_rect, Color::rgba(1, 2, 3, 4)),
            PaintContributionItem::stroke_rect(second_rect, Color::rgba(5, 6, 7, 8), stroke),
        ]);

        assert_eq!(contribution.items().len(), 2);
        assert!(matches!(
            contribution.items()[0].primitive(),
            PaintPrimitive::FillRect { rect, color }
                if *rect == first_rect && *color == Color::rgba(1, 2, 3, 4)
        ));
        assert!(matches!(
            contribution.items()[1].primitive(),
            PaintPrimitive::StrokeRect { rect, color, width }
                if *rect == second_rect
                    && *color == Color::rgba(5, 6, 7, 8)
                    && *width == stroke
        ));
    }

    #[test]
    fn zero_width_stroke_remains_literal_zero() {
        let rect = LogicalRect::try_new(0.0, 0.0, 1.0, 1.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let item = PaintContributionItem::stroke_rect(rect, Color::BLACK, LogicalLength::ZERO);
        assert_eq!(item.primitive().stroke_width(), Some(LogicalLength::ZERO));
    }
}
