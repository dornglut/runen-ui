//! Tolerance-independent logical fill containment for structural `RunenUI` paths.

use core::cmp::Ordering;

use crate::{
    LogicalPoint, LogicalRect, PathFillRule, PathVerb, ScenePath,
    path::{
        cubic, cubic_coefficients, cubic_is_point_degenerate, line_is_point_degenerate, quadratic,
        quadratic_is_point_degenerate, quadratic_roots,
    },
};

impl ScenePath {
    /// Returns whether `point` belongs to the path's logical fill coverage.
    ///
    /// Segment-bearing open contours receive the ADR 0011 synthetic straight
    /// final-to-first edge for fill only. Authored and synthetic boundaries are
    /// inside. Point-degenerate authored/synthetic segments contribute no boundary
    /// or winding. Non-boundary points use the authored non-zero or even-odd rule.
    /// Renderer tessellation, raster scale, and dependency flattening tolerances
    /// are not inputs to this decision.
    #[must_use]
    pub fn contains_fill(&self, point: LogicalPoint) -> bool {
        let Some(bounds) = self.logical_bounds() else {
            return false;
        };
        if !inclusive_rect_contains(bounds, point) {
            return false;
        }

        let state = fill_state(self, point);
        if state.boundary {
            return true;
        }
        match self.fill_rule() {
            PathFillRule::NonZero => state.winding != 0,
            PathFillRule::EvenOdd => state.parity,
        }
    }
}

fn inclusive_rect_contains(rect: LogicalRect, point: LogicalPoint) -> bool {
    (rect.x()..=rect.max_x()).contains(&point.x()) && (rect.y()..=rect.max_y()).contains(&point.y())
}

fn fill_state(path: &ScenePath, point: LogicalPoint) -> FillState {
    let mut state = FillState::default();
    let mut contour_start = None;
    let mut current = None;
    let mut has_segment = false;
    let mut explicitly_closed = false;

    for verb in path.verbs() {
        match *verb {
            PathVerb::MoveTo(to) => {
                finish_open_contour(
                    &mut state,
                    current,
                    contour_start,
                    has_segment,
                    explicitly_closed,
                    point,
                );
                if state.boundary {
                    return state;
                }
                contour_start = Some(to);
                current = Some(to);
                has_segment = false;
                explicitly_closed = false;
            }
            PathVerb::LineTo(to) => {
                let Some(from) = current else {
                    unreachable!("validated path segment always has a current point")
                };
                state.include(Segment::Line { from, to }, point);
                if state.boundary {
                    return state;
                }
                current = Some(to);
                has_segment = true;
            }
            PathVerb::QuadraticTo { control, to } => {
                let Some(from) = current else {
                    unreachable!("validated path segment always has a current point")
                };
                state.include(Segment::Quadratic { from, control, to }, point);
                if state.boundary {
                    return state;
                }
                current = Some(to);
                has_segment = true;
            }
            PathVerb::CubicTo {
                control1,
                control2,
                to,
            } => {
                let Some(from) = current else {
                    unreachable!("validated path segment always has a current point")
                };
                state.include(
                    Segment::Cubic {
                        from,
                        control1,
                        control2,
                        to,
                    },
                    point,
                );
                if state.boundary {
                    return state;
                }
                current = Some(to);
                has_segment = true;
            }
            PathVerb::Close => {
                let Some(from) = current else {
                    unreachable!("validated close always has a current point")
                };
                let Some(to) = contour_start else {
                    unreachable!("validated close always has a contour start")
                };
                state.include(Segment::Line { from, to }, point);
                if state.boundary {
                    return state;
                }
                current = Some(to);
                explicitly_closed = true;
            }
        }
    }

    finish_open_contour(
        &mut state,
        current,
        contour_start,
        has_segment,
        explicitly_closed,
        point,
    );
    state
}

fn finish_open_contour(
    state: &mut FillState,
    current: Option<LogicalPoint>,
    contour_start: Option<LogicalPoint>,
    has_segment: bool,
    explicitly_closed: bool,
    point: LogicalPoint,
) {
    if !has_segment || explicitly_closed {
        return;
    }
    let Some(from) = current else {
        unreachable!("segment-bearing contour always has a current point")
    };
    let Some(to) = contour_start else {
        unreachable!("segment-bearing contour always has a start point")
    };
    state.include(Segment::Line { from, to }, point);
}

#[derive(Default)]
struct FillState {
    winding: i128,
    parity: bool,
    boundary: bool,
}

