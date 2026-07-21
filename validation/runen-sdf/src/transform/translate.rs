use glam::Vec3;

use crate::error::{ValidationError, ensure_finite_vec3};
use crate::{FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Translate<F> {
    field: F,
    offset: Vec3,
}

impl<F> Translate<F> {
    pub fn new(field: F, offset: Vec3) -> Result<Self, ValidationError> {
        ensure_finite_vec3(offset, "translation offset")?;
        Ok(Self { field, offset })
    }

    pub const fn offset(&self) -> Vec3 {
        self.offset
    }

    pub const fn field(&self) -> &F {
        &self.field
    }

    pub fn into_field(self) -> F {
        self.field
    }
}

impl<F> SdfField3 for Translate<F>
where
    F: SdfField3,
{
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        self.field.sample(point - self.offset)
    }

    fn bounds(&self) -> FieldBounds {
        self.field.bounds().translated(self.offset)
    }

    fn capabilities(&self) -> FieldCapabilities {
        self.field.capabilities()
    }
}
