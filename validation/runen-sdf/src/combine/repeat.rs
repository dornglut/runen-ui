use glam::Vec3;

use crate::error::{ValidationError, ensure_finite_vec3};
use crate::{FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Repeat<F> {
    field: F,
    period: Vec3,
}

impl<F> Repeat<F> {
    pub fn new(field: F, period: Vec3) -> Result<Self, ValidationError> {
        ensure_finite_vec3(period, "repeat period")?;
        if !period.cmpge(Vec3::ZERO).all() {
            return Err(ValidationError::Negative {
                parameter: "repeat period component",
                value: period.min_element(),
            });
        }
        if period == Vec3::ZERO {
            return Err(ValidationError::ZeroVector {
                parameter: "repeat period",
            });
        }
        Ok(Self { field, period })
    }

    pub const fn period(&self) -> Vec3 {
        self.period
    }
}

impl<F> SdfField3 for Repeat<F>
where
    F: SdfField3,
{
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        let local = Vec3::new(
            repeat_axis(point.x, self.period.x),
            repeat_axis(point.y, self.period.y),
            repeat_axis(point.z, self.period.z),
        );
        Ok(self.field.sample(local)?.without_safe_step())
    }

    fn bounds(&self) -> FieldBounds {
        FieldBounds::Unbounded
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::SIGNED_FIELD
    }
}

fn repeat_axis(value: f32, period: f32) -> f32 {
    if period == 0.0 {
        value
    } else {
        (value + period * 0.5).rem_euclid(period) - period * 0.5
    }
}
