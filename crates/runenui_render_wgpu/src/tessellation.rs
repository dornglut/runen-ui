//! Disposable CPU tessellation for renderer realization.
//!
//! The module deliberately stops at validated CPU geometry. Lyon paths and
//! tessellator state are temporary implementation details; they do not become
//! scene, publication, hit, bounds, cache, or GPU authority.

use core::{error::Error, fmt};

use lyon_tessellation::{
    FillOptions, FillRule, FillTessellator, LineCap, LineJoin, StrokeOptions, StrokeTessellator,
    geom::{Angle, Arc},
    geometry_builder::{BuffersBuilder, Positions, VertexBuffers},
    math::{Point, point, vector},
    path::{NO_ATTRIBUTES, Path, builder::PathBuilder},
};
use runenui_core::{
    LogicalPoint, LogicalRect, PathFillRule, PathVerb, ScenePath, SceneShape, StrokeCap,
    StrokeJoin, StrokeStyle,
};

const TESSELLATION_TOLERANCE: f32 = 0.05;
const HALF_PI: f32 = core::f32::consts::FRAC_PI_2;

type Tangent = [f64; 2];
type EndpointTangents = (Tangent, Tangent);

/// Failure from the private neutral-to-Lyon realization adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TessellationError {
    /// An intermediate point or generated vertex was not finite.
    NonFiniteGeometry,
    /// Lyon rejected the path or tessellation parameters.
    Lyon,
    /// The returned buffers did not satisfy the renderer substrate contract.
    InvalidGeometry,
}

impl fmt::Display for TessellationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFiniteGeometry => "tessellation produced non-finite geometry",
            Self::Lyon => "Lyon rejected the disposable tessellation input",
            Self::InvalidGeometry => "tessellation produced invalid indexed geometry",
        })
    }
}

impl Error for TessellationError {}

