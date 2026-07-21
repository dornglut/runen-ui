use glam::Vec3;

use crate::error::{ValidationError, ensure_finite_vec3, ensure_non_negative, ensure_sample_point};
use crate::{Bounds3, FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SdfCapsule {
    start: Vec3,
    end: Vec3,
    radius: f32,
    bounds: Bounds3,
}

impl SdfCapsule {
    pub fn new(start: Vec3, end: Vec3, radius: f32) -> Result<Self, ValidationError> {
        ensure_finite_vec3(start, "capsule start")?;
        ensure_finite_vec3(end, "capsule end")?;
        ensure_non_negative(radius, "capsule radius")?;
        let extent = Vec3::splat(radius);
        let bounds = Bounds3::try_new(start.min(end) - extent, start.max(end) + extent)?;
        Ok(Self {
            start,
            end,
            radius,
            bounds,
        })
    }

    pub const fn start(self) -> Vec3 {
        self.start
    }

    pub const fn end(self) -> Vec3 {
        self.end
    }

    pub const fn radius(self) -> f32 {
        self.radius
    }
}

impl SdfField3 for SdfCapsule {
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        ensure_sample_point(point)?;
        let pa = point - self.start;
        let ba = self.end - self.start;
        let denominator = ba.length_squared();
        let factor = if denominator <= f32::EPSILON {
            0.0
        } else {
            (pa.dot(ba) / denominator).clamp(0.0, 1.0)
        };
        let closest = self.start + ba * factor;
        SdfSample::exact_signed_distance((point - closest).length() - self.radius)
    }

    fn bounds(&self) -> FieldBounds {
        FieldBounds::bounded(self.bounds)
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::EXACT_DISTANCE
    }
}
