use runenui_core::{
    __runtime::transform_rect_aabb, DropShadow, ImagePrimitive, LogicalRect, LogicalTransform,
    PaintPrimitive, SceneShape, StrokeCap, StrokeJoin, StrokeStyle,
};
use runenui_text::TextInkBounds;

use crate::scene::{PaintScene, PaintSceneEntry, PaintSceneGroupId, PaintSceneItem, SceneClip};

/// Conservative surface-logical coverage bound derived from one immutable paint snapshot.
///
/// `Finite` may contain a zero-width or zero-height rectangle because accepted path boundary
/// coverage can legitimately be one-dimensional. `Unbounded` is the deterministic conservative
/// top value used when accepted neutral facts prove coverage exists or may exist but no finite
/// representable logical AABB can be established without renderer/resource guesses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PaintSceneBounds {
    /// The accepted logical paint coverage is empty.
    Empty,
    /// One finite conservative surface-logical AABB.
    Finite(LogicalRect),
    /// No finite conservative logical AABB can be established from accepted scene facts.
    Unbounded,
}

impl PaintSceneBounds {
    /// Returns the finite conservative AABB, when available.
    #[must_use]
    pub const fn finite_rect(self) -> Option<LogicalRect> {
        match self {
            Self::Finite(rect) => Some(rect),
            Self::Empty | Self::Unbounded => None,
        }
    }

    /// Returns whether accepted logical coverage is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns whether only the conservative top bound is available.
    #[must_use]
    pub const fn is_unbounded(self) -> bool {
        matches!(self, Self::Unbounded)
    }
}

impl PaintScene {
    /// Derives one paint item's conservative surface-logical coverage bound.
    ///
    /// Bounds are computed from this exact immutable scene snapshot and its retained shaped-text
    /// bindings. No raster scale, renderer tessellation, device cache, atlas, image pixels, or
    /// backend kernel participates. Item opacity and brush/resource alpha intentionally do not
    /// shrink this geometric bound, avoiding a second source-coverage authority.
    #[must_use]
    pub fn item_bounds(&self, item_index: usize) -> Option<PaintSceneBounds> {
        self.items()
            .get(item_index)
            .map(|item| derive_item_bounds(self, item))
    }

    /// Derives one composition group's complete conservative surface-logical output bound.
    ///
    /// Direct items and nested groups first compose recursively into one pre-shadow child bound.
    /// Every ordinary shadow derives independently from that same child bound. Positive spread
    /// expands conservatively, zero/negative spread retain the child AABB when no tighter neutral
    /// erosion bound is available, then offset and finite `3 * sigma` support apply. Group clips
    /// constrain the complete child-plus-effects result. Group opacity and shadow color alpha do
    /// not shrink this geometric metadata or create a second source-alpha authority.
    #[must_use]
    pub fn group_bounds(&self, group_id: PaintSceneGroupId) -> Option<PaintSceneBounds> {
        derive_group_bounds(self, group_id)
    }
}

fn derive_item_bounds(scene: &PaintScene, item: &PaintSceneItem) -> PaintSceneBounds {
    let (local_bounds, local_to_surface) = match item.primitive() {
        PaintPrimitive::Fill { shape, .. } => (fill_shape_bounds(shape), item.local_to_surface()),
        PaintPrimitive::Stroke { shape, style, .. } => {
            (stroke_shape_bounds(shape, *style), item.local_to_surface())
        }
        PaintPrimitive::Image(image) => (image_local_bounds(image), item.local_to_surface()),
        PaintPrimitive::ShapedTextRun(run) => {
            let bounds = scene.shaped_text_resource(run.resource_ref()).map_or(
                PaintSceneBounds::Unbounded,
                |resource| match resource.logical_ink_bounds() {
                    TextInkBounds::Empty => PaintSceneBounds::Empty,
                    TextInkBounds::Finite(rect) => PaintSceneBounds::Finite(rect),
                    TextInkBounds::Unbounded => PaintSceneBounds::Unbounded,
                },
            );
            let Ok(origin) = LogicalTransform::translation(run.origin().x(), run.origin().y())
            else {
                return PaintSceneBounds::Unbounded;
            };
            let Ok(transform) = origin.then(item.local_to_surface()) else {
                return PaintSceneBounds::Unbounded;
            };
            (bounds, transform)
        }
        _ => (PaintSceneBounds::Unbounded, item.local_to_surface()),
    };

    let mut bounds = transform_bounds(local_bounds, local_to_surface);
    for clip in item.clips() {
        bounds = intersect_bounds(bounds, clip_bounds(clip));
        if bounds.is_empty() {
            break;
        }
    }
    bounds
}

