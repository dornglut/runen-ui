use glam::Vec3;

use crate::error::{ValidationError, ensure_finite_scalar};
use crate::{FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ClampDistance<F> {
    field: F,
    min_distance: f32,
    max_distance: f32,
}

impl<F> ClampDistance<F> {
    pub fn new(field: F, min_distance: f32, max_distance: f32) -> Result<Self, ValidationError> {
        ensure_finite_scalar(min_distance, "minimum clamp distance")?;
        ensure_finite_scalar(max_distance, "maximum clamp distance")?;
        if min_distance > max_distance {
            return Err(ValidationError::InvalidRange);
        }
        Ok(Self {
            field,
            min_distance,
            max_distance,
        })
    }

    pub const fn min_distance(&self) -> f32 {
        self.min_distance
    }

    pub const fn max_distance(&self) -> f32 {
        self.max_distance
    }
}

impl<F> SdfField3 for ClampDistance<F>
where
    F: SdfField3,
{
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        let sample = self.field.sample(point)?;
        SdfSample::signed_value_only(
            sample
                .signed_value()
                .clamp(self.min_distance, self.max_distance),
        )
    }

    fn bounds(&self) -> FieldBounds {
        self.field.bounds()
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::SIGNED_FIELD
    }
}
