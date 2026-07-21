use glam::Vec3;

use crate::error::{ValidationError, ensure_finite_vec3, ensure_non_negative, ensure_sample_point};
use crate::{Bounds3, FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SdfSphere {
    center: Vec3,
    radius: f32,
    bounds: Bounds3,
}

impl SdfSphere {
    pub fn new(center: Vec3, radius: f32) -> Result<Self, ValidationError> {
        ensure_finite_vec3(center, "sphere center")?;
        ensure_non_negative(radius, "sphere radius")?;
        let bounds = Bounds3::from_center_half_extents(center, Vec3::splat(radius))?;
        Ok(Self {
            center,
            radius,
            bounds,
        })
    }

    pub const fn center(self) -> Vec3 {
        self.center
    }

    pub const fn radius(self) -> f32 {
        self.radius
    }
}

impl SdfField3 for SdfSphere {
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        ensure_sample_point(point)?;
        SdfSample::exact_signed_distance((point - self.center).length() - self.radius)
    }

    fn bounds(&self) -> FieldBounds {
        FieldBounds::bounded(self.bounds)
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::EXACT_DISTANCE
    }
}
