//! Renderer- and host-neutral scene geometry values.

use core::{error::Error, fmt, num::FpCategory};

use crate::{LogicalPoint, LogicalRect, Radius};

/// Error returned when an affine logical transform contains a non-finite component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogicalTransformError;

impl fmt::Display for LogicalTransformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("logical transform components must be finite")
    }
}

impl Error for LogicalTransformError {}

/// Finite two-dimensional affine transform in logical UI coordinates.
///
/// M6B uses identity and runtime-composed owner placement. The same neutral value
/// is intentionally suitable for the richer item/clip composition semantics
/// introduced by the later M6C slice without creating a backend-specific type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalTransform {
    m11: f32,
    m12: f32,
    m21: f32,
    m22: f32,
    tx: f32,
    ty: f32,
}

impl LogicalTransform {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        m11: 1.0,
        m12: 0.0,
        m21: 0.0,
        m22: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    /// Validates one affine transform.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalTransformError`] when any component is non-finite.
    pub fn try_new(
        m11: f32,
        m12: f32,
        m21: f32,
        m22: f32,
        tx: f32,
        ty: f32,
    ) -> Result<Self, LogicalTransformError> {
        if [m11, m12, m21, m22, tx, ty].into_iter().all(f32::is_finite) {
            Ok(Self {
                m11,
                m12,
                m21,
                m22,
                tx,
                ty,
            })
        } else {
            Err(LogicalTransformError)
        }
    }

    /// Creates a finite translation transform.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalTransformError`] when either offset is non-finite.
    pub fn translation(x: f32, y: f32) -> Result<Self, LogicalTransformError> {
        Self::try_new(1.0, 0.0, 0.0, 1.0, x, y)
    }

    /// Returns the matrix components as `(m11, m12, m21, m22, tx, ty)`.
    #[must_use]
    pub const fn components(self) -> [f32; 6] {
        [self.m11, self.m12, self.m21, self.m22, self.tx, self.ty]
    }

    /// Maps one finite logical point through this transform.
    ///
    /// The input point is already finite by construction. Affine arithmetic may
    /// overflow to a non-finite result for extreme values; in that case no point
    /// is returned rather than manufacturing renderer-dependent coordinates.
    #[must_use]
    pub fn transform_point(self, point: LogicalPoint) -> Option<LogicalPoint> {
        let x = self
            .m11
            .mul_add(point.x(), self.m21.mul_add(point.y(), self.tx));
        let y = self
            .m12
            .mul_add(point.x(), self.m22.mul_add(point.y(), self.ty));
        LogicalPoint::new(x, y).ok()
    }

    /// Composes this transform with a following transform.
    ///
    /// `self.then(next)` is exactly `next(self(point))`. Finite inputs can still
    /// overflow during affine composition; such a result is rejected rather than
    /// creating a non-finite scene transform.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalTransformError`] when composition produces a non-finite
    /// component.
    pub fn then(self, next: Self) -> Result<Self, LogicalTransformError> {
        Self::try_new(
            next.m11.mul_add(self.m11, next.m21 * self.m12),
            next.m12.mul_add(self.m11, next.m22 * self.m12),
            next.m11.mul_add(self.m21, next.m21 * self.m22),
            next.m12.mul_add(self.m21, next.m22 * self.m22),
            next.m11
                .mul_add(self.tx, next.m21.mul_add(self.ty, next.tx)),
            next.m12
                .mul_add(self.tx, next.m22.mul_add(self.ty, next.ty)),
        )
    }

    /// Returns the exact finite inverse when this affine transform is invertible.
    ///
    /// Singular transforms, non-finite determinant arithmetic, or inverses whose
    /// finite inputs overflow during inversion return `None`. Callers must treat
    /// that as non-covering/non-hittable under the M6 scene contract rather than
    /// falling back to untransformed geometry.
    #[must_use]
    pub fn inverse(self) -> Option<Self> {
        let determinant = self.m11.mul_add(self.m22, -(self.m21 * self.m12));
        if !determinant.is_finite() || determinant.classify() == FpCategory::Zero {
            return None;
        }

        let inverse_determinant = determinant.recip();
        Self::try_new(
            self.m22 * inverse_determinant,
            -self.m12 * inverse_determinant,
            -self.m21 * inverse_determinant,
            self.m11 * inverse_determinant,
            self.m21.mul_add(self.ty, -(self.m22 * self.tx)) * inverse_determinant,
            self.m12.mul_add(self.tx, -(self.m11 * self.ty)) * inverse_determinant,
        )
        .ok()
    }
}

