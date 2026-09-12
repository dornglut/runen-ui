//! RunenUI-owned structural path vocabulary.

use core::{error::Error, fmt};
use std::sync::Arc;

use crate::{LogicalPoint, LogicalRect};

/// Fill rule applied to logically closed path contours.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PathFillRule {
    /// Non-zero winding fill.
    #[default]
    NonZero,
    /// Even-odd parity fill.
    EvenOdd,
}

/// One immutable authored path verb.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathVerb {
    /// Starts a new contour at the finite point.
    MoveTo(LogicalPoint),
    /// Adds one straight segment to the finite point.
    LineTo(LogicalPoint),
    /// Adds one quadratic Bézier segment.
    QuadraticTo {
        /// Quadratic control point.
        control: LogicalPoint,
        /// Segment endpoint.
        to: LogicalPoint,
    },
    /// Adds one cubic Bézier segment.
    CubicTo {
        /// First cubic control point.
        control1: LogicalPoint,
        /// Second cubic control point.
        control2: LogicalPoint,
        /// Segment endpoint.
        to: LogicalPoint,
    },
    /// Explicitly closes the current segment-bearing contour.
    Close,
}

/// Validation failure for one structural path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenePathError {
    /// A segment was authored before the first `move`.
    SegmentWithoutContour,
    /// `close` was authored before the contour had a segment.
    CloseWithoutSegment,
    /// The current contour was already explicitly closed.
    AlreadyClosed,
    /// A segment followed an explicit close without a new move.
    SegmentAfterClose,
    /// Segment-bearing path bounds cannot be represented by a finite logical rectangle.
    BoundsOverflow,
}

impl fmt::Display for ScenePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SegmentWithoutContour => "path segment requires a preceding move",
            Self::CloseWithoutSegment => "path close requires a segment-bearing open contour",
            Self::AlreadyClosed => "path contour is already closed",
            Self::SegmentAfterClose => "path segment after close requires a new move",
            Self::BoundsOverflow => "path logical bounds exceed the finite logical rectangle range",
        })
    }
}

impl Error for ScenePathError {}

/// Immutable validated `RunenUI` path content.
///
/// Identity/equality is structural path content plus fill rule. Shared storage is
/// an implementation detail and never participates in scene identity. Move-only
/// contours are valid but have no coverage. Point-degenerate authored segments
/// remain structural identity/validation facts but contribute no geometry. Open
/// segment-bearing contours remain structurally open: ADR 0011's synthetic closing
/// edge exists only during fill evaluation and is never inserted into authored path
/// content.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenePath {
    verbs: Arc<[PathVerb]>,
    fill_rule: PathFillRule,
    logical_bounds: Option<LogicalRect>,
}

impl ScenePath {
    /// Validates and freezes authored path content.
    ///
    /// # Errors
    ///
    /// Returns [`ScenePathError`] for malformed contour ordering or derived
    /// logical bounds that cannot be represented as a finite [`LogicalRect`].
    pub fn new(
        verbs: impl Into<Vec<PathVerb>>,
        fill_rule: PathFillRule,
    ) -> Result<Self, ScenePathError> {
        let verbs = verbs.into();
        validate_verbs(&verbs)?;
        let logical_bounds = path_bounds(&verbs)?;
        Ok(Self {
            verbs: Arc::from(verbs),
            fill_rule,
            logical_bounds,
        })
    }

    /// Returns exact authored verbs in stable order.
    #[must_use]
    pub fn verbs(&self) -> &[PathVerb] {
        self.verbs.as_ref()
    }

    /// Returns the authored fill rule.
    #[must_use]
    pub const fn fill_rule(&self) -> PathFillRule {
        self.fill_rule
    }

    /// Returns deterministic conservative bounds when at least one authored
    /// segment contributes geometry. Empty, move-only, and entirely point-degenerate
    /// paths return `None`.
    #[must_use]
    pub const fn logical_bounds(&self) -> Option<LogicalRect> {
        self.logical_bounds
    }

    /// Returns whether no authored segment contributes geometric coverage.
    #[must_use]
    pub const fn is_coverage_empty(&self) -> bool {
        self.logical_bounds.is_none()
    }
}

