//! Renderer-neutral owner-local paint contribution vocabulary.

use crate::{
    Color, ComputedStyle, ContributionClip, LogicalLength, LogicalRect, LogicalSize,
    LogicalTransform, SceneLayer, SceneOpacity,
};

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
///
/// Every item is self-contained: primitive, owner-local transform, conjunctive
/// clips, validated opacity, and snapshot-local layer are explicit values rather
/// than push/pop command state.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintContributionItem {
    primitive: PaintPrimitive,
    local_transform: LogicalTransform,
    clips: Vec<ContributionClip>,
    opacity: SceneOpacity,
    layer: SceneLayer,
}

impl PaintContributionItem {
    const fn from_primitive(primitive: PaintPrimitive) -> Self {
        Self {
            primitive,
            local_transform: LogicalTransform::IDENTITY,
            clips: Vec::new(),
            opacity: SceneOpacity::OPAQUE,
            layer: SceneLayer::ZERO,
        }
    }

    /// Creates a filled logical rectangle using one literal core color.
    #[must_use]
    pub const fn fill_rect(rect: LogicalRect, color: Color) -> Self {
        Self::from_primitive(PaintPrimitive::FillRect { rect, color })
    }

    /// Creates a centered logical rectangle stroke.
    ///
    /// [`LogicalLength`] guarantees a finite non-negative width. Width zero is
    /// retained literally and means no stroke coverage; it is never a backend
    /// hairline request.
    #[must_use]
    pub const fn stroke_rect(rect: LogicalRect, color: Color, width: LogicalLength) -> Self {
        Self::from_primitive(PaintPrimitive::StrokeRect { rect, color, width })
    }

    /// Replaces the item's primitive-local to owner-local transform.
    #[must_use]
    pub const fn with_transform(mut self, transform: LogicalTransform) -> Self {
        self.local_transform = transform;
        self
    }

    /// Appends one conjunctive owner-local clip.
    #[must_use]
    pub fn with_clip(mut self, clip: ContributionClip) -> Self {
        self.clips.push(clip);
        self
    }

    /// Replaces item opacity.
    #[must_use]
    pub const fn with_opacity(mut self, opacity: SceneOpacity) -> Self {
        self.opacity = opacity;
        self
    }

    /// Replaces snapshot-local ordering layer.
    #[must_use]
    pub const fn with_layer(mut self, layer: SceneLayer) -> Self {
        self.layer = layer;
        self
    }

    /// Returns the renderer-neutral primitive.
    #[must_use]
    pub const fn primitive(&self) -> &PaintPrimitive {
        &self.primitive
    }

    /// Returns primitive-local to owner-local transform.
    #[must_use]
    pub const fn local_transform(&self) -> LogicalTransform {
        self.local_transform
    }

    /// Returns conjunctive clips in authored order.
    #[must_use]
    pub const fn clips(&self) -> &[ContributionClip] {
        self.clips.as_slice()
    }

    /// Returns validated item opacity.
    #[must_use]
    pub const fn opacity(&self) -> SceneOpacity {
        self.opacity
    }

    /// Returns snapshot-local ordering layer.
    #[must_use]
    pub const fn layer(&self) -> SceneLayer {
        self.layer
    }
}

/// Minimum renderer-neutral paint primitive vocabulary.
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
    use crate::{
        Color, ContributionClip, LogicalLength, LogicalRect, LogicalTransform, SceneLayer,
        SceneOpacity, SceneShape,
    };

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

    #[test]
    fn item_composition_defaults_and_explicit_values_are_self_contained() {
        let rect = LogicalRect::try_new(0.0, 0.0, 4.0, 5.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let default_item = PaintContributionItem::fill_rect(rect, Color::WHITE);
        assert_eq!(default_item.local_transform(), LogicalTransform::IDENTITY);
        assert!(default_item.clips().is_empty());
        assert_eq!(default_item.opacity(), SceneOpacity::OPAQUE);
        assert_eq!(default_item.layer(), SceneLayer::ZERO);

        let transform = LogicalTransform::translation(3.0, 7.0)
            .unwrap_or_else(|_| unreachable!("test transform is valid"));
        let opacity =
            SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("test opacity is valid"));
        let clip = ContributionClip::identity(SceneShape::rect(rect));
        let item = default_item
            .with_transform(transform)
            .with_clip(clip)
            .with_opacity(opacity)
            .with_layer(SceneLayer::new(-2));
        assert_eq!(item.local_transform(), transform);
        assert_eq!(item.clips(), &[clip]);
        assert_eq!(item.opacity(), opacity);
        assert_eq!(item.layer(), SceneLayer::new(-2));
    }
}
