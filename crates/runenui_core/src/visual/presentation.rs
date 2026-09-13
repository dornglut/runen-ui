use core::{error::Error, fmt};

use crate::{LogicalSize, LogicalTransform, LogicalTransformError, UnitInterval};

/// Validation failure for one finite presentation scalar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PresentationScalarError;

impl fmt::Display for PresentationScalarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("presentation scalar must be finite")
    }
}

impl Error for PresentationScalarError {}

const fn finite_scalar(value: f32) -> Result<f32, PresentationScalarError> {
    if value.is_nan() || value == f32::INFINITY || value == f32::NEG_INFINITY {
        Err(PresentationScalarError)
    } else if value == 0.0 {
        Ok(0.0)
    } else {
        Ok(value)
    }
}

/// Finite signed node-presentation translation in logical coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentationTranslation {
    x: f32,
    y: f32,
}

impl PresentationTranslation {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Validates one signed logical presentation translation.
    ///
    /// # Errors
    ///
    /// Returns [`PresentationScalarError`] when either component is non-finite.
    pub const fn new(x: f32, y: f32) -> Result<Self, PresentationScalarError> {
        let x = match finite_scalar(x) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        let y = match finite_scalar(y) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        Ok(Self { x, y })
    }

    #[must_use]
    pub const fn x(self) -> f32 {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> f32 {
        self.y
    }
}

/// Finite two-axis node-presentation scale.
///
/// Zero is retained as an explicit singular transform and negative values are
/// retained as explicit reflections; neither is replaced by backend policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentationScale {
    x: f32,
    y: f32,
}

impl PresentationScale {
    pub const IDENTITY: Self = Self { x: 1.0, y: 1.0 };

    /// Validates finite horizontal and vertical presentation scale.
    ///
    /// # Errors
    ///
    /// Returns [`PresentationScalarError`] when either component is non-finite.
    pub const fn new(x: f32, y: f32) -> Result<Self, PresentationScalarError> {
        let x = match finite_scalar(x) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        let y = match finite_scalar(y) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        Ok(Self { x, y })
    }

    #[must_use]
    pub const fn x(self) -> f32 {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> f32 {
        self.y
    }
}

/// Finite node-presentation rotation expressed explicitly in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentationRotation {
    radians: f32,
}

impl PresentationRotation {
    pub const ZERO: Self = Self { radians: 0.0 };

    /// Validates a rotation angle expressed in radians.
    ///
    /// # Errors
    ///
    /// Returns [`PresentationScalarError`] when the angle is non-finite.
    pub const fn radians(value: f32) -> Result<Self, PresentationScalarError> {
        match finite_scalar(value) {
            Ok(radians) => Ok(Self { radians }),
            Err(error) => Err(error),
        }
    }

    #[must_use]
    pub const fn get_radians(self) -> f32 {
        self.radians
    }
}

/// Explicit normalized presentation origin inside the final layout box.
///
/// The origin is always authored explicitly for a presentation transform; the
/// framework does not choose an implicit top-left or center pivot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentationOrigin {
    x: UnitInterval,
    y: UnitInterval,
}

impl PresentationOrigin {
    #[must_use]
    pub const fn new(x: UnitInterval, y: UnitInterval) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn x(self) -> UnitInterval {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> UnitInterval {
        self.y
    }
}

/// Decomposed node-wide presentation transform.
///
/// Runtime resolves the normalized origin against the final layout box, then
/// applies scale, rotation, and presentation translation in that exact order.
/// Presentation translation is therefore never scaled or rotated. Final owner
/// placement remains a separate runtime composition step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentationTransform {
    translation: PresentationTranslation,
    scale: PresentationScale,
    rotation: PresentationRotation,
    origin: PresentationOrigin,
}

impl PresentationTransform {
    #[must_use]
    pub const fn new(
        translation: PresentationTranslation,
        scale: PresentationScale,
        rotation: PresentationRotation,
        origin: PresentationOrigin,
    ) -> Self {
        Self {
            translation,
            scale,
            rotation,
            origin,
        }
    }

    #[must_use]
    pub const fn translation(self) -> PresentationTranslation {
        self.translation
    }

    #[must_use]
    pub const fn scale(self) -> PresentationScale {
        self.scale
    }

    #[must_use]
    pub const fn rotation(self) -> PresentationRotation {
        self.rotation
    }