impl Default for LogicalTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Error returned when scene opacity is not finite and within `[0, 1]`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneOpacityError {
    /// The supplied opacity was NaN or positive/negative infinity.
    NotFinite,
    /// The supplied finite opacity was outside the closed `[0, 1]` range.
    OutOfRange,
}

impl fmt::Display for SceneOpacityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFinite => formatter.write_str("scene opacity must be finite"),
            Self::OutOfRange => formatter.write_str("scene opacity must be within [0, 1]"),
        }
    }
}

impl Error for SceneOpacityError {}

/// Finite renderer-neutral opacity in the closed `[0, 1]` range.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct SceneOpacity(f32);

impl SceneOpacity {
    /// Fully transparent opacity.
    pub const TRANSPARENT: Self = Self(0.0);
    /// Fully opaque opacity and the default M6B-compatible item value.
    pub const OPAQUE: Self = Self(1.0);

    /// Validates one opacity value.
    ///
    /// # Errors
    ///
    /// Returns [`SceneOpacityError`] for non-finite or out-of-range values.
    pub const fn new(value: f32) -> Result<Self, SceneOpacityError> {
        if value.is_nan() || value == f32::INFINITY || value == f32::NEG_INFINITY {
            Err(SceneOpacityError::NotFinite)
        } else if value < 0.0 || value > 1.0 {
            Err(SceneOpacityError::OutOfRange)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the validated scalar opacity.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl Default for SceneOpacity {
    fn default() -> Self {
        Self::OPAQUE
    }
}

/// Snapshot-local signed ordering layer.
///
/// Layer is an ordering fact only and never identifies a widget, scene, or item.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SceneLayer(i64);

impl SceneLayer {
    /// Default layer used by M6B-compatible contributions.
    pub const ZERO: Self = Self(0);

    /// Creates one signed snapshot-local ordering layer.
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// Returns the signed ordering value.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl From<i64> for SceneLayer {
    fn from(value: i64) -> Self {
        Self::new(value)
    }
}

/// Shared logical shape used by M6 paint clips and hit regions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SceneShape {
    /// Half-open logical rectangle.
    Rect(LogicalRect),
    /// Half-open logical rectangle with four normalized circular corner radii.
    RoundedRect { rect: LogicalRect, radius: Radius },
}

impl SceneShape {
    /// Creates one rectangular scene shape.
    #[must_use]
    pub const fn rect(rect: LogicalRect) -> Self {
        Self::Rect(rect)
    }

    /// Creates one rounded-rectangle scene shape.
    #[must_use]
    pub const fn rounded_rect(rect: LogicalRect, radius: Radius) -> Self {
        Self::RoundedRect { rect, radius }
    }

    /// Returns the outer half-open logical rectangle.
    #[must_use]
    pub const fn outer_rect(self) -> LogicalRect {
        match self {
            Self::Rect(rect) | Self::RoundedRect { rect, .. } => rect,
        }
    }

    /// Returns authored circular corner radii for a rounded rectangle.
    #[must_use]
    pub const fn radius(self) -> Option<Radius> {
        match self {
            Self::Rect(_) => None,
            Self::RoundedRect { radius, .. } => Some(radius),
        }
    }

    /// Applies the exact M6 half-open rectangle / normalized circular-corner rule.
    #[must_use]
    pub fn contains(self, point: LogicalPoint) -> bool {
        let rect = self.outer_rect();
        if !rect.contains(point) {
            return false;
        }
        let Self::RoundedRect { radius, .. } = self else {
            return true;
        };

        let radii = normalized_radii(rect, radius);
        let left = f64::from(rect.x());
        let top = f64::from(rect.y());
        let right = f64::from(rect.max_x());
        let bottom = f64::from(rect.max_y());
        let x = f64::from(point.x());
        let y = f64::from(point.y());

        !outside_rounded_corner(x, y, left, top, radii[0], Corner::TopLeft)
            && !outside_rounded_corner(x, y, right, top, radii[1], Corner::TopRight)
            && !outside_rounded_corner(x, y, right, bottom, radii[2], Corner::BottomRight)
            && !outside_rounded_corner(x, y, left, bottom, radii[3], Corner::BottomLeft)
    }
}

/// One owner-local clip authored by a widget contribution.
///
/// The transform maps clip-local geometry into the contributing owner's local
/// logical space. Runtime composes owner placement before publishing a surface
/// clip; widgets never author clip-to-surface placement directly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContributionClip {
    shape: SceneShape,
    local_to_owner: LogicalTransform,
}

impl ContributionClip {
    /// Creates one already-validated owner-local clip.
    #[must_use]
    pub const fn new(shape: SceneShape, local_to_owner: LogicalTransform) -> Self {
        Self {
            shape,
            local_to_owner,
        }
    }

