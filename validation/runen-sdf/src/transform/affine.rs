use glam::{Affine3A, Vec3};

use crate::error::ValidationError;
use crate::{FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Affine<F> {
    field: F,
    transform: Affine3A,
    inverse: Affine3A,
    conservative_scale: f32,
}

impl<F> Affine<F> {
    pub fn new(field: F, transform: Affine3A) -> Result<Self, ValidationError> {
        let matrix = transform.matrix3;
        let finite = matrix.x_axis.is_finite()
            && matrix.y_axis.is_finite()
            && matrix.z_axis.is_finite()
            && transform.translation.is_finite();
        let determinant = matrix.determinant();
        if !finite || !determinant.is_finite() || determinant == 0.0 {
            return Err(ValidationError::SingularTransform);
        }

        let inverse = transform.inverse();
        let inverse_matrix = inverse.matrix3;
        let inverse_finite = inverse_matrix.x_axis.is_finite()
            && inverse_matrix.y_axis.is_finite()
            && inverse_matrix.z_axis.is_finite()
            && inverse.translation.is_finite();
        if !inverse_finite {
            return Err(ValidationError::SingularTransform);
        }

        let frobenius_squared = inverse_matrix.x_axis.length_squared()
            + inverse_matrix.y_axis.length_squared()
            + inverse_matrix.z_axis.length_squared();
        if !frobenius_squared.is_finite() || frobenius_squared <= 0.0 {
            return Err(ValidationError::SingularTransform);
        }
        let conservative_scale = frobenius_squared.sqrt().recip();
        if !conservative_scale.is_finite() || conservative_scale <= 0.0 {
            return Err(ValidationError::SingularTransform);
        }

        Ok(Self {
            field,
            transform,
            inverse,
            conservative_scale,
        })
    }

    pub const fn transform(&self) -> Affine3A {
        self.transform
    }

    pub const fn conservative_scale(&self) -> f32 {
        self.conservative_scale
    }

    pub const fn field(&self) -> &F {
        &self.field
    }

    pub fn into_field(self) -> F {
        self.field
    }
}

impl<F> SdfField3 for Affine<F>
where
    F: SdfField3,
{
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        let local = self.field.sample(self.inverse.transform_point3(point))?;
        SdfSample::from_parts(
            local.signed_value() * self.conservative_scale,
            local.safe_step().map(|step| step * self.conservative_scale),
        )
    }

    fn bounds(&self) -> FieldBounds {
        self.field
            .bounds()
            .map_corners(|point| self.transform.transform_point3(point))
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::SIGNED_FIELD
    }
}