fn derive_group_bounds(
    scene: &PaintScene,
    group_id: PaintSceneGroupId,
) -> Option<PaintSceneBounds> {
    let group = scene.group(group_id)?;
    let mut child_bounds = PaintSceneBounds::Empty;
    for entry in group.entries() {
        child_bounds = union_bounds(child_bounds, entry_bounds(scene, *entry));
    }

    let mut bounds = child_bounds;
    for shadow in group.shadows() {
        bounds = union_bounds(bounds, shadow_effect_bounds(child_bounds, *shadow));
    }
    for clip in group.clips() {
        bounds = intersect_bounds(bounds, clip_bounds(clip));
        if bounds.is_empty() {
            break;
        }
    }
    Some(bounds)
}

fn entry_bounds(scene: &PaintScene, entry: PaintSceneEntry) -> PaintSceneBounds {
    if let Some(item_index) = entry.item_index() {
        return scene
            .item_bounds(item_index)
            .unwrap_or(PaintSceneBounds::Unbounded);
    }
    if let Some(group_id) = entry.group_id() {
        return derive_group_bounds(scene, group_id).unwrap_or(PaintSceneBounds::Unbounded);
    }
    PaintSceneBounds::Unbounded
}

fn shadow_effect_bounds(source: PaintSceneBounds, shadow: DropShadow) -> PaintSceneBounds {
    let spread = shadow.spread();
    let spread_bounds = if spread > 0.0 {
        expand_bounds(source, f64::from(spread))
    } else {
        source
    };

    let Ok(offset) = LogicalTransform::translation(shadow.offset_x(), shadow.offset_y()) else {
        return PaintSceneBounds::Unbounded;
    };
    let offset_bounds = transform_bounds(spread_bounds, offset);
    let blur_margin = 3.0 * f64::from(shadow.sigma().get());
    expand_bounds(offset_bounds, blur_margin)
}

fn fill_shape_bounds(shape: &SceneShape) -> PaintSceneBounds {
    match shape {
        SceneShape::Rect(rect)
        | SceneShape::RoundedRect { rect, .. }
        | SceneShape::Ellipse(rect) => {
            if rect.width() == 0.0 || rect.height() == 0.0 {
                PaintSceneBounds::Empty
            } else {
                PaintSceneBounds::Finite(*rect)
            }
        }
        SceneShape::Path(path) => path
            .logical_bounds()
            .map_or(PaintSceneBounds::Empty, PaintSceneBounds::Finite),
    }
}

fn stroke_shape_bounds(shape: &SceneShape, style: StrokeStyle) -> PaintSceneBounds {
    let width = style.width().get();
    if width == 0.0 {
        return PaintSceneBounds::Empty;
    }

    let centerline = match shape {
        SceneShape::Rect(rect)
        | SceneShape::RoundedRect { rect, .. }
        | SceneShape::Ellipse(rect) => {
            if rect.width() == 0.0 || rect.height() == 0.0 {
                // ADR 0014 preserves zero-extent rectangle-family stroke emptiness;
                // ADR 0011 already owns the equivalent ellipse rule.
                return PaintSceneBounds::Empty;
            }
            *rect
        }
        SceneShape::Path(path) => {
            let Some(bounds) = path.logical_bounds() else {
                return PaintSceneBounds::Empty;
            };
            bounds
        }
    };

    let join_margin = if style.join() == StrokeJoin::Miter {
        f64::from(width) * f64::from(style.miter_limit())
    } else {
        f64::from(width) * 0.5
    };
    let cap_margin = match style.cap() {
        StrokeCap::Square => f64::from(width) * 0.5 * core::f64::consts::SQRT_2,
        StrokeCap::Butt | StrokeCap::Round => f64::from(width) * 0.5,
    };
    let margin = join_margin.max(cap_margin);
    expand_rect(centerline, margin).map_or(PaintSceneBounds::Unbounded, PaintSceneBounds::Finite)
}