    /// Creates an identity-transformed owner-local clip.
    #[must_use]
    pub const fn identity(shape: SceneShape) -> Self {
        Self::new(shape, LogicalTransform::IDENTITY)
    }

    /// Returns the logical clip shape.
    #[must_use]
    pub const fn shape(self) -> SceneShape {
        self.shape
    }

    /// Returns clip-local to owner-local transform.
    #[must_use]
    pub const fn local_to_owner(self) -> LogicalTransform {
        self.local_to_owner
    }
}

#[derive(Clone, Copy)]
enum Corner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

fn normalized_radii(rect: LogicalRect, radius: Radius) -> [f64; 4] {
    let radii = [
        f64::from(radius.top_left().get()),
        f64::from(radius.top_right().get()),
        f64::from(radius.bottom_right().get()),
        f64::from(radius.bottom_left().get()),
    ];
    let width = f64::from(rect.width());
    let height = f64::from(rect.height());
    let mut factor = 1.0_f64;
    for (extent, first, second) in [
        (width, radii[0], radii[1]),
        (width, radii[3], radii[2]),
        (height, radii[0], radii[3]),
        (height, radii[1], radii[2]),
    ] {
        let denominator = first + second;
        if denominator > 0.0 {
            factor = factor.min(extent / denominator);
        }
    }
    radii.map(|value| value * factor)
}

fn outside_rounded_corner(
    x: f64,
    y: f64,
    edge_x: f64,
    edge_y: f64,
    radius: f64,
    corner: Corner,
) -> bool {
    if radius <= 0.0 {
        return false;
    }
    let (center_x, center_y, in_corner) = match corner {
        Corner::TopLeft => (
            edge_x + radius,
            edge_y + radius,
            x < edge_x + radius && y < edge_y + radius,
        ),
        Corner::TopRight => (
            edge_x - radius,
            edge_y + radius,
            x >= edge_x - radius && y < edge_y + radius,
        ),
        Corner::BottomRight => (
            edge_x - radius,
            edge_y - radius,
            x >= edge_x - radius && y >= edge_y - radius,
        ),
        Corner::BottomLeft => (
            edge_x + radius,
            edge_y - radius,
            x < edge_x + radius && y >= edge_y - radius,
        ),
    };
    if !in_corner {
        return false;
    }
    let dx = x - center_x;
    let dy = y - center_y;
    dx.mul_add(dx, dy * dy) > radius * radius
}

#[cfg(test)]
mod tests {
    use super::{
        ContributionClip, LogicalTransform, LogicalTransformError, SceneLayer, SceneOpacity,
        SceneOpacityError, SceneShape,
    };
    use crate::{LogicalLength, LogicalPoint, LogicalRect, Radius};

    #[test]
    fn transform_validation_rejects_non_finite_components() {
        assert_eq!(
            LogicalTransform::try_new(1.0, 0.0, 0.0, 1.0, f32::NAN, 0.0),
            Err(LogicalTransformError)
        );
    }

    #[test]
    fn translation_maps_logical_points_exactly() {
        let transform = LogicalTransform::translation(5.0, -3.0)
            .unwrap_or_else(|_| unreachable!("test translation is finite"));
        let point =
            LogicalPoint::new(2.0, 7.0).unwrap_or_else(|_| unreachable!("test point is finite"));
        let mapped = transform
            .transform_point(point)
            .unwrap_or_else(|| unreachable!("test mapping remains finite"));
        assert_eq!((mapped.x(), mapped.y()), (7.0, 4.0));
    }

