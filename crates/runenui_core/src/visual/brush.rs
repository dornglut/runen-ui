use core::{error::Error, fmt};
use std::sync::Arc;

use crate::{Color, LogicalLength, LogicalPoint};

/// Validation failure for normalized unit-interval values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitIntervalError {
    /// The value was NaN or infinite.
    NotFinite,
    /// The finite value was outside `[0, 1]`.
    OutOfRange,
}

impl fmt::Display for UnitIntervalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotFinite => "normalized value must be finite",
            Self::OutOfRange => "normalized value must be within [0, 1]",
        })
    }
}

impl Error for UnitIntervalError {}

/// Finite normalized value in the closed `[0, 1]` range.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct UnitInterval(f32);

impl UnitInterval {
    /// Start of the normalized interval.
    pub const ZERO: Self = Self(0.0);
    /// Midpoint of the normalized interval.
    pub const HALF: Self = Self(0.5);
    /// End of the normalized interval.
    pub const ONE: Self = Self(1.0);

    /// Validates one unit-interval value.
    ///
    /// # Errors
    ///
    /// Returns [`UnitIntervalError`] when non-finite or outside `[0, 1]`.
    pub const fn new(value: f32) -> Result<Self, UnitIntervalError> {
        if value.is_nan() || value == f32::INFINITY || value == f32::NEG_INFINITY {
            Err(UnitIntervalError::NotFinite)
        } else if value < 0.0 || value > 1.0 {
            Err(UnitIntervalError::OutOfRange)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the validated scalar.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// One validated gradient stop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    offset: UnitInterval,
    color: Color,
}

impl GradientStop {
    /// Creates one already-validated stop.
    #[must_use]
    pub const fn new(offset: UnitInterval, color: Color) -> Self {
        Self { offset, color }
    }

    /// Returns the normalized stop offset.
    #[must_use]
    pub const fn offset(self) -> UnitInterval {
        self.offset
    }

    /// Returns the straight-alpha sRGB8 public color.
    #[must_use]
    pub const fn color(self) -> Color {
        self.color
    }
}

/// Validation failure for a complete gradient-stop list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GradientStopsError {
    /// Fewer than two stops were supplied.
    TooFewStops,
    /// Stop offsets were not in stable nondecreasing order.
    DecreasingOffsets,
}

impl fmt::Display for GradientStopsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TooFewStops => "gradient requires at least two stops",
            Self::DecreasingOffsets => "gradient stop offsets must be nondecreasing",
        })
    }
}

impl Error for GradientStopsError {}

/// Immutable validated stable gradient stops.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientStops(Arc<[GradientStop]>);

impl GradientStops {
    /// Validates and freezes gradient stops.
    ///
    /// Equal offsets are accepted in authored order for deterministic hard-stop
    /// semantics; only decreasing offsets reject.
    ///
    /// # Errors
    ///
    /// Returns [`GradientStopsError`] for too few or decreasing stops.
    pub fn new(stops: impl Into<Vec<GradientStop>>) -> Result<Self, GradientStopsError> {
        let stops = stops.into();
        if stops.len() < 2 {
            return Err(GradientStopsError::TooFewStops);
        }
        if stops
            .windows(2)
            .any(|pair| pair[0].offset().get() > pair[1].offset().get())
        {
            return Err(GradientStopsError::DecreasingOffsets);
        }
        Ok(Self(Arc::from(stops)))
    }

    /// Returns exact authored stops in stable order.
    #[must_use]
    pub fn as_slice(&self) -> &[GradientStop] {
        self.0.as_ref()
    }

    /// Samples the accepted gradient-stop function at one normalized coordinate.
    ///
    /// Outside the authored first/last stop range the corresponding endpoint color
    /// is extended. At an exact shared-offset hard stop the first authored color at
    /// that coordinate is the boundary value; immediately on the increasing side,
    /// the last authored color at that offset becomes the interpolation source.
    /// Interpolation is premultiplied linear-sRGB and is deterministically converted
    /// back to straight-alpha sRGB8.
    #[must_use]
    pub fn sample(&self, coordinate: UnitInterval) -> Color {
        sample_gradient_stops(self.as_slice(), f64::from(coordinate.get()))
    }
}

/// Geometry failure for an accepted gradient.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GradientGeometryError {
    /// A linear gradient used equal endpoints.
    EqualLinearEndpoints,
    /// A radial gradient used zero radius.
    ZeroRadialRadius,
}