fn image_local_bounds(image: &ImagePrimitive) -> PaintSceneBounds {
    let Some(patch_count) = image.resolved_patch_count() else {
        return PaintSceneBounds::Unbounded;
    };
    let mut bounds = PaintSceneBounds::Empty;
    for index in 0..patch_count {
        let Some((_, destination)) = image.resolved_patch(index) else {
            return PaintSceneBounds::Unbounded;
        };
        bounds = union_bounds(bounds, PaintSceneBounds::Finite(destination));
        if bounds.is_unbounded() {
            break;
        }
    }
    bounds
}

fn clip_bounds(clip: &SceneClip) -> PaintSceneBounds {
    if clip.clip_to_surface().inverse().is_none() {
        return PaintSceneBounds::Empty;
    }
    transform_bounds(fill_shape_bounds(clip.shape()), clip.clip_to_surface())
}

fn transform_bounds(bounds: PaintSceneBounds, transform: LogicalTransform) -> PaintSceneBounds {
    match bounds {
        PaintSceneBounds::Empty => PaintSceneBounds::Empty,
        PaintSceneBounds::Finite(rect) => {
            if transform.inverse().is_none() {
                PaintSceneBounds::Empty
            } else {
                transform_rect_aabb(transform, rect)
                    .map_or(PaintSceneBounds::Unbounded, PaintSceneBounds::Finite)
            }
        }
        PaintSceneBounds::Unbounded => {
            if transform.inverse().is_none() {
                PaintSceneBounds::Empty
            } else {
                PaintSceneBounds::Unbounded
            }
        }
    }
}

fn union_bounds(left: PaintSceneBounds, right: PaintSceneBounds) -> PaintSceneBounds {
    match (left, right) {
        (PaintSceneBounds::Unbounded, _) | (_, PaintSceneBounds::Unbounded) => {
            PaintSceneBounds::Unbounded
        }
        (PaintSceneBounds::Empty, other) | (other, PaintSceneBounds::Empty) => other,
        (PaintSceneBounds::Finite(left), PaintSceneBounds::Finite(right)) => {
            let (left_min_x, left_min_y, left_max_x, left_max_y) = rect_edges(left);
            let (right_min_x, right_min_y, right_max_x, right_max_y) = rect_edges(right);
            logical_rect_from_edges(
                left_min_x.min(right_min_x),
                left_min_y.min(right_min_y),
                left_max_x.max(right_max_x),
                left_max_y.max(right_max_y),
            )
            .map_or(PaintSceneBounds::Unbounded, PaintSceneBounds::Finite)
        }
    }
}

fn intersect_bounds(left: PaintSceneBounds, right: PaintSceneBounds) -> PaintSceneBounds {
    match (left, right) {
        (PaintSceneBounds::Empty, _) | (_, PaintSceneBounds::Empty) => PaintSceneBounds::Empty,
        (PaintSceneBounds::Unbounded, other) | (other, PaintSceneBounds::Unbounded) => other,
        (PaintSceneBounds::Finite(left), PaintSceneBounds::Finite(right)) => {
            let (left_min_x, left_min_y, left_max_x, left_max_y) = rect_edges(left);
            let (right_min_x, right_min_y, right_max_x, right_max_y) = rect_edges(right);
            let min_x = left_min_x.max(right_min_x);
            let min_y = left_min_y.max(right_min_y);
            let max_x = left_max_x.min(right_max_x);
            let max_y = left_max_y.min(right_max_y);
            if max_x < min_x || max_y < min_y {
                PaintSceneBounds::Empty
            } else {
                logical_rect_from_edges(min_x, min_y, max_x, max_y)
                    .map_or(PaintSceneBounds::Unbounded, PaintSceneBounds::Finite)
            }
        }
    }
}

fn expand_bounds(bounds: PaintSceneBounds, margin: f64) -> PaintSceneBounds {
    match bounds {
        PaintSceneBounds::Empty => PaintSceneBounds::Empty,
        PaintSceneBounds::Unbounded => PaintSceneBounds::Unbounded,
        PaintSceneBounds::Finite(rect) => {
            expand_rect(rect, margin).map_or(PaintSceneBounds::Unbounded, PaintSceneBounds::Finite)
        }
    }
}

fn expand_rect(rect: LogicalRect, margin: f64) -> Option<LogicalRect> {
    if !margin.is_finite() || margin < 0.0 {
        return None;
    }
    let (min_x, min_y, max_x, max_y) = rect_edges(rect);
    logical_rect_from_edges(
        min_x - margin,
        min_y - margin,
        max_x + margin,
        max_y + margin,
    )
}