/// Disposable indexed CPU geometry. It is intentionally not a renderer cache
/// or a logical scene representation.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TessellatedGeometry {
    positions: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl TessellatedGeometry {
    const fn empty() -> Self {
        Self {
            positions: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub(crate) const fn positions(&self) -> &[[f32; 2]] {
        self.positions.as_slice()
    }

    pub(crate) const fn indices(&self) -> &[u32] {
        self.indices.as_slice()
    }
}

#[derive(Clone, Copy)]
enum ContourMode {
    Fill,
    Stroke,
}

pub(crate) fn tessellate_fill(
    shape: &SceneShape,
) -> Result<TessellatedGeometry, TessellationError> {
    let Some(path) = disposable_path(shape, ContourMode::Fill)? else {
        return Ok(TessellatedGeometry::empty());
    };

    let fill_rule = match shape {
        SceneShape::Path(path) => path_fill_rule(path.fill_rule()),
        SceneShape::Rect(_) | SceneShape::RoundedRect { .. } | SceneShape::Ellipse(_) => {
            FillRule::NonZero
        }
    };
    let options = fill_options(fill_rule);
    let mut buffers = VertexBuffers::<Point, u32>::new();
    let mut output = BuffersBuilder::new(&mut buffers, Positions);
    FillTessellator::new()
        .tessellate_path(&path, &options, &mut output)
        .map_err(|_| TessellationError::Lyon)?;
    validate_buffers(buffers)
}

pub(crate) fn tessellate_stroke(
    shape: &SceneShape,
    style: StrokeStyle,
) -> Result<TessellatedGeometry, TessellationError> {
    if style.width().get() == 0.0 {
        return Ok(TessellatedGeometry::empty());
    }
    let Some(path) = disposable_path(shape, ContourMode::Stroke)? else {
        return Ok(TessellatedGeometry::empty());
    };

    let options = stroke_options(style);
    let mut buffers = VertexBuffers::<Point, u32>::new();
    let mut output = BuffersBuilder::new(&mut buffers, Positions);
    StrokeTessellator::new()
        .tessellate_path(&path, &options, &mut output)
        .map_err(|_| TessellationError::Lyon)?;
    validate_buffers(buffers)
}

fn fill_options(fill_rule: FillRule) -> FillOptions {
    FillOptions::tolerance(TESSELLATION_TOLERANCE)
        .with_fill_rule(fill_rule)
        .with_sweep_orientation(lyon_tessellation::Orientation::Vertical)
        .with_intersections(true)
}

fn stroke_options(style: StrokeStyle) -> StrokeOptions {
    StrokeOptions::tolerance(TESSELLATION_TOLERANCE)
        .with_start_cap(stroke_cap(style.cap()))
        .with_end_cap(stroke_cap(style.cap()))
        .with_line_join(stroke_join(style.join()))
        .with_line_width(style.width().get())
        .with_miter_limit(style.miter_limit())
}

const fn path_fill_rule(rule: PathFillRule) -> FillRule {
    match rule {
        PathFillRule::NonZero => FillRule::NonZero,
        PathFillRule::EvenOdd => FillRule::EvenOdd,
    }
}

const fn stroke_cap(cap: StrokeCap) -> LineCap {
    match cap {
        StrokeCap::Butt => LineCap::Butt,
        StrokeCap::Round => LineCap::Round,
        StrokeCap::Square => LineCap::Square,
    }
}

const fn stroke_join(join: StrokeJoin) -> LineJoin {
    match join {
        StrokeJoin::Miter => LineJoin::Miter,
        StrokeJoin::Bevel => LineJoin::Bevel,
        StrokeJoin::Round => LineJoin::Round,
    }
}

fn disposable_path(
    shape: &SceneShape,
    mode: ContourMode,
) -> Result<Option<Path>, TessellationError> {
    match shape {
        SceneShape::Rect(rect) => {
            if is_degenerate(*rect) {
                Ok(None)
            } else {
                build_rect_path(*rect).map(Some)
            }
        }
        SceneShape::RoundedRect { rect, .. } => {
            if is_degenerate(*rect) {
                Ok(None)
            } else {
                build_rounded_rect_path(shape, *rect).map(Some)
            }
        }
        SceneShape::Ellipse(rect) => {
            if is_degenerate(*rect) {
                Ok(None)
            } else {
                build_ellipse_path(*rect).map(Some)
            }
        }
        SceneShape::Path(path) => {
            if path.is_coverage_empty() {
                Ok(None)
            } else {
                build_scene_path(path, mode).map(Some)
            }
        }
    }
}

fn build_rect_path(rect: LogicalRect) -> Result<Path, TessellationError> {
    let left = rect.x();
    let top = rect.y();
    let right = rect.max_x();
    let bottom = rect.max_y();
    let mut builder = Path::builder();
    builder.begin(finite_point(left, top)?);
    builder.line_to(finite_point(right, top)?);
    builder.line_to(finite_point(right, bottom)?);
    builder.line_to(finite_point(left, bottom)?);
    builder.end(true);
    Ok(builder.build())
}

fn build_rounded_rect_path(
    shape: &SceneShape,
    rect: LogicalRect,
) -> Result<Path, TessellationError> {
    let radius = shape
        .normalized_radius()
        .ok_or(TessellationError::NonFiniteGeometry)?;
    let left = rect.x();
    let top = rect.y();
    let right = rect.max_x();
    let bottom = rect.max_y();
    let top_left = radius.top_left().get();
    let top_right = radius.top_right().get();
    let bottom_right = radius.bottom_right().get();
    let bottom_left = radius.bottom_left().get();

    let mut builder = Path::builder();
    builder.begin(finite_point(left + top_left, top)?);
    builder.line_to(finite_point(right - top_right, top)?);
    append_arc(
        &mut builder,
        point(right - top_right, top + top_right),
        vector(top_right, top_right),
        -HALF_PI,
        HALF_PI,
    )?;
    builder.line_to(finite_point(right, bottom - bottom_right)?);
    append_arc(
        &mut builder,
        point(right - bottom_right, bottom - bottom_right),
        vector(bottom_right, bottom_right),
        0.0,
        HALF_PI,
    )?;
    builder.line_to(finite_point(left + bottom_left, bottom)?);
    append_arc(
        &mut builder,
        point(left + bottom_left, bottom - bottom_left),
        vector(bottom_left, bottom_left),
        HALF_PI,
        HALF_PI,
    )?;
    builder.line_to(finite_point(left, top + top_left)?);
    append_arc(
        &mut builder,
        point(left + top_left, top + top_left),
        vector(top_left, top_left),
        core::f32::consts::PI,
        HALF_PI,
    )?;
    builder.end(true);
    Ok(builder.build())
}

fn build_ellipse_path(rect: LogicalRect) -> Result<Path, TessellationError> {
    let center = point(
        rect.width().mul_add(0.5, rect.x()),
        rect.height().mul_add(0.5, rect.y()),
    );
    let radii = vector(rect.width() * 0.5, rect.height() * 0.5);
    let mut builder = Path::builder();
    let start = point(center.x + radii.x, center.y);
    builder.begin(finite_point(start.x, start.y)?);
    for start_angle in [0.0, HALF_PI, core::f32::consts::PI, 3.0 * HALF_PI] {
        append_arc(&mut builder, center, radii, start_angle, HALF_PI)?;
    }
    builder.end(true);
    Ok(builder.build())
}

fn build_scene_path(scene_path: &ScenePath, mode: ContourMode) -> Result<Path, TessellationError> {
    let mut builder = Path::builder();
    let mut contour_start = None;
    let mut current = None;
    let mut active = false;

    for verb in scene_path.verbs() {
        match *verb {
            PathVerb::MoveTo(to) => {
                if active {
                    builder.end(matches!(mode, ContourMode::Fill));
                }
                contour_start = Some(to);
                current = Some(to);
                active = false;
            }
            PathVerb::LineTo(to) => {
                let from = current.ok_or(TessellationError::Lyon)?;
                if line_endpoint_tangents(from, to).is_some() {
                    begin_geometric_contour(&mut builder, &mut active, from)?;
                    builder.line_to(finite_point_from_logical(to)?);
                }
                current = Some(to);
            }
            PathVerb::QuadraticTo { control, to } => {
                let from = current.ok_or(TessellationError::Lyon)?;
                if quadratic_endpoint_tangents(from, control, to).is_some() {
                    begin_geometric_contour(&mut builder, &mut active, from)?;
                    builder.quadratic_bezier_to(
                        finite_point_from_logical(control)?,
                        finite_point_from_logical(to)?,
                    );
                }
                current = Some(to);
            }
            PathVerb::CubicTo {
                control1,
                control2,
                to,
            } => {
                let from = current.ok_or(TessellationError::Lyon)?;
                if cubic_endpoint_tangents(from, control1, control2, to).is_some() {
                    begin_geometric_contour(&mut builder, &mut active, from)?;
                    builder.cubic_bezier_to(
                        finite_point_from_logical(control1)?,
                        finite_point_from_logical(control2)?,
                        finite_point_from_logical(to)?,
                    );
                }
                current = Some(to);
            }
            PathVerb::Close => {
                let start = contour_start.ok_or(TessellationError::Lyon)?;
                current.ok_or(TessellationError::Lyon)?;
                if active {
                    builder.end(true);
                    active = false;
                }
                current = Some(start);
            }
        }
    }

    if active {
        builder.end(matches!(mode, ContourMode::Fill));
    }
    Ok(builder.build())
}

fn begin_geometric_contour(
    builder: &mut impl PathBuilder,
    active: &mut bool,
    from: LogicalPoint,
) -> Result<(), TessellationError> {
    if !*active {
        builder.begin(finite_point_from_logical(from)?, NO_ATTRIBUTES);
        *active = true;
    }
    Ok(())
}

fn line_endpoint_tangents(from: LogicalPoint, to: LogicalPoint) -> Option<EndpointTangents> {
    let tangent = chord(from, to)?;
    Some((tangent, tangent))
}

fn quadratic_endpoint_tangents(
    from: LogicalPoint,
    control: LogicalPoint,
    to: LogicalPoint,
) -> Option<EndpointTangents> {
    let start = chord(from, control).or_else(|| chord(from, to))?;
    let end = chord(control, to).or_else(|| chord(from, to))?;
    Some((start, end))
}

fn cubic_endpoint_tangents(
    from: LogicalPoint,
    control1: LogicalPoint,
    control2: LogicalPoint,
    to: LogicalPoint,
) -> Option<EndpointTangents> {
    let start = chord(from, control1)
        .or_else(|| chord(from, control2))
        .or_else(|| chord(from, to))?;
    let end = chord(control2, to)
        .or_else(|| chord(control1, to))
        .or_else(|| chord(from, to))?;
    Some((start, end))
}

fn chord(from: LogicalPoint, to: LogicalPoint) -> Option<Tangent> {
    let tangent = [
        f64::from(to.x()) - f64::from(from.x()),
        f64::from(to.y()) - f64::from(from.y()),
    ];
    if tangent == [0.0, 0.0] {
        None
    } else {
        Some(tangent)
    }
}

fn append_arc(
    builder: &mut impl PathBuilder,
    center: Point,
    radii: lyon_tessellation::math::Vector,
    start_angle: f32,
    sweep_angle: f32,
) -> Result<(), TessellationError> {
    if ![
        center.x,
        center.y,
        radii.x,
        radii.y,
        start_angle,
        sweep_angle,
    ]
    .into_iter()
    .all(f32::is_finite)
    {
        return Err(TessellationError::NonFiniteGeometry);
    }
    if radii.x == 0.0 || radii.y == 0.0 {
        return Ok(());
    }
    let arc = Arc {
        center,
        radii,
        start_angle: Angle::radians(start_angle),
        sweep_angle: Angle::radians(sweep_angle),
        x_rotation: Angle::zero(),
    };
    let mut invalid = false;
    arc.for_each_cubic_bezier(&mut |curve| {
        if [
            curve.ctrl1.x,
            curve.ctrl1.y,
            curve.ctrl2.x,
            curve.ctrl2.y,
            curve.to.x,
            curve.to.y,
        ]
        .into_iter()
        .all(f32::is_finite)
        {
            builder.cubic_bezier_to(curve.ctrl1, curve.ctrl2, curve.to, NO_ATTRIBUTES);
        } else {
            invalid = true;
        }
    });
    if invalid {
        Err(TessellationError::NonFiniteGeometry)
    } else {
        Ok(())
    }
}

fn is_degenerate(rect: LogicalRect) -> bool {
    rect.width() == 0.0 || rect.height() == 0.0
}

fn finite_point(x: f32, y: f32) -> Result<Point, TessellationError> {
    if x.is_finite() && y.is_finite() {
        Ok(point(x, y))
    } else {
        Err(TessellationError::NonFiniteGeometry)
    }
}

fn finite_point_from_logical(point: LogicalPoint) -> Result<Point, TessellationError> {
    finite_point(point.x(), point.y())
}

fn validate_buffers(
    buffers: VertexBuffers<Point, u32>,
) -> Result<TessellatedGeometry, TessellationError> {
    if !buffers.indices.len().is_multiple_of(3) {
        return Err(TessellationError::InvalidGeometry);
    }
    let positions = buffers
        .vertices
        .into_iter()
        .map(|position| {
            if position.x.is_finite() && position.y.is_finite() {
                Ok([position.x, position.y])
            } else {
                Err(TessellationError::NonFiniteGeometry)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if buffers
        .indices
        .iter()
        .any(|index| usize::try_from(*index).map_or(true, |index| index >= positions.len()))
    {
        return Err(TessellationError::InvalidGeometry);
    }
    Ok(TessellatedGeometry {
        positions,
        indices: buffers.indices,
    })
}

#[cfg(test)]
mod tests {
    use lyon_tessellation::path::Event;
    use runenui_core::{
        LogicalLength, LogicalPoint, LogicalRect, PathFillRule, PathVerb, Radius, ScenePath,
        SceneShape, StrokeCap, StrokeJoin, StrokeStyle,
    };

    use super::{
        ContourMode, TessellatedGeometry, build_scene_path, cubic_endpoint_tangents, fill_options,
        line_endpoint_tangents, path_fill_rule, quadratic_endpoint_tangents, stroke_cap,
        stroke_join, stroke_options, tessellate_fill, tessellate_stroke,
    };

    fn rect(width: f32, height: f32) -> LogicalRect {
        LogicalRect::try_new(0.0, 0.0, width, height)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"))
    }

    fn length(value: f32) -> LogicalLength {
        LogicalLength::new(value).unwrap_or_else(|_| unreachable!("test length is valid"))
    }

    fn point(x: f32, y: f32) -> LogicalPoint {
        LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is valid"))
    }

    fn path(verbs: Vec<PathVerb>, fill_rule: PathFillRule) -> SceneShape {
        SceneShape::path(
            ScenePath::new(verbs, fill_rule)
                .unwrap_or_else(|_| unreachable!("test path is structurally valid")),
        )
    }

    fn assert_valid_geometry(geometry: &TessellatedGeometry) {
        assert_eq!(geometry.indices.len() % 3, 0);
        assert!(
            geometry
                .positions
                .iter()
                .flatten()
                .all(|value| value.is_finite())
        );
        assert!(geometry.indices.iter().all(|index| {
            usize::try_from(*index).is_ok_and(|index| index < geometry.positions.len())
        }));
    }

    #[test]
    fn fill_output_is_deterministic_and_valid() {
        let shape = SceneShape::rect(rect(10.0, 6.0));
        let first =
            tessellate_fill(&shape).unwrap_or_else(|_| unreachable!("rect fill tessellates"));
        let second =
            tessellate_fill(&shape).unwrap_or_else(|_| unreachable!("rect fill tessellates"));
        assert_eq!(first, second);
        assert!(!first.positions.is_empty());
        assert_valid_geometry(&first);
    }

    #[test]
    fn positive_rounded_rect_and_ellipse_fills_are_non_empty() {
        let rounded = SceneShape::rounded_rect(
            rect(20.0, 12.0),
            Radius::new(length(14.0), length(14.0), length(14.0), length(0.0)),
        );
        let ellipse = SceneShape::ellipse(rect(20.0, 12.0));
        let rounded_geometry = tessellate_fill(&rounded)
            .unwrap_or_else(|_| unreachable!("rounded rectangle fill tessellates"));
        let ellipse_geometry =
            tessellate_fill(&ellipse).unwrap_or_else(|_| unreachable!("ellipse fill tessellates"));
        assert!(!rounded_geometry.positions.is_empty());
        assert!(!ellipse_geometry.positions.is_empty());
        assert_valid_geometry(&rounded_geometry);
    }

    #[test]
    fn zero_extent_ellipse_fill_is_empty() {
        let shape = SceneShape::ellipse(rect(0.0, 10.0));
        assert_eq!(
            tessellate_fill(&shape)
                .unwrap_or_else(|_| unreachable!("degenerate ellipse is handled")),
            TessellatedGeometry::empty()
        );
    }

    #[test]
    fn zero_extent_rectangle_family_strokes_are_empty() {
        let radius = Radius::all(length(3.0));
        for shape in [
            SceneShape::rect(rect(0.0, 10.0)),
            SceneShape::rect(rect(10.0, 0.0)),
            SceneShape::rounded_rect(rect(0.0, 10.0), radius),
            SceneShape::rounded_rect(rect(10.0, 0.0), radius),
        ] {
            assert_eq!(
                tessellate_stroke(&shape, StrokeStyle::new(length(2.0)))
                    .unwrap_or_else(|_| unreachable!("degenerate stroke is handled")),
                TessellatedGeometry::empty()
            );
        }
    }

    #[test]
    fn path_with_line_quadratic_and_cubic_tessellates() {
        let shape = path(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::LineTo(point(20.0, 0.0)),
                PathVerb::QuadraticTo {
                    control: point(25.0, 5.0),
                    to: point(20.0, 10.0),
                },
                PathVerb::CubicTo {
                    control1: point(15.0, 15.0),
                    control2: point(5.0, 15.0),
                    to: point(0.0, 10.0),
                },
            ],
            PathFillRule::NonZero,
        );
        let fill =
            tessellate_fill(&shape).unwrap_or_else(|_| unreachable!("path fill tessellates"));
        let stroke = tessellate_stroke(&shape, StrokeStyle::new(length(2.0)))
            .unwrap_or_else(|_| unreachable!("path stroke tessellates"));
        assert!(!fill.positions.is_empty());
        assert!(!stroke.positions.is_empty());
        assert_valid_geometry(&fill);
        assert_valid_geometry(&stroke);
    }

    #[test]
    fn entirely_point_degenerate_structural_contour_is_empty() {
        let at = point(4.0, 5.0);
        let shape = path(
            vec![
                PathVerb::MoveTo(at),
                PathVerb::LineTo(at),
                PathVerb::QuadraticTo {
                    control: at,
                    to: at,
                },
                PathVerb::CubicTo {
                    control1: at,
                    control2: at,
                    to: at,
                },
                PathVerb::Close,
            ],
            PathFillRule::NonZero,
        );
        assert_eq!(
            tessellate_fill(&shape).unwrap_or_else(|_| unreachable!("empty fill is handled")),
            TessellatedGeometry::empty()
        );
        assert_eq!(
            tessellate_stroke(&shape, StrokeStyle::new(length(2.0)))
                .unwrap_or_else(|_| unreachable!("empty stroke is handled")),
            TessellatedGeometry::empty()
        );
    }

    #[test]
    fn point_degenerate_segments_are_skipped_without_changing_contour_closure() {
        let open = ScenePath::new(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::LineTo(point(0.0, 0.0)),
                PathVerb::LineTo(point(10.0, 0.0)),
                PathVerb::QuadraticTo {
                    control: point(10.0, 0.0),
                    to: point(10.0, 0.0),
                },
                PathVerb::LineTo(point(10.0, 10.0)),
            ],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("test path is valid"));
        let open_events = build_scene_path(&open, ContourMode::Stroke)
            .unwrap_or_else(|_| unreachable!("open path converts"))
            .iter()
            .collect::<Vec<_>>();
        assert_eq!(open_events.len(), 4);
        assert!(matches!(open_events.first(), Some(Event::Begin { .. })));
        assert!(matches!(open_events[1], Event::Line { .. }));
        assert!(matches!(open_events[2], Event::Line { .. }));
        assert!(matches!(
            open_events.last(),
            Some(Event::End { close: false, .. })
        ));

        let closed = ScenePath::new(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::LineTo(point(0.0, 0.0)),
                PathVerb::LineTo(point(10.0, 0.0)),
                PathVerb::LineTo(point(0.0, 0.0)),
                PathVerb::Close,
            ],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("test path is valid"));
        let closed_events = build_scene_path(&closed, ContourMode::Stroke)
            .unwrap_or_else(|_| unreachable!("closed path converts"))
            .iter()
            .collect::<Vec<_>>();
        assert_eq!(closed_events.len(), 4);
        assert!(matches!(
            closed_events.last(),
            Some(Event::End { close: true, .. })
        ));
    }

    #[test]
    fn limiting_endpoint_tangents_follow_polynomial_control_order() {
        let origin = point(0.0, 0.0);
        assert_eq!(
            line_endpoint_tangents(origin, point(2.0, 3.0)),
            Some(([2.0, 3.0], [2.0, 3.0]))
        );
        assert_eq!(
            quadratic_endpoint_tangents(origin, origin, point(10.0, 0.0)),
            Some(([10.0, 0.0], [10.0, 0.0]))
        );
        assert_eq!(
            quadratic_endpoint_tangents(origin, point(10.0, 0.0), point(10.0, 0.0)),
            Some(([10.0, 0.0], [10.0, 0.0]))
        );
        assert_eq!(
            cubic_endpoint_tangents(origin, origin, point(0.0, 10.0), point(10.0, 10.0),),
            Some(([0.0, 10.0], [10.0, 0.0]))
        );
        assert_eq!(
            cubic_endpoint_tangents(
                origin,
                point(0.0, 10.0),
                point(10.0, 10.0),
                point(10.0, 10.0),
            ),
            Some(([0.0, 10.0], [10.0, 0.0]))
        );
        assert_eq!(
            cubic_endpoint_tangents(origin, point(10.0, 0.0), point(10.0, 0.0), point(10.0, 0.0),),
            Some(([10.0, 0.0], [10.0, 0.0]))
        );
        assert_eq!(
            cubic_endpoint_tangents(origin, origin, origin, origin),
            None
        );
    }

    #[test]
    fn zero_derivative_bezier_endpoint_caps_follow_limiting_tangents() {
        let horizontal_cases = [
            path(
                vec![
                    PathVerb::MoveTo(point(0.0, 0.0)),
                    PathVerb::QuadraticTo {
                        control: point(0.0, 0.0),
                        to: point(10.0, 0.0),
                    },
                ],
                PathFillRule::NonZero,
            ),
            path(
                vec![
                    PathVerb::MoveTo(point(0.0, 0.0)),
                    PathVerb::QuadraticTo {
                        control: point(10.0, 0.0),
                        to: point(10.0, 0.0),
                    },
                ],
                PathFillRule::NonZero,
            ),
            path(
                vec![
                    PathVerb::MoveTo(point(0.0, 0.0)),
                    PathVerb::CubicTo {
                        control1: point(0.0, 0.0),
                        control2: point(10.0, 0.0),
                        to: point(10.0, 0.0),
                    },
                ],
                PathFillRule::NonZero,
            ),
        ];
        for shape in horizontal_cases {
            let geometry = tessellate_stroke(
                &shape,
                StrokeStyle::new(length(2.0)).with_cap(StrokeCap::Square),
            )
            .unwrap_or_else(|_| unreachable!("Bezier stroke tessellates"));
            assert_valid_geometry(&geometry);
            let min_x = geometry
                .positions
                .iter()
                .map(|position| position[0])
                .fold(f32::INFINITY, f32::min);
            let max_x = geometry
                .positions
                .iter()
                .map(|position| position[0])
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(min_x <= -0.9, "start cap must follow limiting tangent");
            assert!(max_x >= 10.9, "end cap must follow limiting tangent");
        }

        let curved_cubic = path(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::CubicTo {
                    control1: point(0.0, 0.0),
                    control2: point(0.0, 10.0),
                    to: point(10.0, 10.0),
                },
            ],
            PathFillRule::NonZero,
        );
        let geometry = tessellate_stroke(
            &curved_cubic,
            StrokeStyle::new(length(2.0)).with_cap(StrokeCap::Square),
        )
        .unwrap_or_else(|_| unreachable!("curve stroke tessellates"));
        assert_valid_geometry(&geometry);
        let min_y = geometry
            .positions
            .iter()
            .map(|position| position[1])
            .fold(f32::INFINITY, f32::min);
        let max_x = geometry
            .positions
            .iter()
            .map(|position| position[0])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            min_y <= -0.9,
            "start cap must extend against vertical limiting tangent"
        );
        assert!(
            max_x >= 10.9,
            "end cap must extend along horizontal limiting tangent"
        );
    }

    #[test]
    fn miter_limit_falls_back_to_bevel_for_over_limit_join() {
        // At the join, the incoming ray points left and the outgoing segment is
        // (-2, 1). The enclosed angle is about 26.565 degrees, so the accepted
        // miter-length / full-stroke-width ratio is about 2.176: limit 1 must
        // bevel while limit 4 must retain the finite miter.
        let shape = path(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::LineTo(point(10.0, 0.0)),
                PathVerb::LineTo(point(8.0, 1.0)),
            ],
            PathFillRule::NonZero,
        );
        let limited = tessellate_stroke(
            &shape,
            StrokeStyle::new(length(2.0))
                .with_join(StrokeJoin::Miter)
                .with_miter_limit(1.0)
                .unwrap_or_else(|_| unreachable!("miter limit is valid")),
        )
        .unwrap_or_else(|_| unreachable!("limited miter tessellates"));
        let bevel = tessellate_stroke(
            &shape,
            StrokeStyle::new(length(2.0)).with_join(StrokeJoin::Bevel),
        )
        .unwrap_or_else(|_| unreachable!("bevel tessellates"));
        let generous = tessellate_stroke(
            &shape,
            StrokeStyle::new(length(2.0))
                .with_join(StrokeJoin::Miter)
                .with_miter_limit(4.0)
                .unwrap_or_else(|_| unreachable!("miter limit is valid")),
        )
        .unwrap_or_else(|_| unreachable!("generous miter tessellates"));
        for geometry in [&limited, &bevel, &generous] {
            assert_valid_geometry(geometry);
        }

        let max_x = |geometry: &TessellatedGeometry| {
            geometry
                .positions
                .iter()
                .map(|position| position[0])
                .fold(f32::NEG_INFINITY, f32::max)
        };
        let limited_extent = max_x(&limited);
        let bevel_extent = max_x(&bevel);
        let generous_extent = max_x(&generous);

        assert_eq!(limited_extent.to_bits(), bevel_extent.to_bits());
        assert!(limited_extent < 10.5);
        assert!(generous_extent > 14.0);
    }

    #[test]
    fn nondegenerate_one_dimensional_curve_stroke_remains_real_geometry() {
        let shape = path(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::QuadraticTo {
                    control: point(0.0, 0.0),
                    to: point(10.0, 0.0),
                },
            ],
            PathFillRule::NonZero,
        );
        let geometry = tessellate_stroke(
            &shape,
            StrokeStyle::new(length(2.0)).with_cap(StrokeCap::Square),
        )
        .unwrap_or_else(|_| unreachable!("one-dimensional curve tessellates"));
        assert!(!geometry.positions.is_empty());
        assert_valid_geometry(&geometry);
    }

    #[test]
    fn fill_rule_mapping_is_explicit() {
        assert_eq!(
            path_fill_rule(PathFillRule::NonZero),
            lyon_tessellation::FillRule::NonZero
        );
        assert_eq!(
            path_fill_rule(PathFillRule::EvenOdd),
            lyon_tessellation::FillRule::EvenOdd
        );
        assert_eq!(
            fill_options(lyon_tessellation::FillRule::NonZero).fill_rule,
            lyon_tessellation::FillRule::NonZero
        );
        assert_eq!(
            fill_options(lyon_tessellation::FillRule::EvenOdd).fill_rule,
            lyon_tessellation::FillRule::EvenOdd
        );
    }

    #[test]
    fn fill_closes_open_contours_but_stroke_does_not() {
        let shape = path(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::LineTo(point(10.0, 0.0)),
                PathVerb::LineTo(point(10.0, 10.0)),
            ],
            PathFillRule::NonZero,
        );
        let fill_events = build_scene_path(
            match &shape {
                SceneShape::Path(path) => path,
                _ => unreachable!("test shape is a path"),
            },
            ContourMode::Fill,
        )
        .unwrap_or_else(|_| unreachable!("fill path converts"))
        .iter()
        .collect::<Vec<_>>();
        let stroke_events = build_scene_path(
            match &shape {
                SceneShape::Path(path) => path,
                _ => unreachable!("test shape is a path"),
            },
            ContourMode::Stroke,
        )
        .unwrap_or_else(|_| unreachable!("stroke path converts"))
        .iter()
        .collect::<Vec<_>>();
        assert!(matches!(
            fill_events.last(),
            Some(Event::End { close: true, .. })
        ));
        assert!(matches!(
            stroke_events.last(),
            Some(Event::End { close: false, .. })
        ));
    }

    #[test]
    fn authored_close_remains_a_stroke_close() {
        let shape = path(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::LineTo(point(10.0, 0.0)),
                PathVerb::LineTo(point(10.0, 10.0)),
                PathVerb::Close,
            ],
            PathFillRule::NonZero,
        );
        let events = build_scene_path(
            match &shape {
                SceneShape::Path(path) => path,
                _ => unreachable!("test shape is a path"),
            },
            ContourMode::Stroke,
        )
        .unwrap_or_else(|_| unreachable!("authored close converts"))
        .iter()
        .collect::<Vec<_>>();
        assert!(matches!(
            events.last(),
            Some(Event::End { close: true, .. })
        ));
    }

    #[test]
    fn stroke_options_map_caps_joins_width_and_miter_limit_explicitly() {
        let style = StrokeStyle::new(length(3.0))
            .with_cap(StrokeCap::Square)
            .with_join(StrokeJoin::Round)
            .with_miter_limit(2.5)
            .unwrap_or_else(|_| unreachable!("test miter limit is valid"));
        let options = stroke_options(style);
        assert_eq!(options.start_cap, lyon_tessellation::LineCap::Square);
        assert_eq!(options.end_cap, lyon_tessellation::LineCap::Square);
        assert_eq!(options.line_join, lyon_tessellation::LineJoin::Round);
        assert_eq!(options.line_width.to_bits(), 3.0_f32.to_bits());
        assert_eq!(options.miter_limit.to_bits(), 2.5_f32.to_bits());
        assert_eq!(
            options.tolerance.to_bits(),
            super::TESSELLATION_TOLERANCE.to_bits()
        );
        assert_eq!(
            stroke_cap(StrokeCap::Butt),
            lyon_tessellation::LineCap::Butt
        );
        assert_eq!(
            stroke_cap(StrokeCap::Round),
            lyon_tessellation::LineCap::Round
        );
        assert_eq!(
            stroke_join(StrokeJoin::Miter),
            lyon_tessellation::LineJoin::Miter
        );
        assert_eq!(
            stroke_join(StrokeJoin::Bevel),
            lyon_tessellation::LineJoin::Bevel
        );
    }

    #[test]
    fn zero_width_and_move_only_path_are_empty() {
        let rectangle = SceneShape::rect(rect(10.0, 10.0));
        assert_eq!(
            tessellate_stroke(&rectangle, StrokeStyle::new(length(0.0)))
                .unwrap_or_else(|_| unreachable!("zero width is handled")),
            TessellatedGeometry::empty()
        );
        let move_only = path(
            vec![PathVerb::MoveTo(point(1.0, 1.0))],
            PathFillRule::NonZero,
        );
        assert_eq!(
            tessellate_fill(&move_only).unwrap_or_else(|_| unreachable!("move-only is handled")),
            TessellatedGeometry::empty()
        );
        assert_eq!(
            tessellate_stroke(&move_only, StrokeStyle::new(length(2.0)))
                .unwrap_or_else(|_| unreachable!("move-only is handled")),
            TessellatedGeometry::empty()
        );
    }
}