impl fmt::Display for GradientGeometryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EqualLinearEndpoints => "linear gradient endpoints must differ",
            Self::ZeroRadialRadius => "radial gradient radius must be positive",
        })
    }
}

impl Error for GradientGeometryError {}

/// Primitive-local linear gradient.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradient {
    start: LogicalPoint,
    end: LogicalPoint,
    stops: GradientStops,
}

impl LinearGradient {
    /// Creates one nondegenerate linear gradient.
    ///
    /// # Errors
    ///
    /// Returns [`GradientGeometryError::EqualLinearEndpoints`] when endpoints match.
    pub fn new(
        start: LogicalPoint,
        end: LogicalPoint,
        stops: GradientStops,
    ) -> Result<Self, GradientGeometryError> {
        if start == end {
            return Err(GradientGeometryError::EqualLinearEndpoints);
        }
        Ok(Self { start, end, stops })
    }

    /// Returns the primitive-local start point.
    #[must_use]
    pub const fn start(&self) -> LogicalPoint {
        self.start
    }

    /// Returns the primitive-local end point.
    #[must_use]
    pub const fn end(&self) -> LogicalPoint {
        self.end
    }

    /// Returns stable gradient stops.
    #[must_use]
    pub const fn stops(&self) -> &GradientStops {
        &self.stops
    }

    /// Samples this gradient at one primitive-local logical point.
    #[must_use]
    pub fn sample_at(&self, point: LogicalPoint) -> Color {
        let start_x = f64::from(self.start.x());
        let start_y = f64::from(self.start.y());
        let direction_x = f64::from(self.end.x()) - start_x;
        let direction_y = f64::from(self.end.y()) - start_y;
        let point_x = f64::from(point.x()) - start_x;
        let point_y = f64::from(point.y()) - start_y;
        let denominator = direction_y.mul_add(direction_y, direction_x * direction_x);
        let numerator = point_y.mul_add(direction_y, point_x * direction_x);
        let coordinate = (numerator / denominator).clamp(0.0, 1.0);
        sample_gradient_stops(self.stops.as_slice(), coordinate)
    }
}

/// Primitive-local concentric radial gradient.
#[derive(Clone, Debug, PartialEq)]
pub struct RadialGradient {
    center: LogicalPoint,
    radius: LogicalLength,
    stops: GradientStops,
}

impl RadialGradient {
    /// Creates one positive-radius concentric radial gradient.
    ///
    /// # Errors
    ///
    /// Returns [`GradientGeometryError::ZeroRadialRadius`] for radius zero.
    pub fn new(
        center: LogicalPoint,
        radius: LogicalLength,
        stops: GradientStops,
    ) -> Result<Self, GradientGeometryError> {
        if radius == LogicalLength::ZERO {
            return Err(GradientGeometryError::ZeroRadialRadius);
        }
        Ok(Self {
            center,
            radius,
            stops,
        })
    }

    /// Returns the primitive-local center.
    #[must_use]
    pub const fn center(&self) -> LogicalPoint {
        self.center
    }

    /// Returns the positive logical radius.
    #[must_use]
    pub const fn radius(&self) -> LogicalLength {
        self.radius
    }

    /// Returns stable gradient stops.
    #[must_use]
    pub const fn stops(&self) -> &GradientStops {
        &self.stops
    }

    /// Samples this gradient at one primitive-local logical point.
    #[must_use]
    pub fn sample_at(&self, point: LogicalPoint) -> Color {
        let offset_x = f64::from(point.x()) - f64::from(self.center.x());
        let offset_y = f64::from(point.y()) - f64::from(self.center.y());
        let coordinate = (offset_x.hypot(offset_y) / f64::from(self.radius.get())).clamp(0.0, 1.0);
        sample_gradient_stops(self.stops.as_slice(), coordinate)
    }
}

/// RunenUI-owned initial brush vocabulary.
#[derive(Clone, Debug, PartialEq)]
pub enum Brush {
    /// Straight-alpha sRGB8 literal solid color.
    Solid(Color),
    /// Nondegenerate primitive-local linear gradient.
    Linear(LinearGradient),
    /// Positive-radius primitive-local concentric radial gradient.
    Radial(RadialGradient),
}

impl Brush {
    /// Creates the trivial solid-brush case.
    #[must_use]
    pub const fn solid(color: Color) -> Self {
        Self::Solid(color)
    }