    #[test]
    fn composition_matches_sequential_point_mapping() {
        let translate = LogicalTransform::translation(5.0, -3.0)
            .unwrap_or_else(|_| unreachable!("test translation is finite"));
        let scale = LogicalTransform::try_new(2.0, 0.0, 0.0, 4.0, 0.0, 0.0)
            .unwrap_or_else(|_| unreachable!("test scale is finite"));
        let combined = translate
            .then(scale)
            .unwrap_or_else(|_| unreachable!("test composition is finite"));
        let point =
            LogicalPoint::new(2.0, 7.0).unwrap_or_else(|_| unreachable!("test point is finite"));
        let sequential = scale
            .transform_point(
                translate
                    .transform_point(point)
                    .unwrap_or_else(|| unreachable!("test translation remains finite")),
            )
            .unwrap_or_else(|| unreachable!("test scale remains finite"));
        let mapped = combined
            .transform_point(point)
            .unwrap_or_else(|| unreachable!("test composition remains finite"));
        assert_eq!(mapped, sequential);
    }

    #[test]
    fn inverse_round_trips_and_rejects_singular_transforms() {
        let transform = LogicalTransform::try_new(2.0, 0.0, 0.0, 4.0, 8.0, -12.0)
            .unwrap_or_else(|_| unreachable!("test transform is finite"));
        let inverse = transform
            .inverse()
            .unwrap_or_else(|| unreachable!("test transform is invertible"));
        let point =
            LogicalPoint::new(3.0, 5.0).unwrap_or_else(|_| unreachable!("test point is finite"));
        let mapped = transform
            .transform_point(point)
            .and_then(|mapped| inverse.transform_point(mapped))
            .unwrap_or_else(|| unreachable!("test round trip remains finite"));
        assert_eq!(mapped, point);

        let singular = LogicalTransform::try_new(1.0, 2.0, 2.0, 4.0, 0.0, 0.0)
            .unwrap_or_else(|_| unreachable!("test singular transform is finite"));
        assert!(singular.inverse().is_none());
    }

    #[test]
    fn opacity_and_layer_values_keep_validation_and_ordering_explicit() {
        assert_eq!(
            SceneOpacity::new(f32::NAN),
            Err(SceneOpacityError::NotFinite)
        );
        assert_eq!(SceneOpacity::new(-0.1), Err(SceneOpacityError::OutOfRange));
        assert_eq!(SceneOpacity::new(1.1), Err(SceneOpacityError::OutOfRange));
        assert_eq!(SceneOpacity::default(), SceneOpacity::OPAQUE);
        assert!(SceneLayer::new(-1) < SceneLayer::ZERO);
        assert!(SceneLayer::new(1) > SceneLayer::ZERO);
    }

    #[test]
    fn rounded_shape_normalizes_all_radii_by_one_factor() {
        let rect = LogicalRect::try_new(0.0, 0.0, 10.0, 10.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let ten = LogicalLength::new(10.0).unwrap_or_else(|_| unreachable!("test radius is valid"));
        let shape = SceneShape::rounded_rect(rect, Radius::all(ten));
        let inside_arc_boundary =
            LogicalPoint::new(0.0, 5.0).unwrap_or_else(|_| unreachable!("test point is valid"));
        let outside_corner =
            LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!("test point is valid"));
        assert!(shape.contains(inside_arc_boundary));
        assert!(!shape.contains(outside_corner));
    }

    #[test]
    fn scene_shape_keeps_outer_half_open_edges_and_clip_space_explicit() {
        let rect = LogicalRect::try_new(2.0, 3.0, 4.0, 5.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let shape = SceneShape::rect(rect);
        let inside =
            LogicalPoint::new(5.999, 7.999).unwrap_or_else(|_| unreachable!("test point is valid"));
        let right =
            LogicalPoint::new(6.0, 4.0).unwrap_or_else(|_| unreachable!("test point is valid"));
        assert!(shape.contains(inside));
        assert!(!shape.contains(right));
        assert_eq!(
            ContributionClip::identity(shape).local_to_owner(),
            LogicalTransform::IDENTITY
        );
    }
}
