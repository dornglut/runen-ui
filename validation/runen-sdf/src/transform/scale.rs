use glam::Vec3;

use crate::error::{ValidationError, ensure_finite_scalar};
use crate::{FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Scale<F> {
    field: F,
    scale: f32,
}

impl<F> Scale<F> {
    pub fn new(field: F, scale: f32) -> Result<Self, ValidationError> {
        ensure_finite_scalar(scale, "uniform scale")?;
        if scale.abs() <= f32::EPSILON {
            return Err(ValidationError::NonPositive {
                parameter: "absolute uniform scale",
                value: scale.abs(),
            });
        }
        Ok(Self { field, scale })
    }

    pub const fn scale(&self) -> f32 {
        self.scale
    }

    pub const fn field(&self) -> &F {
        &self.field
    }

    pub fn into_field(self) -> F {
        self.field
    }
}

impl<F> SdfField3 for Scale<F>
where
    F: SdfField3,
{
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        let absolute_scale = self.scale.abs();
        let local = self.field.sample(point / self.scale)?;
        SdfSample::from_parts(
            local.signed_value() * absolute_scale,
            local.safe_step().map(|step| step * absolute_scale),
        )
    }

    fn bounds(&self) -> FieldBounds {
        self.field.bounds().map_corners(|point| point * self.scale)
    }

    fn capabilities(&self) -> FieldCapabilities {
        self.field.capabilities()
    }
}
