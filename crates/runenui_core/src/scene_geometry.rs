//! Renderer- and host-neutral scene geometry values.

use core::{error::Error, fmt};

use crate::LogicalPoint;

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
}

impl Default for LogicalTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::{LogicalTransform, LogicalTransformError};
    use crate::LogicalPoint;

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
}
