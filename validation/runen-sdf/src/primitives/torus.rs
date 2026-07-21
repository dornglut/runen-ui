use glam::{Vec2, Vec3};

use crate::error::{ValidationError, ensure_finite_vec3, ensure_non_negative, ensure_sample_point};
use crate::{Bounds3, FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SdfTorus {
    center: Vec3,
    major_radius: f32,
    minor_radius: f32,
    bounds: Bounds3,
}

impl SdfTorus {
    pub fn new(
        center: Vec3,
        major_radius: f32,
        minor_radius: f32,
    ) -> Result<Self, ValidationError> {
        ensure_finite_vec3(center, "torus center")?;
        ensure_non_negative(major_radius, "torus major radius")?;
        ensure_non_negative(minor_radius, "torus minor radius")?;
        let ring = major_radius + minor_radius;
        if !ring.is_finite() {
            return Err(ValidationError::NonFinite {
                parameter: "torus combined radius",
            });
        }
        let bounds =
            Bounds3::from_center_half_extents(center, Vec3::new(ring, minor_radius, ring))?;
        Ok(Self {
            center,
            major_radius,
            minor_radius,
            bounds,
        })
    }

    pub const fn center(self) -> Vec3 {
        self.center
    }

    pub const fn major_radius(self) -> f32 {
        self.major_radius
    }

    pub const fn minor_radius(self) -> f32 {
        self.minor_radius
    }
}

impl SdfField3 for SdfTorus {
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        ensure_sample_point(point)?;
        let local = point - self.center;
        let q = Vec2::new(
            Vec2::new(local.x, local.z).length() - self.major_radius,
            local.y,
        );
        SdfSample::exact_signed_distance(q.length() - self.minor_radius)
    }

    fn bounds(&self) -> FieldBounds {
        FieldBounds::bounded(self.bounds)
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::EXACT_DISTANCE
    }
}