fn validate_verbs(verbs: &[PathVerb]) -> Result<(), ScenePathError> {
    let mut has_contour = false;
    let mut has_segment = false;
    let mut closed = false;

    for verb in verbs {
        match verb {
            PathVerb::MoveTo(_) => {
                has_contour = true;
                has_segment = false;
                closed = false;
            }
            PathVerb::LineTo(_) | PathVerb::QuadraticTo { .. } | PathVerb::CubicTo { .. } => {
                if !has_contour {
                    return Err(ScenePathError::SegmentWithoutContour);
                }
                if closed {
                    return Err(ScenePathError::SegmentAfterClose);
                }
                has_segment = true;
            }
            PathVerb::Close => {
                if !has_contour || !has_segment {
                    return Err(ScenePathError::CloseWithoutSegment);
                }
                if closed {
                    return Err(ScenePathError::AlreadyClosed);
                }
                closed = true;
            }
        }
    }
    Ok(())
}

pub fn line_is_point_degenerate(from: LogicalPoint, to: LogicalPoint) -> bool {
    from == to
}

pub fn quadratic_is_point_degenerate(
    from: LogicalPoint,
    control: LogicalPoint,
    to: LogicalPoint,
) -> bool {
    from == control && control == to
}

pub fn cubic_is_point_degenerate(
    from: LogicalPoint,
    control1: LogicalPoint,
    control2: LogicalPoint,
    to: LogicalPoint,
) -> bool {
    from == control1 && control1 == control2 && control2 == to
}

