use glam::Vec3;

use crate::{FieldBounds, SampleError, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FieldCapabilities {
    exact_distance: bool,
}

impl FieldCapabilities {
    pub const SIGNED_FIELD: Self = Self {
        exact_distance: false,
    };

    pub const EXACT_DISTANCE: Self = Self {
        exact_distance: true,
    };

    pub const fn has_exact_distance(self) -> bool {
        self.exact_distance
    }
}

impl Default for FieldCapabilities {
    fn default() -> Self {
        Self::SIGNED_FIELD
    }
}

pub trait SdfField3 {
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError>;

    fn bounds(&self) -> FieldBounds {
        FieldBounds::Unbounded
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::SIGNED_FIELD
    }
}