    /// Samples this brush at one primitive-local logical point.
    #[must_use]
    pub fn sample_at(&self, point: LogicalPoint) -> Color {
        match self {
            Self::Solid(color) => *color,
            Self::Linear(gradient) => gradient.sample_at(point),
            Self::Radial(gradient) => gradient.sample_at(point),
        }
    }
}

impl From<Color> for Brush {
    fn from(color: Color) -> Self {
        Self::Solid(color)
    }
}

#[allow(
    clippy::float_cmp,
    reason = "exact equality is the public hard-stop boundary rule for validated authored offsets"
)]
fn sample_gradient_stops(stops: &[GradientStop], coordinate: f64) -> Color {
    let first = stops[0];
    let first_offset = f64::from(first.offset().get());
    if coordinate <= first_offset {
        return first.color();
    }

    for (index, stop) in stops.iter().copied().enumerate().skip(1) {
        let stop_offset = f64::from(stop.offset().get());
        if coordinate > stop_offset {
            continue;
        }
        if coordinate == stop_offset {
            let mut first_equal = index;
            while first_equal > 0 && stops[first_equal - 1].offset().get() == stop.offset().get() {
                first_equal -= 1;
            }
            return stops[first_equal].color();
        }

        let previous = stops[index - 1];
        let previous_offset = f64::from(previous.offset().get());
        let progress = (coordinate - previous_offset) / (stop_offset - previous_offset);
        return interpolate_color(previous.color(), stop.color(), progress);
    }

    stops[stops.len() - 1].color()
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
    let start = premultiplied_linear(start);
    let end = premultiplied_linear(end);
    let interpolated = start
        .into_iter()
        .zip(end)
        .map(|(start, end)| (end - start).mul_add(progress, start))
        .collect::<Vec<_>>();
    let alpha = interpolated[3].clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return Color::TRANSPARENT;
    }
    Color::rgba(
        linear_to_srgb8(interpolated[0] / alpha),
        linear_to_srgb8(interpolated[1] / alpha),
        linear_to_srgb8(interpolated[2] / alpha),
        unit_to_u8(alpha),
    )
}

fn premultiplied_linear(color: Color) -> [f64; 4] {
    let alpha = f64::from(color.alpha()) / 255.0;
    [
        srgb8_to_linear(color.red()) * alpha,
        srgb8_to_linear(color.green()) * alpha,
        srgb8_to_linear(color.blue()) * alpha,
        alpha,
    ]
}

