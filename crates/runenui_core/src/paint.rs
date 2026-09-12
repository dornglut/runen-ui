//! Renderer-neutral owner-local paint contribution vocabulary.

use crate::paint_group::{NormalizedPaintGroup, PaintContributionEntry, normalize_entries};
use crate::{
    Brush, Color, ComputedStyle, ContributionClip, DropShadow, ImageIntrinsicSize,
    ImagePaintDescriptor, LogicalPoint, LogicalRect, LogicalSize, LogicalTransform, ResourceKind,
    ResourceKindMismatch, ResourceRef, SceneLayer, SceneOpacity, SceneShape, StrokeStyle,
};

/// Read-only facts supplied while one mounted widget contributes paint.
///
/// The context deliberately contains no mounted identity, surface origin,
/// raster scale, renderer/backend object, resource provider, semantic data, or
/// publication history.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintContributionContext {
    local_size: LogicalSize,
    computed_style: ComputedStyle,
}

impl PaintContributionContext {
    /// Returns the owner's final local logical size.
    #[must_use]
    pub const fn local_size(&self) -> LogicalSize {
        self.local_size
    }

    /// Returns the owner's resolved style facts.
    #[must_use]
    pub const fn computed_style(&self) -> &ComputedStyle {
        &self.computed_style
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
///
/// Ordinary flat contributions keep no group metadata. Explicit recursive authoring
/// is normalized immediately by [`Self::from_entries`] into this same flat item order
/// plus private transient group facts; the recursive authoring tree is not retained.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintContribution {
    items: Vec<PaintContributionItem>,
    groups: Vec<NormalizedPaintGroup>,
    item_groups: Vec<Option<usize>>,
}

impl PaintContribution {
    /// Empty contribution.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            items: Vec::new(),
            groups: Vec::new(),
            item_groups: Vec::new(),
        }
    }

    /// Creates one flat contribution from already validated items in local order.
    #[must_use]
    pub const fn new(items: Vec<PaintContributionItem>) -> Self {
        Self {
            items,
            groups: Vec::new(),
            item_groups: Vec::new(),
        }
    }

    /// Creates one contribution from recursive owner-local item/group authoring.
    ///
    /// The structure is consumed immediately. Items are retained in exact recursive
    /// authored order while structurally empty groups are omitted and remaining group
    /// membership becomes private contribution-local numeric structure only.
    #[must_use]
    pub fn from_entries(entries: Vec<PaintContributionEntry>) -> Self {
        let normalized = normalize_entries(entries);
        Self {
            items: normalized.items,
            groups: normalized.groups,
            item_groups: normalized.item_groups,
        }
    }

    /// Creates a one-item flat contribution.
    #[must_use]
    pub fn single(item: PaintContributionItem) -> Self {
        Self::new(vec![item])
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

    /// Runtime-only count of normalized owner-local groups.
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_group_count(&self) -> usize {
        self.groups.len()
    }

    /// Runtime-only normalized parent ordinal for one owner-local group.
    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_group_parent(&self, group_index: usize) -> Option<usize> {
        self.groups.get(group_index).and_then(|group| group.parent)
    }

    /// Runtime-only owner-local clips for one normalized group.
    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_group_clips(&self, group_index: usize) -> Option<&[ContributionClip]> {
        self.groups
            .get(group_index)
            .map(|group| group.clips.as_slice())
    }

    /// Runtime-only validated opacity for one normalized group.
    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_group_opacity(&self, group_index: usize) -> Option<SceneOpacity> {
        self.groups.get(group_index).map(|group| group.opacity)
    }

    /// Runtime-only ordered ordinary shadows for one normalized group.
    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_group_shadows(&self, group_index: usize) -> Option<&[DropShadow]> {
        self.groups
            .get(group_index)
            .map(|group| group.shadows.as_slice())
    }

    /// Runtime-only immediate normalized group ordinal for one flat item.
    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_item_group(&self, item_index: usize) -> Option<usize> {
        if self.item_groups.is_empty() {
            None
        } else {
            self.item_groups.get(item_index).copied().flatten()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ResolvedImagePatch {
    source: [f64; 4],
    destination: LogicalRect,
}

#[derive(Clone, Debug, PartialEq)]
struct ResolvedImagePrimitive {
    resource: ResourceRef,
    intrinsic_size: ImageIntrinsicSize,
    patches: Vec<ResolvedImagePatch>,
}

#[derive(Clone, Debug, PartialEq)]
enum ImagePrimitivePhase {
    Authored(ImagePaintDescriptor),
    Resolved(ResolvedImagePrimitive),
}

/// Image paint value with a strict owner-authored to runtime-publication phase boundary.
///
/// A contribution contains [`ImagePaintDescriptor`] policy. Runtime publication
/// replaces that policy with exact source-pixel/destination-logical patches before
/// a renderer can observe the item. Fit, crop, alignment and nine-slice policy
/// therefore never become renderer authority.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagePrimitive {
    phase: ImagePrimitivePhase,
}

impl ImagePrimitive {
    const fn authored(descriptor: ImagePaintDescriptor) -> Self {
        Self {
            phase: ImagePrimitivePhase::Authored(descriptor),
        }
    }

    /// Runtime-only bridge for exact resolved image publication geometry.
    ///
    /// Each source tuple is `[x, y, width, height]` in intrinsic pixel space.
    /// Invalid/non-finite/out-of-bounds source geometry returns `None` rather than
    /// creating a renderer-visible fallback interpretation.
    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_resolved(
        resource: ResourceRef,
        intrinsic_size: ImageIntrinsicSize,
        patches: Vec<([f64; 4], LogicalRect)>,
    ) -> Option<Self> {
        if resource.kind() != ResourceKind::Image {
            return None;
        }
        let intrinsic_width = f64::from(intrinsic_size.width());
        let intrinsic_height = f64::from(intrinsic_size.height());
        let mut resolved = Vec::with_capacity(patches.len());
        for (source, destination) in patches {
            let [x, y, width, height] = source;
            if !source.into_iter().all(f64::is_finite)
                || x < 0.0
                || y < 0.0
                || width < 0.0
                || height < 0.0
                || width > intrinsic_width
                || height > intrinsic_height
                || x > intrinsic_width - width
                || y > intrinsic_height - height
            {
                return None;
            }
            resolved.push(ResolvedImagePatch {
                source,
                destination,
            });
        }
        Some(Self {
            phase: ImagePrimitivePhase::Resolved(ResolvedImagePrimitive {
                resource,
                intrinsic_size,
                patches: resolved,
            }),
        })
    }

    /// Returns the complete opaque image resource reference in either phase.
    #[must_use]
    pub const fn resource_ref(&self) -> &ResourceRef {
        match &self.phase {
            ImagePrimitivePhase::Authored(descriptor) => descriptor.image().resource_ref(),
            ImagePrimitivePhase::Resolved(resolved) => &resolved.resource,
        }
    }

    /// Returns owner-authored image policy while this primitive is contribution-local.
    #[must_use]
    pub const fn authored_descriptor(&self) -> Option<&ImagePaintDescriptor> {
        match &self.phase {
            ImagePrimitivePhase::Authored(descriptor) => Some(descriptor),
            ImagePrimitivePhase::Resolved(_) => None,
        }
    }

    /// Returns runtime-retained intrinsic metadata for a resolved publication image.
    #[must_use]
    pub const fn resolved_intrinsic_size(&self) -> Option<ImageIntrinsicSize> {
        match &self.phase {
            ImagePrimitivePhase::Authored(_) => None,
            ImagePrimitivePhase::Resolved(resolved) => Some(resolved.intrinsic_size),
        }
    }

    /// Returns the number of runtime-resolved image patches, or `None` while authored.
    #[must_use]
    pub const fn resolved_patch_count(&self) -> Option<usize> {
        match &self.phase {
            ImagePrimitivePhase::Authored(_) => None,
            ImagePrimitivePhase::Resolved(resolved) => Some(resolved.patches.len()),
        }
    }

    /// Returns one exact runtime-resolved source/destination patch.
    ///
    /// The source tuple is `[x, y, width, height]` in intrinsic pixel space.
    #[must_use]
    pub fn resolved_patch(&self, index: usize) -> Option<([f64; 4], LogicalRect)> {
        let ImagePrimitivePhase::Resolved(resolved) = &self.phase else {
            return None;
        };
        resolved
            .patches
            .get(index)
            .map(|patch| (patch.source, patch.destination))
    }
}

/// One validated shaped-text-run paint primitive.
///
/// `origin` maps resource-local logical `(0, 0)` into the owner's local logical
/// coordinates. Glyph geometry remains resource-owned; `foreground` is ordinary
/// scene-owned literal color and is intentionally outside resource identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapedTextRunPrimitive {
    resource: ResourceRef,
    origin: LogicalPoint,
    foreground: Color,
}

impl ShapedTextRunPrimitive {
    /// Creates a shaped-run primitive from a shaped-text-run resource reference.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceKindMismatch`] when `resource` has another kind.
    pub fn new(
        resource: ResourceRef,
        origin: LogicalPoint,
        foreground: Color,
    ) -> Result<Self, ResourceKindMismatch> {
        if resource.kind() != ResourceKind::ShapedTextRun {
            return Err(ResourceKindMismatch::new(
                ResourceKind::ShapedTextRun,
                resource.kind(),
            ));
        }
        Ok(Self {
            resource,
            origin,
            foreground,
        })
    }

    /// Returns the complete opaque shaped-run resource reference.
    #[must_use]
    pub const fn resource_ref(&self) -> &ResourceRef {
        &self.resource
    }

    /// Returns the finite owner-local placement of resource-local `(0, 0)`.
    #[must_use]
    pub const fn origin(&self) -> LogicalPoint {
        self.origin
    }

    /// Returns the ordinary literal core foreground color.
    #[must_use]
    pub const fn foreground(&self) -> Color {
        self.foreground
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

    /// Creates one generic filled logical shape.
    #[must_use]
    pub const fn fill(shape: SceneShape, brush: Brush) -> Self {
        Self::from_primitive(PaintPrimitive::Fill { shape, brush })
    }

    /// Creates one generic centered logical shape stroke.
    ///
    /// [`StrokeStyle`] owns the complete initial cap/join/miter contract. A zero
    /// width remains literal no-coverage semantics and is never a backend hairline.
    #[must_use]
    pub const fn stroke(shape: SceneShape, brush: Brush, style: StrokeStyle) -> Self {
        Self::from_primitive(PaintPrimitive::Stroke {
            shape,
            brush,
            style,
        })
    }

    /// Creates one owner-local image item from complete validated image paint policy.
    #[must_use]
    pub const fn image(descriptor: ImagePaintDescriptor) -> Self {
        Self::from_primitive(PaintPrimitive::Image(ImagePrimitive::authored(descriptor)))
    }

    /// Creates a shaped-text-run item with exact owner-local origin and literal foreground.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceKindMismatch`] when `resource` is not shaped-run-kind.
    pub fn shaped_text_run(
        resource: ResourceRef,
        origin: LogicalPoint,
        foreground: Color,
    ) -> Result<Self, ResourceKindMismatch> {
        ShapedTextRunPrimitive::new(resource, origin, foreground)
            .map(|run| Self::from_primitive(PaintPrimitive::ShapedTextRun(run)))
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
    /// Generic logical shape filled by one `RunenUI` brush.
    Fill { shape: SceneShape, brush: Brush },
    /// Generic logical shape stroked by one `RunenUI` brush and centered stroke style.
    Stroke {
        shape: SceneShape,
        brush: Brush,
        style: StrokeStyle,
    },
    /// Image contribution/publication value with an explicit authored/resolved phase boundary.
    Image(ImagePrimitive),
    /// Shaped resource whose local origin is placed at one finite logical point.
    ShapedTextRun(ShapedTextRunPrimitive),
}

impl PaintPrimitive {
    /// Returns generic shape geometry for fill/stroke primitives.
    #[must_use]
    pub const fn shape(&self) -> Option<&SceneShape> {
        match self {
            Self::Fill { shape, .. } | Self::Stroke { shape, .. } => Some(shape),
            Self::Image(_) | Self::ShapedTextRun(_) => None,
        }
    }

    /// Returns the `RunenUI` brush for generic fill/stroke primitives.
    #[must_use]
    pub const fn brush(&self) -> Option<&Brush> {
        match self {
            Self::Fill { brush, .. } | Self::Stroke { brush, .. } => Some(brush),
            Self::Image(_) | Self::ShapedTextRun(_) => None,
        }
    }

    /// Returns centered stroke policy when this is a stroke primitive.
    #[must_use]
    pub const fn stroke_style(&self) -> Option<StrokeStyle> {
        match self {
            Self::Stroke { style, .. } => Some(*style),
            Self::Fill { .. } | Self::Image(_) | Self::ShapedTextRun(_) => None,
        }
    }

    /// Returns the complete opaque resource reference for resource-backed primitives.
    #[must_use]
    pub const fn resource_ref(&self) -> Option<&ResourceRef> {
        match self {
            Self::Image(image) => Some(image.resource_ref()),
            Self::ShapedTextRun(run) => Some(run.resource_ref()),
            Self::Fill { .. } | Self::Stroke { .. } => None,
        }
    }

    /// Returns image-specific authored/resolved facts when this is an image primitive.
    #[must_use]
    pub const fn as_image(&self) -> Option<&ImagePrimitive> {
        match self {
            Self::Image(image) => Some(image),
            Self::Fill { .. } | Self::Stroke { .. } | Self::ShapedTextRun(_) => None,
        }
    }

    /// Returns shaped-run-specific placement/color facts when this is a shaped run.
    #[must_use]
    pub const fn as_shaped_text_run(&self) -> Option<&ShapedTextRunPrimitive> {
        match self {
            Self::ShapedTextRun(run) => Some(run),
            Self::Fill { .. } | Self::Stroke { .. } | Self::Image(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PaintContribution, PaintContributionItem, PaintPrimitive};
    use crate::{
        Brush, Color, ContributionClip, ImageDescriptor, ImageIntrinsicSize, ImageMapping,
        ImagePaintDescriptor, LogicalLength, LogicalPoint, LogicalRect, LogicalTransform,
        ResourceKind, ResourceKindMismatch, ResourceRef, SceneLayer, SceneOpacity, SceneShape,
        StrokeStyle,
    };

    #[test]
    fn contribution_preserves_generic_shape_brush_stroke_and_order() {
        let first_rect = LogicalRect::try_new(0.0, 0.0, 10.0, 20.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let second_rect = LogicalRect::try_new(1.0, 2.0, 3.0, 4.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let stroke = StrokeStyle::new(
            LogicalLength::new(2.0).unwrap_or_else(|_| unreachable!("test stroke width is valid")),
        );
        let first_brush = Brush::solid(Color::rgba(1, 2, 3, 4));
        let second_brush = Brush::solid(Color::rgba(5, 6, 7, 8));
        let contribution = PaintContribution::new(vec![
            PaintContributionItem::fill(SceneShape::rect(first_rect), first_brush.clone()),
            PaintContributionItem::stroke(
                SceneShape::rect(second_rect),
                second_brush.clone(),
                stroke,
            ),
        ]);

        assert_eq!(contribution.items().len(), 2);
        assert!(matches!(
            contribution.items()[0].primitive(),
            PaintPrimitive::Fill { shape: SceneShape::Rect(rect), brush }
                if *rect == first_rect && brush == &first_brush
        ));
        assert!(matches!(
            contribution.items()[1].primitive(),
            PaintPrimitive::Stroke { shape: SceneShape::Rect(rect), brush, style }
                if *rect == second_rect && brush == &second_brush && *style == stroke
        ));
        assert_eq!(
            contribution.items()[0].primitive().shape(),
            Some(&SceneShape::rect(first_rect))
        );
        assert_eq!(
            contribution.items()[0].primitive().brush(),
            Some(&first_brush)
        );
        assert_eq!(
            contribution.items()[1].primitive().stroke_style(),
            Some(stroke)
        );
        assert_eq!(contribution.__runtime_group_count(), 0);
        assert_eq!(contribution.__runtime_item_group(0), None);
    }

    #[test]
    fn zero_width_stroke_remains_literal_zero() {
        let rect = LogicalRect::try_new(0.0, 0.0, 1.0, 1.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let style = StrokeStyle::new(LogicalLength::ZERO);
        let item = PaintContributionItem::stroke(
            SceneShape::rect(rect),
            Brush::solid(Color::BLACK),
            style,
        );
        assert_eq!(item.primitive().stroke_style(), Some(style));
    }

    #[test]
    fn image_contribution_is_authored_and_runtime_bridge_is_resolved() {
        let resource = ResourceRef::new(ResourceKind::Image);
        let intrinsic = ImageIntrinsicSize::new(40, 20)
            .unwrap_or_else(|| unreachable!("test image extent is non-zero"));
        let descriptor = ImageDescriptor::new(resource.clone(), intrinsic)
            .unwrap_or_else(|_| unreachable!("test resource has image kind"));
        let destination = LogicalRect::try_new(2.0, 3.0, 80.0, 40.0)
            .unwrap_or_else(|_| unreachable!("test destination is valid"));
        let paint = ImagePaintDescriptor::new(descriptor, destination, ImageMapping::default())
            .unwrap_or_else(|_| unreachable!("test mapping is valid"));
        let authored = PaintContributionItem::image(paint.clone());
        let image = authored
            .primitive()
            .as_image()
            .unwrap_or_else(|| unreachable!("fixture is image"));
        assert_eq!(image.resource_ref(), &resource);
        assert_eq!(image.authored_descriptor(), Some(&paint));
        assert_eq!(image.resolved_intrinsic_size(), None);
        assert_eq!(image.resolved_patch_count(), None);

        let resolved = super::ImagePrimitive::__runtime_resolved(
            resource.clone(),
            intrinsic,
            vec![([0.0, 0.0, 40.0, 20.0], destination)],
        )
        .unwrap_or_else(|| unreachable!("fixture runtime geometry is valid"));
        assert_eq!(resolved.resource_ref(), &resource);
        assert_eq!(resolved.authored_descriptor(), None);
        assert_eq!(resolved.resolved_intrinsic_size(), Some(intrinsic));
        assert_eq!(resolved.resolved_patch_count(), Some(1));
        assert_eq!(
            resolved.resolved_patch(0),
            Some(([0.0, 0.0, 40.0, 20.0], destination))
        );
    }

    #[test]
    fn runtime_image_bridge_rejects_out_of_bounds_source_geometry() {
        let intrinsic = ImageIntrinsicSize::new(40, 20)
            .unwrap_or_else(|| unreachable!("test image extent is non-zero"));
        let destination = LogicalRect::try_new(0.0, 0.0, 1.0, 1.0)
            .unwrap_or_else(|_| unreachable!("test destination is valid"));
        assert!(
            super::ImagePrimitive::__runtime_resolved(
                ResourceRef::new(ResourceKind::Image),
                intrinsic,
                vec![([39.0, 0.0, 2.0, 20.0], destination)],
            )
            .is_none()
        );
    }

    #[test]
    fn runtime_image_bridge_preserves_large_exact_intrinsic_source_extent() {
        let intrinsic = ImageIntrinsicSize::new(16_777_217, 1)
            .unwrap_or_else(|| unreachable!("test image extent is non-zero"));
        let destination = LogicalRect::try_new(0.0, 0.0, 1.0, 1.0)
            .unwrap_or_else(|_| unreachable!("test destination is valid"));
        let resolved = super::ImagePrimitive::__runtime_resolved(
            ResourceRef::new(ResourceKind::Image),
            intrinsic,
            vec![([0.0, 0.0, 16_777_217.0, 1.0], destination)],
        )
        .unwrap_or_else(|| unreachable!("f64 source geometry retains exact u32 extent"));
        assert_eq!(
            resolved.resolved_patch(0),
            Some(([0.0, 0.0, 16_777_217.0, 1.0], destination))
        );
    }

    #[test]
    fn shaped_run_validates_kind_and_preserves_facts() {
        let origin =
            LogicalPoint::new(4.0, 7.0).unwrap_or_else(|_| unreachable!("test origin is finite"));
        let shaped_ref = ResourceRef::new(ResourceKind::ShapedTextRun);
        let run = PaintContributionItem::shaped_text_run(
            shaped_ref.clone(),
            origin,
            Color::rgba(1, 2, 3, 4),
        )
        .unwrap_or_else(|_| unreachable!("shaped ref has shaped-run kind"));
        assert_eq!(run.primitive().resource_ref(), Some(&shaped_ref));
        assert_eq!(
            run.primitive()
                .as_shaped_text_run()
                .map(super::ShapedTextRunPrimitive::origin),
            Some(origin)
        );
        assert_eq!(
            run.primitive()
                .as_shaped_text_run()
                .map(super::ShapedTextRunPrimitive::foreground),
            Some(Color::rgba(1, 2, 3, 4))
        );

        let Err(wrong_run) = PaintContributionItem::shaped_text_run(
            ResourceRef::new(ResourceKind::Image),
            origin,
            Color::BLACK,
        ) else {
            unreachable!("image refs cannot become shaped-run primitives");
        };
        assert_eq!(
            wrong_run,
            ResourceKindMismatch::new(ResourceKind::ShapedTextRun, ResourceKind::Image)
        );
    }

    #[test]
    fn item_composition_defaults_and_explicit_values_are_self_contained() {
        let rect = LogicalRect::try_new(0.0, 0.0, 4.0, 5.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let default_item =
            PaintContributionItem::fill(SceneShape::rect(rect), Brush::solid(Color::WHITE));
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
            .with_clip(clip.clone())
            .with_opacity(opacity)
            .with_layer(SceneLayer::new(-2));
        assert_eq!(item.local_transform(), transform);
        assert_eq!(item.clips(), &[clip]);
        assert_eq!(item.opacity(), opacity);
        assert_eq!(item.layer(), SceneLayer::new(-2));
    }
}
