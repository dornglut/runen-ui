use glam::Vec3;

use crate::sample::minimum_safe_step;
use crate::{FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Subtract<A, B> {
    left: A,
    right: B,
}

impl<A, B> Subtract<A, B> {
    pub const fn new(left: A, right: B) -> Self {
        Self { left, right }
    }

    pub const fn left(&self) -> &A {
        &self.left
    }

    pub const fn right(&self) -> &B {
        &self.right
    }
}

impl<A, B> SdfField3 for Subtract<A, B>
where
    A: SdfField3,
    B: SdfField3,
{
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        let left = self.left.sample(point)?;
        let right = self.right.sample(point)?;
        SdfSample::from_parts(
            left.signed_value().max(-right.signed_value()),
            minimum_safe_step(left, right),
        )
    }

    fn bounds(&self) -> FieldBounds {
        self.left.bounds()
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::SIGNED_FIELD
    }
}
