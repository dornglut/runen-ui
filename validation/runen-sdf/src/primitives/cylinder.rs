use glam::{Vec2, Vec3};

use crate::error::{ValidationError, ensure_finite_vec3, ensure_non_negative, ensure_sample_point};
use crate::{Bounds3, FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SdfCylinder {
    center: Vec3,
    radius: f32,
    half_height: f32,
    bounds: Bounds3,
}

impl SdfCylinder {
    pub fn new(center: Vec3, radius: f32, half_height: f32) -> Result<Self, ValidationError> {
        ensure_finite_vec3(center, "cylinder center")?;
        ensure_non_negative(radius, "cylinder radius")?;
        ensure_non_negative(half_height, "cylinder half height")?;
        let extents = Vec3::new(radius, half_height, radius);
        let bounds = Bounds3::from_center_half_extents(center, extents)?;
        Ok(Self {
            center,
            radius,
            half_height,
            bounds,
        })
    }

    pub const fn center(self) -> Vec3 {
        self.center
    }

    pub const fn radius(self) -> f32 {
        self.radius
    }

    pub const fn half_height(self) -> f32 {
        self.half_height
    }
}

impl SdfField3 for SdfCylinder {
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        ensure_sample_point(point)?;
        let local = point - self.center;
        let distance = Vec2::new(
            Vec2::new(local.x, local.z).length() - self.radius,
            local.y.abs() - self.half_height,
        );
        let outside = distance.max(Vec2::ZERO).length();
        let inside = distance.x.max(distance.y).min(0.0);
        SdfSample::exact_signed_distance(outside + inside)
    }

    fn bounds(&self) -> FieldBounds {
        FieldBounds::bounded(self.bounds)
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::EXACT_DISTANCE
    }
}