    #[must_use]
    pub const fn origin(self) -> PresentationOrigin {
        self.origin
    }

    /// Resolves the node presentation into owner-local affine geometry for one
    /// exact final layout-box size.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalTransformError`] when finite authored values overflow
    /// while resolving the affine transform. Callers must diagnose/reject that
    /// composition rather than falling back to untransformed geometry.
    pub fn resolve_in_box(
        self,
        size: LogicalSize,
    ) -> Result<LogicalTransform, LogicalTransformError> {
        let origin_x = self.origin.x().get() * size.width();
        let origin_y = self.origin.y().get() * size.height();
        let (sin, cos) = self.rotation.get_radians().sin_cos();
        let m11 = cos * self.scale.x();
        let m12 = sin * self.scale.x();
        let m21 = -sin * self.scale.y();
        let m22 = cos * self.scale.y();
        let tx = origin_x + self.translation.x() - m11.mul_add(origin_x, m21 * origin_y);
        let ty = origin_y + self.translation.y() - m12.mul_add(origin_x, m22 * origin_y);
        LogicalTransform::try_new(m11, m12, m21, m22, tx, ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LogicalPoint, LogicalSize};

    fn unit(value: f32) -> UnitInterval {
        UnitInterval::new(value).unwrap_or_else(|_| unreachable!("test unit value is valid"))
    }

    fn point(x: f32, y: f32) -> LogicalPoint {
        LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
    }

    fn approx_eq(left: f32, right: f32) {
        assert!((left - right).abs() <= 1.0e-4, "{left} != {right}");
    }

    #[test]
    fn presentation_scalars_reject_non_finite_values_and_keep_singular_scale() {
        assert_eq!(
            PresentationTranslation::new(f32::NAN, 0.0),
            Err(PresentationScalarError)
        );
        assert_eq!(
            PresentationScale::new(1.0, f32::INFINITY),
            Err(PresentationScalarError)
        );
        assert_eq!(
            PresentationRotation::radians(f32::NEG_INFINITY),
            Err(PresentationScalarError)
        );
        let singular = PresentationScale::new(0.0, -1.0)
            .unwrap_or_else(|_| unreachable!("finite zero/negative scale is accepted"));
        assert_eq!((singular.x(), singular.y()), (0.0, -1.0));
    }

    #[test]
    fn resolution_uses_explicit_origin_and_scale_rotate_translate_order() {
        let transform = PresentationTransform::new(
            PresentationTranslation::new(10.0, 20.0)
                .unwrap_or_else(|_| unreachable!("test translation is finite")),
            PresentationScale::new(2.0, 1.0)
                .unwrap_or_else(|_| unreachable!("test scale is finite")),
            PresentationRotation::radians(core::f32::consts::FRAC_PI_2)
                .unwrap_or_else(|_| unreachable!("test rotation is finite")),
            PresentationOrigin::new(unit(0.5), unit(0.5)),
        );
        let size = LogicalSize::try_new(100.0, 50.0)
            .unwrap_or_else(|_| unreachable!("test size is valid"));
        let resolved = transform
            .resolve_in_box(size)
            .unwrap_or_else(|_| unreachable!("test transform resolves finitely"));

        let origin = resolved
            .transform_point(point(50.0, 25.0))
            .unwrap_or_else(|| unreachable!("origin mapping remains finite"));
        approx_eq(origin.x(), 60.0);
        approx_eq(origin.y(), 45.0);

        let right = resolved
            .transform_point(point(60.0, 25.0))
            .unwrap_or_else(|| unreachable!("test mapping remains finite"));
        approx_eq(right.x(), 60.0);
        approx_eq(right.y(), 65.0);
    }

    #[test]
    fn resolution_rejects_finite_authored_values_that_overflow_affine_composition() {
        let transform = PresentationTransform::new(
            PresentationTranslation::ZERO,
            PresentationScale::new(f32::MAX, f32::MAX)
                .unwrap_or_else(|_| unreachable!("maximum finite scale is accepted")),
            PresentationRotation::ZERO,
            PresentationOrigin::new(unit(1.0), unit(1.0)),
        );
        let size = LogicalSize::try_new(f32::MAX, f32::MAX)
            .unwrap_or_else(|_| unreachable!("maximum finite layout size is accepted"));
        assert_eq!(transform.resolve_in_box(size), Err(LogicalTransformError));
    }
}
