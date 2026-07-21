use glam::Vec3;

use crate::error::{ValidationError, ensure_finite_vec3};
use crate::{FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DomainWarp<F> {
    field: F,
    amplitude: Vec3,
    frequency: Vec3,
    phase: Vec3,
}

impl<F> DomainWarp<F> {
    pub fn new(
        field: F,
        amplitude: Vec3,
        frequency: Vec3,
        phase: Vec3,
    ) -> Result<Self, ValidationError> {
        ensure_finite_vec3(amplitude, "warp amplitude")?;
        ensure_finite_vec3(frequency, "warp frequency")?;
        ensure_finite_vec3(phase, "warp phase")?;
        Ok(Self {
            field,
            amplitude,
            frequency,
            phase,
        })
    }

    fn warp_offset(&self, point: Vec3) -> Vec3 {
        Vec3::new(
            (point.x * self.frequency.x + self.phase.x).sin(),
            (point.y * self.frequency.y + self.phase.y).sin(),
            (point.z * self.frequency.z + self.phase.z).sin(),
        ) * self.amplitude
    }
}

impl<F> SdfField3 for DomainWarp<F>
where
    F: SdfField3,
{
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        let sample = self.field.sample(point + self.warp_offset(point))?;
        Ok(sample.without_safe_step())
    }

    fn bounds(&self) -> FieldBounds {
        self.field.bounds().expanded_vector(self.amplitude.abs())
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::SIGNED_FIELD
    }
}
