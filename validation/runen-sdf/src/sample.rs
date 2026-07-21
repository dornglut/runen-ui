#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SdfSample {
    signed_value: f32,
    safe_step: Option<f32>,
}

impl SdfSample {
    pub fn signed_value_only(signed_value: f32) -> Result<Self, crate::SampleError> {
        Self::from_parts(signed_value, None)
    }

    pub fn exact_signed_distance(signed_value: f32) -> Result<Self, crate::SampleError> {
        Self::from_parts(signed_value, Some(signed_value.abs()))
    }

    pub fn with_safe_step(signed_value: f32, safe_step: f32) -> Result<Self, crate::SampleError> {
        Self::from_parts(signed_value, Some(safe_step))
    }

    pub fn from_parts(
        signed_value: f32,
        safe_step: Option<f32>,
    ) -> Result<Self, crate::SampleError> {
        if !signed_value.is_finite() {
            return Err(crate::SampleError::NonFiniteSignedValue);
        }

        if let Some(step) = safe_step {
            if !step.is_finite() {
                return Err(crate::SampleError::NonFiniteSafeStep);
            }
            if step < 0.0 {
                return Err(crate::SampleError::NegativeSafeStep);
            }
        }

        Ok(Self {
            signed_value,
            safe_step,
        })
    }

    pub const fn signed_value(self) -> f32 {
        self.signed_value
    }

    pub const fn safe_step(self) -> Option<f32> {
        self.safe_step
    }

    pub const fn without_safe_step(self) -> Self {
        Self {
            signed_value: self.signed_value,
            safe_step: None,
        }
    }
}

pub(crate) fn minimum_safe_step(left: SdfSample, right: SdfSample) -> Option<f32> {
    match (left.safe_step(), right.safe_step()) {
        (Some(a), Some(b)) => Some(a.min(b)),
        _ => None,
    }
}