fn srgb8_to_linear(channel: u8) -> f64 {
    let srgb = f64::from(channel) / 255.0;
    if srgb <= 0.040_45 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb8(linear: f64) -> u8 {
    let linear = linear.clamp(0.0, 1.0);
    let srgb = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055_f64.mul_add(linear.powf(1.0 / 2.4), -0.055)
    };
    unit_to_u8(srgb)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to [0,1], scaled to the exact u8 range, and rounded before conversion"
)]
fn unit_to_u8(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::{
        Brush, GradientGeometryError, GradientStop, GradientStops, GradientStopsError,
        LinearGradient, RadialGradient, UnitInterval,
    };
    use crate::{Color, LogicalLength, LogicalPoint};

    fn point(x: f32, y: f32) -> LogicalPoint {
        LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
    }

    fn stops() -> GradientStops {
        GradientStops::new(vec![
            GradientStop::new(UnitInterval::ZERO, Color::BLACK),
            GradientStop::new(UnitInterval::ONE, Color::WHITE),
        ])
        .unwrap_or_else(|_| unreachable!("test stops are valid"))
    }

    #[test]
    fn gradient_validation_preserves_stable_hard_stops() {
        let half = UnitInterval::new(0.5).unwrap_or_else(|_| unreachable!("half is valid"));
        let hard = GradientStops::new(vec![
            GradientStop::new(UnitInterval::ZERO, Color::BLACK),
            GradientStop::new(half, Color::BLACK),
            GradientStop::new(half, Color::WHITE),
            GradientStop::new(UnitInterval::ONE, Color::WHITE),
        ])
        .unwrap_or_else(|_| unreachable!("nondecreasing hard stops are valid"));
        assert_eq!(hard.as_slice()[1].offset(), hard.as_slice()[2].offset());
        assert_eq!(
            GradientStops::new(vec![GradientStop::new(UnitInterval::ZERO, Color::BLACK)]),
            Err(GradientStopsError::TooFewStops)
        );
    }

    #[test]
    fn decreasing_gradient_offsets_reject() {
        assert_eq!(
            GradientStops::new(vec![
                GradientStop::new(UnitInterval::ONE, Color::BLACK),
                GradientStop::new(UnitInterval::ZERO, Color::WHITE),
            ]),
            Err(GradientStopsError::DecreasingOffsets)
        );
    }

    #[test]
    fn degenerate_gradient_geometry_rejects() {
        assert_eq!(
            LinearGradient::new(point(1.0, 1.0), point(1.0, 1.0), stops()),
            Err(GradientGeometryError::EqualLinearEndpoints)
        );
        assert_eq!(
            RadialGradient::new(point(0.0, 0.0), LogicalLength::ZERO, stops()),
            Err(GradientGeometryError::ZeroRadialRadius)
        );
        assert_eq!(Brush::from(Color::BLACK), Brush::Solid(Color::BLACK));
    }

    #[test]
    fn gradient_sampling_uses_linear_srgb_not_straight_srgb() {
        assert_eq!(
            stops().sample(UnitInterval::HALF),
            Color::rgb(188, 188, 188)
        );
    }

    #[test]
    fn gradient_sampling_interpolates_premultiplied_alpha() {
        let stops = GradientStops::new(vec![
            GradientStop::new(UnitInterval::ZERO, Color::rgb(255, 0, 0)),
            GradientStop::new(UnitInterval::ONE, Color::rgba(0, 0, 255, 0)),
        ])
        .unwrap_or_else(|_| unreachable!("test stops are valid"));
        assert_eq!(
            stops.sample(UnitInterval::HALF),
            Color::rgba(255, 0, 0, 128)
        );
    }

    #[test]
    fn hard_stop_boundary_and_increasing_side_are_stable() {
        let half = UnitInterval::HALF;
        let hard = GradientStops::new(vec![
            GradientStop::new(UnitInterval::ZERO, Color::BLACK),
            GradientStop::new(half, Color::rgb(255, 0, 0)),
            GradientStop::new(half, Color::rgb(0, 0, 255)),
            GradientStop::new(UnitInterval::ONE, Color::WHITE),
        ])
        .unwrap_or_else(|_| unreachable!("test hard stops are valid"));
        assert_eq!(hard.sample(half), Color::rgb(255, 0, 0));
        let increasing = hard.sample(
            UnitInterval::new(0.500_001)
                .unwrap_or_else(|_| unreachable!("test coordinate is valid")),
        );
        assert!(increasing.red() < 5);
        assert!(increasing.green() < 5);
        assert!(increasing.blue() > 250);
    }

    #[test]
    fn endpoint_extension_and_primitive_local_geometry_are_authoritative() {
        let quarter =
            UnitInterval::new(0.25).unwrap_or_else(|_| unreachable!("quarter coordinate is valid"));
        let three_quarters = UnitInterval::new(0.75)
            .unwrap_or_else(|_| unreachable!("three-quarter coordinate is valid"));
        let stops = GradientStops::new(vec![
            GradientStop::new(quarter, Color::rgb(255, 0, 0)),
            GradientStop::new(three_quarters, Color::rgb(0, 0, 255)),
        ])
        .unwrap_or_else(|_| unreachable!("test stops are valid"));
        assert_eq!(stops.sample(UnitInterval::ZERO), Color::rgb(255, 0, 0));
        assert_eq!(stops.sample(UnitInterval::ONE), Color::rgb(0, 0, 255));

        let linear = LinearGradient::new(point(0.0, 0.0), point(10.0, 0.0), stops.clone())
            .unwrap_or_else(|_| unreachable!("test linear gradient is valid"));
        assert_eq!(linear.sample_at(point(-5.0, 0.0)), Color::rgb(255, 0, 0));
        assert_eq!(linear.sample_at(point(15.0, 0.0)), Color::rgb(0, 0, 255));

        let radial = RadialGradient::new(
            point(4.0, 4.0),
            LogicalLength::new(8.0).unwrap_or_else(|_| unreachable!("radius is valid")),
            stops,
        )
        .unwrap_or_else(|_| unreachable!("test radial gradient is valid"));
        assert_eq!(radial.sample_at(point(4.0, 4.0)), Color::rgb(255, 0, 0));
        assert_eq!(radial.sample_at(point(20.0, 4.0)), Color::rgb(0, 0, 255));
    }
}