#[derive(Clone, Copy)]
struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Bounds {
    fn point(point: LogicalPoint) -> Self {
        let x = f64::from(point.x());
        let y = f64::from(point.y());
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    const fn include(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    fn rect(self) -> Result<LogicalRect, ScenePathError> {
        let min_x = checked_f32_down(self.min_x)?;
        let min_y = checked_f32_down(self.min_y)?;
        let max_x = checked_f32_up(self.max_x)?;
        let max_y = checked_f32_up(self.max_y)?;
        let width = checked_f32_up(f64::from(max_x) - f64::from(min_x))?;
        let height = checked_f32_up(f64::from(max_y) - f64::from(min_y))?;
        LogicalRect::try_new(min_x, min_y, width, height)
            .map_err(|_| ScenePathError::BoundsOverflow)
    }
}

#[allow(clippy::cast_possible_truncation)]
fn checked_f32(value: f64) -> Result<f32, ScenePathError> {
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return Err(ScenePathError::BoundsOverflow);
    }
    Ok(value as f32)
}

fn checked_f32_down(value: f64) -> Result<f32, ScenePathError> {
    let rounded = checked_f32(value)?;
    if f64::from(rounded) > value {
        Ok(rounded.next_down())
    } else {
        Ok(rounded)
    }
}

fn checked_f32_up(value: f64) -> Result<f32, ScenePathError> {
    let rounded = checked_f32(value)?;
    if f64::from(rounded) < value {
        Ok(rounded.next_up())
    } else {
        Ok(rounded)
    }
}

fn path_bounds(verbs: &[PathVerb]) -> Result<Option<LogicalRect>, ScenePathError> {
    let mut current = None;
    let mut first = None;
    let mut bounds: Option<Bounds> = None;

    for verb in verbs {
        match *verb {
            PathVerb::MoveTo(point) => {
                current = Some(point);
                first = Some(point);
            }
            PathVerb::LineTo(to) => {
                let Some(from) = current else {
                    unreachable!("validated path segment always has a current point")
                };
                include_line(&mut bounds, from, to);
                current = Some(to);
            }
            PathVerb::QuadraticTo { control, to } => {
                let Some(from) = current else {
                    unreachable!("validated path segment always has a current point")
                };
                include_quadratic(&mut bounds, from, control, to);
                current = Some(to);
            }
            PathVerb::CubicTo {
                control1,
                control2,
                to,
            } => {
                let Some(from) = current else {
                    unreachable!("validated path segment always has a current point")
                };
                include_cubic(&mut bounds, from, control1, control2, to);
                current = Some(to);
            }
            PathVerb::Close => {
                let Some(from) = current else {
                    unreachable!("validated close always has a current point")
                };
                let Some(to) = first else {
                    unreachable!("validated close always has a contour start")
                };
                include_line(&mut bounds, from, to);
                current = Some(to);
            }
        }
    }

    bounds.map(Bounds::rect).transpose()
}

fn ensure_bounds(bounds: &mut Option<Bounds>, point: LogicalPoint) -> &mut Bounds {
    bounds.get_or_insert_with(|| Bounds::point(point))
}

fn include_line(bounds: &mut Option<Bounds>, from: LogicalPoint, to: LogicalPoint) {
    if line_is_point_degenerate(from, to) {
        return;
    }
    let bounds = ensure_bounds(bounds, from);
    bounds.include(f64::from(to.x()), f64::from(to.y()));
}

fn include_quadratic(
    bounds: &mut Option<Bounds>,
    from: LogicalPoint,
    control: LogicalPoint,
    to: LogicalPoint,
) {
    if quadratic_is_point_degenerate(from, control, to) {
        return;
    }
    include_line(bounds, from, to);
    let bounds = ensure_bounds(bounds, from);

    for axis in 0..2 {
        let (p0, p1, p2) = match axis {
            0 => (
                f64::from(from.x()),
                f64::from(control.x()),
                f64::from(to.x()),
            ),
            _ => (
                f64::from(from.y()),
                f64::from(control.y()),
                f64::from(to.y()),
            ),
        };
        let denominator = (-2.0_f64).mul_add(p1, p0) + p2;
        if denominator == 0.0 {
            continue;
        }
        let t = (p0 - p1) / denominator;
        if (0.0..1.0).contains(&t) {
            bounds.include(
                quadratic(
                    f64::from(from.x()),
                    f64::from(control.x()),
                    f64::from(to.x()),
                    t,
                ),
                quadratic(
                    f64::from(from.y()),
                    f64::from(control.y()),
                    f64::from(to.y()),
                    t,
                ),
            );
        }
    }
}

fn include_cubic(
    bounds: &mut Option<Bounds>,
    from: LogicalPoint,
    control1: LogicalPoint,
    control2: LogicalPoint,
    to: LogicalPoint,
) {
    if cubic_is_point_degenerate(from, control1, control2, to) {
        return;
    }
    include_line(bounds, from, to);
    let _ = ensure_bounds(bounds, from);

    for axis in 0..2 {
        let (p0, p1, p2, p3) = match axis {
            0 => (
                f64::from(from.x()),
                f64::from(control1.x()),
                f64::from(control2.x()),
                f64::from(to.x()),
            ),
            _ => (
                f64::from(from.y()),
                f64::from(control1.y()),
                f64::from(control2.y()),
                f64::from(to.y()),
            ),
        };
        let [a, b, c, _] = cubic_coefficients(p0, p1, p2, p3);
        let mut roots = [0.0_f64; 2];
        let count = quadratic_roots(3.0 * a, 2.0 * b, c, &mut roots);
        for &t in &roots[..count] {
            if (0.0..1.0).contains(&t) {
                ensure_bounds(bounds, from).include(
                    cubic(
                        f64::from(from.x()),
                        f64::from(control1.x()),
                        f64::from(control2.x()),
                        f64::from(to.x()),
                        t,
                    ),
                    cubic(
                        f64::from(from.y()),
                        f64::from(control1.y()),
                        f64::from(control2.y()),
                        f64::from(to.y()),
                        t,
                    ),
                );
            }
        }
    }
}

pub fn quadratic_roots(a: f64, b: f64, c: f64, roots: &mut [f64; 2]) -> usize {
    if a == 0.0 {
        if b == 0.0 {
            return 0;
        }
        roots[0] = -c / b;
        return 1;
    }

    let discriminant = b.mul_add(b, -4.0 * a * c);
    if discriminant < 0.0 {
        return 0;
    }
    if discriminant == 0.0 {
        roots[0] = -b / (2.0 * a);
        return 1;
    }

    let sqrt = discriminant.sqrt();
    let q = -0.5 * (b + sqrt.copysign(b));
    roots[0] = q / a;
    roots[1] = c / q;
    2
}

pub fn quadratic(p0: f64, p1: f64, p2: f64, t: f64) -> f64 {
    let a = (-2.0_f64).mul_add(p1, p0) + p2;
    let b = 2.0 * (p1 - p0);
    a.mul_add(t, b).mul_add(t, p0)
}

pub fn cubic_coefficients(p0: f64, p1: f64, p2: f64, p3: f64) -> [f64; 4] {
    let a = (-3.0_f64).mul_add(p2, 3.0_f64.mul_add(p1, -p0)) + p3;
    let b = 3.0 * ((-2.0_f64).mul_add(p1, p0) + p2);
    let c = 3.0 * (p1 - p0);
    [a, b, c, p0]
}

pub fn cubic(p0: f64, p1: f64, p2: f64, p3: f64, parameter: f64) -> f64 {
    let [cubic_term, quadratic_term, linear_term, constant_term] =
        cubic_coefficients(p0, p1, p2, p3);
    cubic_term
        .mul_add(parameter, quadratic_term)
        .mul_add(parameter, linear_term)
        .mul_add(parameter, constant_term)
}

#[cfg(test)]
mod tests {
    use super::{PathFillRule, PathVerb, ScenePath, ScenePathError, checked_f32, quadratic};
    use crate::LogicalPoint;

    fn point(x: f32, y: f32) -> LogicalPoint {
        LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
    }

    #[test]
    fn malformed_contours_reject_without_dependency_recovery() {
        assert_eq!(
            ScenePath::new(
                vec![PathVerb::LineTo(point(1.0, 1.0))],
                PathFillRule::NonZero,
            ),
            Err(ScenePathError::SegmentWithoutContour)
        );
        assert_eq!(
            ScenePath::new(
                vec![PathVerb::MoveTo(point(0.0, 0.0)), PathVerb::Close],
                PathFillRule::NonZero,
            ),
            Err(ScenePathError::CloseWithoutSegment)
        );
        assert_eq!(
            ScenePath::new(
                vec![
                    PathVerb::MoveTo(point(0.0, 0.0)),
                    PathVerb::LineTo(point(1.0, 0.0)),
                    PathVerb::Close,
                    PathVerb::LineTo(point(2.0, 0.0)),
                ],
                PathFillRule::NonZero,
            ),
            Err(ScenePathError::SegmentAfterClose)
        );
    }

    #[test]
    fn move_only_contours_are_valid_but_have_no_coverage() {
        let path = ScenePath::new(
            vec![
                PathVerb::MoveTo(point(20.0, 30.0)),
                PathVerb::MoveTo(point(40.0, 50.0)),
            ],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("move-only path is valid"));
        assert!(path.logical_bounds().is_none());
        assert!(path.is_coverage_empty());
    }

    #[test]
    fn point_degenerate_segments_remain_structural_but_are_coverage_empty() {
        let verbs = vec![
            PathVerb::MoveTo(point(2.0, 3.0)),
            PathVerb::LineTo(point(2.0, 3.0)),
            PathVerb::QuadraticTo {
                control: point(2.0, 3.0),
                to: point(2.0, 3.0),
            },
            PathVerb::CubicTo {
                control1: point(2.0, 3.0),
                control2: point(2.0, 3.0),
                to: point(2.0, 3.0),
            },
            PathVerb::Close,
        ];
        let path = ScenePath::new(verbs.clone(), PathFillRule::NonZero)
            .unwrap_or_else(|_| unreachable!("degenerate segments remain structurally valid"));

        assert_eq!(path.verbs(), verbs.as_slice());
        assert!(path.logical_bounds().is_none());
        assert!(path.is_coverage_empty());
    }

    #[test]
    fn nondegenerate_one_dimensional_curves_retain_geometry() {
        let path = ScenePath::new(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::QuadraticTo {
                    control: point(0.0, 0.0),
                    to: point(10.0, 0.0),
                },
                PathVerb::CubicTo {
                    control1: point(10.0, 0.0),
                    control2: point(20.0, 0.0),
                    to: point(20.0, 0.0),
                },
            ],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("one-dimensional curves remain valid geometry"));

        let bounds = path
            .logical_bounds()
            .unwrap_or_else(|| unreachable!("nondegenerate curves retain finite bounds"));
        assert_eq!(
            (
                bounds.x().to_bits(),
                bounds.y().to_bits(),
                bounds.width().to_bits(),
                bounds.height().to_bits(),
            ),
            (
                0.0_f32.to_bits(),
                0.0_f32.to_bits(),
                20.0_f32.to_bits(),
                0.0_f32.to_bits(),
            )
        );
    }

    #[test]
    fn structural_identity_does_not_depend_on_shared_allocation() {
        let verbs = vec![
            PathVerb::MoveTo(point(0.0, 0.0)),
            PathVerb::QuadraticTo {
                control: point(5.0, 10.0),
                to: point(10.0, 0.0),
            },
        ];
        let a = ScenePath::new(verbs.clone(), PathFillRule::EvenOdd)
            .unwrap_or_else(|_| unreachable!("test path is valid"));
        let b = ScenePath::new(verbs, PathFillRule::EvenOdd)
            .unwrap_or_else(|_| unreachable!("test path is valid"));
        assert_eq!(a, b);
        assert_eq!(a.verbs(), b.verbs());
    }

    #[test]
    fn tight_bounds_include_quadratic_and_cubic_extrema() {
        let quadratic_path = ScenePath::new(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::QuadraticTo {
                    control: point(5.0, 10.0),
                    to: point(10.0, 0.0),
                },
            ],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("test path is valid"));
        let quadratic_bounds = quadratic_path
            .logical_bounds()
            .unwrap_or_else(|| unreachable!("segment-bearing path has bounds"));
        assert_eq!(
            (
                quadratic_bounds.x().to_bits(),
                quadratic_bounds.y().to_bits(),
                quadratic_bounds.width().to_bits(),
                quadratic_bounds.height().to_bits(),
            ),
            (
                0.0_f32.to_bits(),
                0.0_f32.to_bits(),
                10.0_f32.to_bits(),
                5.0_f32.to_bits(),
            )
        );

        let cubic_path = ScenePath::new(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::CubicTo {
                    control1: point(0.0, 10.0),
                    control2: point(10.0, 10.0),
                    to: point(10.0, 0.0),
                },
            ],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("test cubic path is valid"));
        let cubic_bounds = cubic_path
            .logical_bounds()
            .unwrap_or_else(|| unreachable!("segment-bearing path has bounds"));
        assert_eq!(
            (
                cubic_bounds.x().to_bits(),
                cubic_bounds.y().to_bits(),
                cubic_bounds.width().to_bits(),
                cubic_bounds.height().to_bits(),
            ),
            (
                0.0_f32.to_bits(),
                0.0_f32.to_bits(),
                10.0_f32.to_bits(),
                7.5_f32.to_bits(),
            )
        );
    }

    #[test]
    fn nonrepresentable_curve_extrema_round_bounds_outward() {
        let path = ScenePath::new(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::QuadraticTo {
                    control: point(-1.0, 1.0),
                    to: point(0.3, -0.3),
                },
            ],
            PathFillRule::NonZero,
        )
        .unwrap_or_else(|_| unreachable!("test path is valid"));
        let bounds = path
            .logical_bounds()
            .unwrap_or_else(|| unreachable!("segment-bearing path has bounds"));

        let endpoint = f64::from(0.3_f32);
        let parameter = 1.0 / (2.0 + endpoint);
        let exact_min_x = quadratic(0.0, -1.0, endpoint, parameter);
        let exact_max_y = quadratic(0.0, 1.0, -endpoint, parameter);
        let nearest_min_x = checked_f32(exact_min_x)
            .unwrap_or_else(|_| unreachable!("test extremum is representable"));
        let nearest_max_y = checked_f32(exact_max_y)
            .unwrap_or_else(|_| unreachable!("test extremum is representable"));

        assert!(f64::from(nearest_min_x) > exact_min_x);
        assert!(f64::from(nearest_max_y) < exact_max_y);
        assert!(f64::from(bounds.x()) <= exact_min_x);
        assert!(f64::from(bounds.max_y()) >= exact_max_y);
    }

    #[test]
    fn unrepresentable_derived_bounds_reject_instead_of_appearing_empty() {
        assert_eq!(
            ScenePath::new(
                vec![
                    PathVerb::MoveTo(point(-f32::MAX, 0.0)),
                    PathVerb::LineTo(point(f32::MAX, 0.0)),
                ],
                PathFillRule::NonZero,
            ),
            Err(ScenePathError::BoundsOverflow)
        );
    }
}
