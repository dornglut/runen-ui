use glam::{Quat, Vec3};

use crate::error::ValidationError;
use crate::{FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Rotate<F> {
    field: F,
    rotation: Quat,
    inverse: Quat,
}

impl<F> Rotate<F> {
    pub fn new(field: F, rotation: Quat) -> Result<Self, ValidationError> {
        let components = rotation.to_array();
        if !components.into_iter().all(f32::is_finite) {
            return Err(ValidationError::NonFinite {
                parameter: "rotation quaternion",
            });
        }
        let length_squared = rotation.length_squared();
        if !length_squared.is_finite() || length_squared <= f32::EPSILON {
            return Err(ValidationError::ZeroVector {
                parameter: "rotation quaternion",
            });
        }
        let rotation = rotation / length_squared.sqrt();
        Ok(Self {
            field,
            rotation,
            inverse: rotation.conjugate(),
        })
    }

    pub const fn rotation(&self) -> Quat {
        self.rotation
    }

    pub const fn field(&self) -> &F {
        &self.field
    }

    pub fn into_field(self) -> F {
        self.field
    }
}

impl<F> SdfField3 for Rotate<F>
where
    F: SdfField3,
{
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        self.field.sample(self.inverse * point)
    }

    fn bounds(&self) -> FieldBounds {
        self.field
            .bounds()
            .map_corners(|point| self.rotation * point)
    }

    fn capabilities(&self) -> FieldCapabilities {
        self.field.capabilities()
    }
}
