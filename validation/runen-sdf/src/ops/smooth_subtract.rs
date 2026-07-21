use glam::Vec3;

use crate::error::{ValidationError, ensure_non_negative};
use crate::sample::minimum_safe_step;
use crate::{FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SmoothSubtract<A, B> {
    left: A,
    right: B,
    smoothness: f32,
}

impl<A, B> SmoothSubtract<A, B> {
    pub fn new(left: A, right: B, smoothness: f32) -> Result<Self, ValidationError> {
        ensure_non_negative(smoothness, "smooth subtraction smoothness")?;
        Ok(Self {
            left,
            right,
            smoothness,
        })
    }

    pub const fn smoothness(&self) -> f32 {
        self.smoothness
    }
}

impl<A, B> SdfField3 for SmoothSubtract<A, B>
where
    A: SdfField3,
    B: SdfField3,
{
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        let left = self.left.sample(point)?;
        let right = self.right.sample(point)?;
        if self.smoothness == 0.0 {
            return SdfSample::from_parts(
                left.signed_value().max(-right.signed_value()),
                minimum_safe_step(left, right),
            );
        }

        let h = (0.5 - 0.5 * (right.signed_value() + left.signed_value()) / self.smoothness)
            .clamp(0.0, 1.0);
        let value = left.signed_value()
            + (-right.signed_value() - left.signed_value()) * h
            + self.smoothness * h * (1.0 - h);
        SdfSample::signed_value_only(value)
    }

    fn bounds(&self) -> FieldBounds {
        self.left.bounds()
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::SIGNED_FIELD
    }
}