fn rect_edges(rect: LogicalRect) -> (f64, f64, f64, f64) {
    let min_x = f64::from(rect.x());
    let min_y = f64::from(rect.y());
    (
        min_x,
        min_y,
        min_x + f64::from(rect.width()),
        min_y + f64::from(rect.height()),
    )
}

fn logical_rect_from_edges(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Option<LogicalRect> {
    if ![min_x, min_y, max_x, max_y].into_iter().all(f64::is_finite)
        || max_x < min_x
        || max_y < min_y
    {
        return None;
    }
    let min_x = checked_f32_down(min_x)?;
    let min_y = checked_f32_down(min_y)?;
    let max_x = checked_f32_up(max_x)?;
    let max_y = checked_f32_up(max_y)?;
    let width = checked_f32_up(f64::from(max_x) - f64::from(min_x))?;
    let height = checked_f32_up(f64::from(max_y) - f64::from(min_y))?;
    LogicalRect::try_new(min_x, min_y, width, height).ok()
}

#[allow(clippy::cast_possible_truncation)]
fn checked_f32(value: f64) -> Option<f32> {
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return None;
    }
    Some(value as f32)
}

fn checked_f32_down(value: f64) -> Option<f32> {
    let rounded = checked_f32(value)?;
    Some(if f64::from(rounded) > value {
        rounded.next_down()
    } else {
        rounded
    })
}

