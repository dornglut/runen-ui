use glam::Vec3;

use crate::error::{ValidationError, ensure_finite_vec3, ensure_sample_point};
use crate::{Bounds3, FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SdfBox3 {
    center: Vec3,
    half_extents: Vec3,
    bounds: Bounds3,
}

impl SdfBox3 {
    pub fn new(center: Vec3, half_extents: Vec3) -> Result<Self, ValidationError> {
        ensure_finite_vec3(center, "box center")?;
        ensure_finite_vec3(half_extents, "box half extents")?;
        if !half_extents.cmpge(Vec3::ZERO).all() {
            return Err(ValidationError::InvalidBounds);
        }
        let bounds = Bounds3::from_center_half_extents(center, half_extents)?;
        Ok(Self {
            center,
            half_extents,
            bounds,
        })
    }

    pub const fn center(self) -> Vec3 {
        self.center
    }

    pub const fn half_extents(self) -> Vec3 {
        self.half_extents
    }
}

impl SdfField3 for SdfBox3 {
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        ensure_sample_point(point)?;
        let q = (point - self.center).abs() - self.half_extents;
        let outside = q.max(Vec3::ZERO).length();
        let inside = q.x.max(q.y).max(q.z).min(0.0);
        SdfSample::exact_signed_distance(outside + inside)
    }

    fn bounds(&self) -> FieldBounds {
        FieldBounds::bounded(self.bounds)
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::EXACT_DISTANCE
    }
}
