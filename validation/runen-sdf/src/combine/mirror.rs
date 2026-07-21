use glam::Vec3;

use crate::{Bounds3, FieldBounds, FieldCapabilities, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MirrorAxes {
    x: bool,
    y: bool,
    z: bool,
}

impl MirrorAxes {
    pub const NONE: Self = Self::new(false, false, false);
    pub const XYZ: Self = Self::new(true, true, true);

    pub const fn new(x: bool, y: bool, z: bool) -> Self {
        Self { x, y, z }
    }

    pub const fn x(self) -> bool {
        self.x
    }

    pub const fn y(self) -> bool {
        self.y
    }

    pub const fn z(self) -> bool {
        self.z
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Mirror<F> {
    field: F,
    axes: MirrorAxes,
}

impl<F> Mirror<F> {
    pub const fn new(field: F, axes: MirrorAxes) -> Self {
        Self { field, axes }
    }
}

impl<F> SdfField3 for Mirror<F>
where
    F: SdfField3,
{
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        let local = Vec3::new(
            if self.axes.x { point.x.abs() } else { point.x },
            if self.axes.y { point.y.abs() } else { point.y },
            if self.axes.z { point.z.abs() } else { point.z },
        );
        Ok(self.field.sample(local)?.without_safe_step())
    }

    fn bounds(&self) -> FieldBounds {
        let field_bounds = self.field.bounds();
        let FieldBounds::Bounded(bounds) = field_bounds else {
            return field_bounds;
        };
        let mut min = bounds.min();
        let mut max = bounds.max();

        if self.axes.x {
            let extent = min.x.abs().max(max.x.abs());
            min.x = -extent;
            max.x = extent;
        }
        if self.axes.y {
            let extent = min.y.abs().max(max.y.abs());
            min.y = -extent;
            max.y = extent;
        }
        if self.axes.z {
            let extent = min.z.abs().max(max.z.abs());
            min.z = -extent;
            max.z = extent;
        }

        Bounds3::try_new(min, max)
            .map(FieldBounds::Bounded)
            .unwrap_or(FieldBounds::Unbounded)
    }

    fn capabilities(&self) -> FieldCapabilities {
        FieldCapabilities::SIGNED_FIELD
    }
}
