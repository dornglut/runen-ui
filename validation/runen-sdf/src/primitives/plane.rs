use glam::Vec3;

use crate::error::{
    ValidationError, ensure_finite_scalar, ensure_finite_vec3, ensure_sample_point,
};
use crate::{FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SdfPlane {
    normal: Vec3,
    distance: f32,
}

impl SdfPlane {
    pub fn new(normal: Vec3, distance: f32) -> Result<Self, ValidationError> {
        ensure_finite_vec3(normal, "plane normal")?;
        ensure_finite_scalar(distance, "plane distance")?;
        let length_squared = normal.length_squared();
        if !length_squared.is_finite() || length_squared <= f32::EPSILON {
            return Err(ValidationError::ZeroVector {
                parameter: "plane normal",
            });
        }
        let length = length_squared.sqrt();
        Ok(Self {
            normal: normal / length,
            distance: distance / length,
        })
    }

    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Result<Self, ValidationError> {
        ensure_finite_vec3(point, "plane point")?;
        Self::new(normal, -normal.dot(point))
    }

    pub const fn normal(self) -> Vec3 {
        self.normal
    }

    pub const fn distance(self) -> f32 {
        self.distance
    }
}

impl SdfField3 for SdfPlane {
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        ensure_sample_point(point)?;
        SdfSample::exact_signed_distance(self.normal.dot(point) + self.distance)
    }

    fn bounds(&self) -> FieldBounds {
        FieldBounds::Unbounded
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::EXACT_DISTANCE
    }
}