fn checked_f32_up(value: f64) -> Option<f32> {
    let rounded = checked_f32(value)?;
    Some(if f64::from(rounded) < value {
        rounded.next_up()
    } else {
        rounded
    })
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        Brush, Color, ImageIntrinsicSize, ImagePrimitive, LogicalLength, LogicalPoint, LogicalRect,
        LogicalTransform, PaintPrimitive, PathFillRule, PathVerb, Radius, ResourceKind,
        ResourceRef, SceneLayer, SceneOpacity, ScenePath, SceneShape, ShapedTextRunPrimitive,
        StrokeCap, StrokeJoin, StrokeStyle,
    };

    use crate::scene::{PaintSceneComposition, PaintSceneItem, SceneClip};

    use super::{PaintScene, PaintSceneBounds, intersect_bounds};

    fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
        LogicalRect::try_new(x, y, width, height)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"))
    }

    fn scene_item(
        primitive: PaintPrimitive,
        transform: LogicalTransform,
        clips: Vec<SceneClip>,
    ) -> PaintSceneItem {
        PaintSceneItem::new(
            primitive,
            transform,
            clips,
            SceneOpacity::OPAQUE,
            SceneLayer::ZERO,
        )
    }

    fn scene(items: Vec<PaintSceneItem>) -> PaintScene {
        let item_count = items.len();
        PaintScene::with_composition(
            items,
            Vec::new(),
            PaintSceneComposition::ungrouped(item_count),
        )
    }

    #[test]
    fn transformed_fill_and_independent_clip_intersect_conservatively() {
        let transform = LogicalTransform::translation(5.0, 7.0)
            .unwrap_or_else(|_| unreachable!("test transform is finite"));
        let clip_transform = LogicalTransform::translation(8.0, 9.0)
            .unwrap_or_else(|_| unreachable!("test transform is finite"));
        let item = scene_item(
            PaintPrimitive::Fill {
                shape: SceneShape::rect(rect(0.0, 0.0, 10.0, 10.0)),
                brush: Brush::solid(Color::BLACK),
            },
            transform,
            vec![SceneClip::new(
                SceneShape::rect(rect(0.0, 0.0, 4.0, 4.0)),
                clip_transform,
            )],
        );
        assert_eq!(
            scene(vec![item]).item_bounds(0),
            Some(PaintSceneBounds::Finite(rect(8.0, 9.0, 4.0, 4.0)))
        );
    }

    #[test]
    fn singular_item_transform_and_zero_area_rect_are_empty() {
        let singular = LogicalTransform::try_new(0.0, 0.0, 0.0, 0.0, 1.0, 2.0)
            .unwrap_or_else(|_| unreachable!("singular transform is still finite"));
        let fill = PaintPrimitive::Fill {
            shape: SceneShape::rect(rect(0.0, 0.0, 10.0, 10.0)),
            brush: Brush::solid(Color::BLACK),
        };
        let zero = PaintPrimitive::Fill {
            shape: SceneShape::rect(rect(0.0, 0.0, 0.0, 10.0)),
            brush: Brush::solid(Color::BLACK),
        };
        let scene = scene(vec![
            scene_item(fill, singular, Vec::new()),
            scene_item(zero, LogicalTransform::IDENTITY, Vec::new()),
        ]);
        assert_eq!(scene.item_bounds(0), Some(PaintSceneBounds::Empty));
        assert_eq!(scene.item_bounds(1), Some(PaintSceneBounds::Empty));
    }

    #[test]
    fn segment_bearing_zero_area_path_keeps_finite_boundary_bound() {
        let path = ScenePath::new(
            vec![
                PathVerb::MoveTo(LogicalPoint::new(2.0, 3.0).unwrap_or_else(|_| unreachable!())),
                PathVerb::LineTo(LogicalPoint::new(12.0, 3.0).unwrap_or_else(|_| unreachable!())),
            ],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("test path is valid"));
        let item = scene_item(
            PaintPrimitive::Fill {
                shape: SceneShape::path(path),
                brush: Brush::solid(Color::BLACK),
            },
            LogicalTransform::IDENTITY,
            Vec::new(),
        );
        assert_eq!(
            scene(vec![item]).item_bounds(0),
            Some(PaintSceneBounds::Finite(rect(2.0, 3.0, 10.0, 0.0)))
        );
    }

    #[test]
    fn stroke_bound_conservatively_expands_centerline_geometry() {
        let path = ScenePath::new(
            vec![
                PathVerb::MoveTo(LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!())),
                PathVerb::LineTo(LogicalPoint::new(10.0, 0.0).unwrap_or_else(|_| unreachable!())),
            ],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("test path is valid"));
        let item = scene_item(
            PaintPrimitive::Stroke {
                shape: SceneShape::path(path),
                brush: Brush::solid(Color::BLACK),
                style: StrokeStyle::new(LogicalLength::new(2.0).unwrap_or_else(|_| unreachable!())),
            },
            LogicalTransform::IDENTITY,
            Vec::new(),
        );
        let Some(PaintSceneBounds::Finite(bounds)) = scene(vec![item]).item_bounds(0) else {
            unreachable!("ordinary finite stroke must have finite conservative bounds");
        };
        assert!(bounds.x() <= -1.0);
        assert!(bounds.y() <= -1.0);
        assert!(bounds.max_x() >= 11.0);
        assert!(bounds.max_y() >= 1.0);
    }

    #[test]
    fn square_cap_bounds_diagonal_open_segment_without_underreporting() {
        let path = ScenePath::new(
            vec![
                PathVerb::MoveTo(LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!())),
                PathVerb::LineTo(LogicalPoint::new(10.0, 10.0).unwrap_or_else(|_| unreachable!())),
            ],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("test path is valid"));
        let item = scene_item(
            PaintPrimitive::Stroke {
                shape: SceneShape::path(path),
                brush: Brush::solid(Color::BLACK),
                style: StrokeStyle::new(LogicalLength::new(2.0).unwrap_or_else(|_| unreachable!()))
                    .with_cap(StrokeCap::Square)
                    .with_join(StrokeJoin::Bevel),
            },
            LogicalTransform::IDENTITY,
            Vec::new(),
        );
        let Some(PaintSceneBounds::Finite(bounds)) = scene(vec![item]).item_bounds(0) else {
            unreachable!("diagonal square-capped stroke must have finite bounds");
        };
        let square_cap_extent = 2.0_f32.sqrt();
        assert!(bounds.x() <= -square_cap_extent);
        assert!(bounds.y() <= -square_cap_extent);
        assert!(bounds.max_x() >= 10.0 + square_cap_extent);
        assert!(bounds.max_y() >= 10.0 + square_cap_extent);
    }

    #[test]
    fn zero_extent_rectangle_family_strokes_are_empty() {
        let radius = Radius::all(
            LogicalLength::new(3.0).unwrap_or_else(|_| unreachable!("test radius is valid")),
        );
        for shape in [
            SceneShape::rect(rect(4.0, 5.0, 0.0, 10.0)),
            SceneShape::rect(rect(4.0, 5.0, 10.0, 0.0)),
            SceneShape::rounded_rect(rect(4.0, 5.0, 0.0, 10.0), radius),
            SceneShape::rounded_rect(rect(4.0, 5.0, 10.0, 0.0), radius),
        ] {
            let item = scene_item(
                PaintPrimitive::Stroke {
                    shape,
                    brush: Brush::solid(Color::BLACK),
                    style: StrokeStyle::new(
                        LogicalLength::new(2.0).unwrap_or_else(|_| unreachable!()),
                    ),
                },
                LogicalTransform::IDENTITY,
                Vec::new(),
            );
            assert_eq!(
                scene(vec![item]).item_bounds(0),
                Some(PaintSceneBounds::Empty)
            );
        }
    }

    #[test]
    fn point_degenerate_path_stroke_is_empty() {
        let at = LogicalPoint::new(4.0, 5.0).unwrap_or_else(|_| unreachable!());
        let path = ScenePath::new(
            vec![PathVerb::MoveTo(at), PathVerb::LineTo(at), PathVerb::Close],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("point-degenerate segment is structurally valid"));
        let item = scene_item(
            PaintPrimitive::Stroke {
                shape: SceneShape::path(path),
                brush: Brush::solid(Color::BLACK),
                style: StrokeStyle::new(LogicalLength::new(2.0).unwrap_or_else(|_| unreachable!())),
            },
            LogicalTransform::IDENTITY,
            Vec::new(),
        );
        assert_eq!(
            scene(vec![item]).item_bounds(0),
            Some(PaintSceneBounds::Empty)
        );
    }

    #[test]
    fn zero_width_stroke_is_empty_and_never_a_hairline() {
        let item = scene_item(
            PaintPrimitive::Stroke {
                shape: SceneShape::rect(rect(0.0, 0.0, 10.0, 10.0)),
                brush: Brush::solid(Color::BLACK),
                style: StrokeStyle::new(LogicalLength::new(0.0).unwrap_or_else(|_| unreachable!()))
                    .with_cap(StrokeCap::Square),
            },
            LogicalTransform::IDENTITY,
            Vec::new(),
        );
        assert_eq!(
            scene(vec![item]).item_bounds(0),
            Some(PaintSceneBounds::Empty)
        );
    }

    #[test]
    fn zero_extent_ellipse_stroke_remains_empty_under_ellipse_authority() {
        let item = scene_item(
            PaintPrimitive::Stroke {
                shape: SceneShape::ellipse(rect(4.0, 5.0, 0.0, 10.0)),
                brush: Brush::solid(Color::BLACK),
                style: StrokeStyle::new(LogicalLength::new(2.0).unwrap_or_else(|_| unreachable!())),
            },
            LogicalTransform::IDENTITY,
            Vec::new(),
        );
        assert_eq!(
            scene(vec![item]).item_bounds(0),
            Some(PaintSceneBounds::Empty)
        );
    }

    #[test]
    fn bound_intersection_preserves_empty_top_and_zero_extent_algebra() {
        let finite = PaintSceneBounds::Finite(rect(2.0, 3.0, 4.0, 5.0));
        assert_eq!(
            intersect_bounds(PaintSceneBounds::Empty, finite),
            PaintSceneBounds::Empty
        );
        assert_eq!(
            intersect_bounds(PaintSceneBounds::Unbounded, finite),
            finite
        );
        assert_eq!(
            intersect_bounds(PaintSceneBounds::Unbounded, PaintSceneBounds::Unbounded),
            PaintSceneBounds::Unbounded
        );
        assert_eq!(
            intersect_bounds(
                PaintSceneBounds::Finite(rect(0.0, 0.0, 2.0, 2.0)),
                PaintSceneBounds::Finite(rect(3.0, 0.0, 2.0, 2.0)),
            ),
            PaintSceneBounds::Empty
        );
        assert_eq!(
            intersect_bounds(
                PaintSceneBounds::Finite(rect(0.0, 0.0, 10.0, 10.0)),
                PaintSceneBounds::Finite(rect(10.0, 2.0, 0.0, 6.0)),
            ),
            PaintSceneBounds::Finite(rect(10.0, 2.0, 0.0, 6.0))
        );
    }

    #[test]
    fn unbounded_coverage_is_narrowed_by_finite_clip() {
        let clipped = intersect_bounds(
            PaintSceneBounds::Unbounded,
            PaintSceneBounds::Finite(rect(3.0, 4.0, 5.0, 6.0)),
        );
        assert_eq!(clipped, PaintSceneBounds::Finite(rect(3.0, 4.0, 5.0, 6.0)));
    }

    #[test]
    fn singular_clip_excludes_item_coverage() {
        let singular = LogicalTransform::try_new(0.0, 0.0, 0.0, 0.0, 1.0, 2.0)
            .unwrap_or_else(|_| unreachable!("singular transform is still finite"));
        let item = scene_item(
            PaintPrimitive::Fill {
                shape: SceneShape::rect(rect(0.0, 0.0, 10.0, 10.0)),
                brush: Brush::solid(Color::BLACK),
            },
            LogicalTransform::IDENTITY,
            vec![SceneClip::new(
                SceneShape::rect(rect(0.0, 0.0, 10.0, 10.0)),
                singular,
            )],
        );
        assert_eq!(
            scene(vec![item]).item_bounds(0),
            Some(PaintSceneBounds::Empty)
        );
    }

    #[test]
    fn resolved_image_uses_union_of_runtime_destination_patches() {
        let resource = ResourceRef::new(ResourceKind::Image);
        let intrinsic = ImageIntrinsicSize::new(100, 100)
            .unwrap_or_else(|| unreachable!("test intrinsic extent is valid"));
        let image = ImagePrimitive::__runtime_resolved(
            resource,
            intrinsic,
            vec![
                ([0.0, 0.0, 50.0, 50.0], rect(2.0, 3.0, 5.0, 6.0)),
                ([50.0, 50.0, 50.0, 50.0], rect(20.0, 10.0, 4.0, 8.0)),
            ],
        )
        .unwrap_or_else(|| unreachable!("test image patches are valid"));
        let item = scene_item(
            PaintPrimitive::Image(image),
            LogicalTransform::IDENTITY,
            Vec::new(),
        );
        assert_eq!(
            scene(vec![item]).item_bounds(0),
            Some(PaintSceneBounds::Finite(rect(2.0, 3.0, 22.0, 15.0)))
        );
    }

    #[test]
    fn shaped_resource_without_retained_binding_is_unbounded_not_empty() {
        let resource = ResourceRef::new(ResourceKind::ShapedTextRun);
        let run = ShapedTextRunPrimitive::new(
            resource,
            LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!()),
            Color::BLACK,
        )
        .unwrap_or_else(|_| unreachable!("resource kind matches"));
        let item = scene_item(
            PaintPrimitive::ShapedTextRun(run),
            LogicalTransform::IDENTITY,
            Vec::new(),
        );
        assert_eq!(
            scene(vec![item]).item_bounds(0),
            Some(PaintSceneBounds::Unbounded)
        );
    }

    #[test]
    fn touching_clip_edges_preserve_zero_extent_finite_bound() {
        let item = scene_item(
            PaintPrimitive::Fill {
                shape: SceneShape::rect(rect(0.0, 0.0, 10.0, 10.0)),
                brush: Brush::solid(Color::BLACK),
            },
            LogicalTransform::IDENTITY,
            vec![SceneClip::new(
                SceneShape::path(
                    ScenePath::new(
                        vec![
                            PathVerb::MoveTo(
                                LogicalPoint::new(10.0, 2.0).unwrap_or_else(|_| unreachable!()),
                            ),
                            PathVerb::LineTo(
                                LogicalPoint::new(10.0, 8.0).unwrap_or_else(|_| unreachable!()),
                            ),
                        ],
                        PathFillRule::NonZero,
                    )
                    .unwrap_or_else(|_| unreachable!()),
                ),
                LogicalTransform::IDENTITY,
            )],
        );
        assert_eq!(
            scene(vec![item]).item_bounds(0),
            Some(PaintSceneBounds::Finite(rect(10.0, 2.0, 0.0, 6.0)))
        );
    }

    #[test]
    fn unrepresentable_item_transform_is_unbounded_without_untransformed_fallback() {
        let transform = LogicalTransform::try_new(f32::MAX, 0.0, 0.0, 1.0, 0.0, 0.0)
            .unwrap_or_else(|_| unreachable!("extreme transform remains finite"));
        let item = scene_item(
            PaintPrimitive::Fill {
                shape: SceneShape::rect(rect(0.0, 0.0, 2.0, 1.0)),
                brush: Brush::solid(Color::BLACK),
            },
            transform,
            Vec::new(),
        );
        assert_eq!(
            scene(vec![item]).item_bounds(0),
            Some(PaintSceneBounds::Unbounded)
        );
    }
}