impl FillState {
    fn include(&mut self, segment: Segment, point: LogicalPoint) {
        if segment.is_point_degenerate() {
            return;
        }

        let mut parameters = [0.0_f64; 6];
        let count = segment.monotonic_parameters(&mut parameters);
        for pair in parameters[..count].windows(2) {
            match interval_contribution(segment, pair[0], pair[1], point) {
                IntervalContribution::None => {}
                IntervalContribution::Boundary => {
                    self.boundary = true;
                    return;
                }
                IntervalContribution::Crossing(direction) => {
                    self.winding += i128::from(direction);
                    self.parity = !self.parity;
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Segment {
    Line {
        from: LogicalPoint,
        to: LogicalPoint,
    },
    Quadratic {
        from: LogicalPoint,
        control: LogicalPoint,
        to: LogicalPoint,
    },
    Cubic {
        from: LogicalPoint,
        control1: LogicalPoint,
        control2: LogicalPoint,
        to: LogicalPoint,
    },
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

impl Segment {
    fn is_point_degenerate(self) -> bool {
        match self {
            Self::Line { from, to } => line_is_point_degenerate(from, to),
            Self::Quadratic { from, control, to } => {
                quadratic_is_point_degenerate(from, control, to)
            }
            Self::Cubic {
                from,
                control1,
                control2,
                to,
            } => cubic_is_point_degenerate(from, control1, control2, to),
        }
    }

    fn evaluate(self, axis: Axis, parameter: f64) -> f64 {
        let coordinate = |point: LogicalPoint| match axis {
            Axis::X => f64::from(point.x()),
            Axis::Y => f64::from(point.y()),
        };
        match self {
            Self::Line { from, to } => {
                let start = coordinate(from);
                (coordinate(to) - start).mul_add(parameter, start)
            }
            Self::Quadratic { from, control, to } => quadratic(
                coordinate(from),
                coordinate(control),
                coordinate(to),
                parameter,
            ),
            Self::Cubic {
                from,
                control1,
                control2,
                to,
            } => cubic(
                coordinate(from),
                coordinate(control1),
                coordinate(control2),
                coordinate(to),
                parameter,
            ),
        }
    }

    fn monotonic_parameters(self, parameters: &mut [f64; 6]) -> usize {
        parameters[0] = 0.0;
        parameters[1] = 1.0;
        let mut count = 2;
        for axis in [Axis::X, Axis::Y] {
            let mut roots = [0.0_f64; 2];
            let root_count = self.derivative_roots(axis, &mut roots);
            for &root in &roots[..root_count] {
                if (0.0..1.0).contains(&root) {
                    parameters[count] = root;
                    count += 1;
                }
            }
        }
        parameters[..count].sort_by(f64::total_cmp);

        let mut unique = 1;
        for index in 1..count {
            if parameters[index].total_cmp(&parameters[unique - 1]) != Ordering::Equal {
                parameters[unique] = parameters[index];
                unique += 1;
            }
        }
        unique
    }

    fn derivative_roots(self, axis: Axis, roots: &mut [f64; 2]) -> usize {
        let coordinate = |point: LogicalPoint| match axis {
            Axis::X => f64::from(point.x()),
            Axis::Y => f64::from(point.y()),
        };
        match self {
            Self::Line { .. } => 0,
            Self::Quadratic { from, control, to } => {
                let start = coordinate(from);
                let control = coordinate(control);
                let end = coordinate(to);
                let denominator = (-2.0_f64).mul_add(control, start) + end;
                if denominator == 0.0 {
                    0
                } else {
                    roots[0] = (start - control) / denominator;
                    1
                }
            }
            Self::Cubic {
                from,
                control1,
                control2,
                to,
            } => {
                let [cubic_term, quadratic_term, linear_term, _] = cubic_coefficients(
                    coordinate(from),
                    coordinate(control1),
                    coordinate(control2),
                    coordinate(to),
                );
                quadratic_roots(3.0 * cubic_term, 2.0 * quadratic_term, linear_term, roots)
            }
        }
    }
}

#[derive(Clone, Copy)]
enum IntervalContribution {
    None,
    Boundary,
    Crossing(i8),
}

fn interval_contribution(
    segment: Segment,
    start_parameter: f64,
    end_parameter: f64,
    point: LogicalPoint,
) -> IntervalContribution {
    if point_on_monotonic_interval(segment, start_parameter, end_parameter, point) {
        return IntervalContribution::Boundary;
    }

    let point_y = f64::from(point.y());
    let start_y = segment.evaluate(Axis::Y, start_parameter);
    let end_y = segment.evaluate(Axis::Y, end_parameter);
    let direction = if start_y <= point_y && point_y < end_y {
        1
    } else if end_y <= point_y && point_y < start_y {
        -1
    } else {
        return IntervalContribution::None;
    };

    let bracket = root_bracket(segment, Axis::Y, point_y, start_parameter, end_parameter);
    let start_x = segment.evaluate(Axis::X, bracket.0);
    let end_x = segment.evaluate(Axis::X, bracket.1);
    let min_x = start_x.min(end_x);
    let max_x = start_x.max(end_x);
    let point_x = f64::from(point.x());

    if point_x < min_x {
        IntervalContribution::Crossing(direction)
    } else if point_x > max_x {
        IntervalContribution::None
    } else {
        // The y-root is narrowed to the closest representable parameter bracket.
        // If x remains inside that bracket's monotonic image, the logical point is
        // indistinguishable from the exact curve intersection at f32 input
        // precision and is therefore treated as the accepted inclusive boundary.
        IntervalContribution::Boundary
    }
}

fn point_on_monotonic_interval(
    segment: Segment,
    start_parameter: f64,
    end_parameter: f64,
    point: LogicalPoint,
) -> bool {
    let point_x = f64::from(point.x());
    let point_y = f64::from(point.y());
    let start_x = segment.evaluate(Axis::X, start_parameter);
    let end_x = segment.evaluate(Axis::X, end_parameter);
    let start_y = segment.evaluate(Axis::Y, start_parameter);
    let end_y = segment.evaluate(Axis::Y, end_parameter);

    if !inclusive_between(point_x, start_x, end_x) || !inclusive_between(point_y, start_y, end_y) {
        return false;
    }

    let x_constant = start_x.total_cmp(&end_x) == Ordering::Equal;
    let y_constant = start_y.total_cmp(&end_y) == Ordering::Equal;
    match (x_constant, y_constant) {
        (true, true) => {
            point_x.total_cmp(&start_x) == Ordering::Equal
                && point_y.total_cmp(&start_y) == Ordering::Equal
        }
        (true, false) => point_x.total_cmp(&start_x) == Ordering::Equal,
        (false, true) => point_y.total_cmp(&start_y) == Ordering::Equal,
        (false, false) => {
            let x_bracket = root_bracket(segment, Axis::X, point_x, start_parameter, end_parameter);
            let y_bracket = root_bracket(segment, Axis::Y, point_y, start_parameter, end_parameter);
            brackets_overlap(x_bracket, y_bracket)
        }
    }
}

fn inclusive_between(value: f64, first: f64, second: f64) -> bool {
    value >= first.min(second) && value <= first.max(second)
}

fn root_bracket(
    segment: Segment,
    axis: Axis,
    target: f64,
    mut low: f64,
    mut high: f64,
) -> (f64, f64) {
    let low_value = segment.evaluate(axis, low);
    if low_value.total_cmp(&target) == Ordering::Equal {
        return (low, low);
    }
    let high_value = segment.evaluate(axis, high);
    if high_value.total_cmp(&target) == Ordering::Equal {
        return (high, high);
    }
    let increasing = low_value < high_value;

    for _ in 0..96 {
        let middle = f64::midpoint(low, high);
        if middle.total_cmp(&low) == Ordering::Equal || middle.total_cmp(&high) == Ordering::Equal {
            break;
        }
        let value = segment.evaluate(axis, middle);
        if value.total_cmp(&target) == Ordering::Equal {
            return (middle, middle);
        }
        if (value < target) == increasing {
            low = middle;
        } else {
            high = middle;
        }
    }
    (low, high)
}

fn brackets_overlap(first: (f64, f64), second: (f64, f64)) -> bool {
    first.0.next_down() <= second.1.next_up() && second.0.next_down() <= first.1.next_up()
}

#[cfg(test)]
mod tests {
    use crate::{LogicalPoint, PathFillRule, PathVerb, ScenePath};

    fn point(x: f32, y: f32) -> LogicalPoint {
        LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
    }

    fn path(verbs: Vec<PathVerb>, fill_rule: PathFillRule) -> ScenePath {
        ScenePath::new(verbs, fill_rule).unwrap_or_else(|_| unreachable!("test path is valid"))
    }

    #[test]
    fn open_contour_fill_uses_synthetic_edge_and_includes_boundaries() {
        let triangle = path(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::LineTo(point(10.0, 0.0)),
                PathVerb::LineTo(point(0.0, 10.0)),
            ],
            PathFillRule::NonZero,
        );

        assert!(triangle.contains_fill(point(1.0, 1.0)));
        assert!(triangle.contains_fill(point(5.0, 5.0)));
        assert!(triangle.contains_fill(point(10.0, 0.0)));
        assert!(!triangle.contains_fill(point(9.0, 9.0)));
    }

    #[test]
    fn move_only_contours_remain_coverage_empty() {
        let empty = path(
            vec![
                PathVerb::MoveTo(point(2.0, 3.0)),
                PathVerb::MoveTo(point(8.0, 13.0)),
            ],
            PathFillRule::EvenOdd,
        );
        assert!(!empty.contains_fill(point(2.0, 3.0)));
    }

    #[test]
    fn point_degenerate_segments_do_not_manufacture_fill_boundaries() {
        let mixed = path(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::LineTo(point(10.0, 10.0)),
                PathVerb::MoveTo(point(0.0, 10.0)),
                PathVerb::LineTo(point(0.0, 10.0)),
                PathVerb::QuadraticTo {
                    control: point(0.0, 10.0),
                    to: point(0.0, 10.0),
                },
                PathVerb::CubicTo {
                    control1: point(0.0, 10.0),
                    control2: point(0.0, 10.0),
                    to: point(0.0, 10.0),
                },
                PathVerb::Close,
            ],
            PathFillRule::NonZero,
        );

        assert!(!mixed.contains_fill(point(0.0, 10.0)));
        assert!(mixed.contains_fill(point(5.0, 5.0)));
    }

    #[test]
    fn entirely_point_degenerate_closed_contour_is_coverage_empty() {
        let empty = path(
            vec![
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
            ],
            PathFillRule::EvenOdd,
        );
        assert!(!empty.contains_fill(point(2.0, 3.0)));
    }

    #[test]
    fn authored_fill_rule_controls_non_boundary_overlap() {
        let double_square = vec![
            PathVerb::MoveTo(point(0.0, 0.0)),
            PathVerb::LineTo(point(10.0, 0.0)),
            PathVerb::LineTo(point(10.0, 10.0)),
            PathVerb::LineTo(point(0.0, 10.0)),
            PathVerb::Close,
            PathVerb::MoveTo(point(0.0, 0.0)),
            PathVerb::LineTo(point(10.0, 0.0)),
            PathVerb::LineTo(point(10.0, 10.0)),
            PathVerb::LineTo(point(0.0, 10.0)),
            PathVerb::Close,
        ];
        assert!(path(double_square.clone(), PathFillRule::NonZero).contains_fill(point(5.0, 5.0)));
        assert!(!path(double_square, PathFillRule::EvenOdd).contains_fill(point(5.0, 5.0)));
    }

    #[test]
    fn opposite_winding_contours_cancel_under_non_zero() {
        let opposite_squares = vec![
            PathVerb::MoveTo(point(0.0, 0.0)),
            PathVerb::LineTo(point(10.0, 0.0)),
            PathVerb::LineTo(point(10.0, 10.0)),
            PathVerb::LineTo(point(0.0, 10.0)),
            PathVerb::Close,
            PathVerb::MoveTo(point(0.0, 0.0)),
            PathVerb::LineTo(point(0.0, 10.0)),
            PathVerb::LineTo(point(10.0, 10.0)),
            PathVerb::LineTo(point(10.0, 0.0)),
            PathVerb::Close,
        ];
        assert!(!path(opposite_squares, PathFillRule::NonZero).contains_fill(point(5.0, 5.0)));
    }

    #[test]
    fn quadratic_and_cubic_boundaries_are_inside_without_flattening() {
        let quadratic_arch = path(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::QuadraticTo {
                    control: point(5.0, 10.0),
                    to: point(10.0, 0.0),
                },
            ],
            PathFillRule::NonZero,
        );
        assert!(quadratic_arch.contains_fill(point(5.0, 5.0)));
        assert!(quadratic_arch.contains_fill(point(2.5, 3.75)));
        assert!(!quadratic_arch.contains_fill(point(2.5, 3.75_f32.next_up())));
        assert!(quadratic_arch.contains_fill(point(5.0, 2.0)));
        assert!(!quadratic_arch.contains_fill(point(5.0, 6.0)));

        let cubic_arch = path(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::CubicTo {
                    control1: point(0.0, 10.0),
                    control2: point(10.0, 10.0),
                    to: point(10.0, 0.0),
                },
            ],
            PathFillRule::NonZero,
        );
        assert!(cubic_arch.contains_fill(point(5.0, 7.5)));
        assert!(cubic_arch.contains_fill(point(5.0, 3.0)));
        assert!(!cubic_arch.contains_fill(point(5.0, 8.0)));
    }

    #[test]
    fn a_single_segment_has_boundary_fill_but_no_manufactured_area() {
        let segment = path(
            vec![
                PathVerb::MoveTo(point(0.0, 0.0)),
                PathVerb::LineTo(point(10.0, 0.0)),
            ],
            PathFillRule::NonZero,
        );
        assert!(segment.contains_fill(point(5.0, 0.0)));
        assert!(!segment.contains_fill(point(5.0, 1.0)));
    }
}
