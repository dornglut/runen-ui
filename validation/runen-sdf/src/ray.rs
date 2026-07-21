use glam::Vec3;

use crate::Bounds3;
use crate::error::{ValidationError, ensure_finite_vec3};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Ray3 {
    origin: Vec3,
    direction: Vec3,
}

impl Ray3 {
    pub fn try_new(origin: Vec3, direction: Vec3) -> Result<Self, ValidationError> {
        ensure_finite_vec3(origin, "ray origin")?;
        ensure_finite_vec3(direction, "ray direction")?;
        let length_squared = direction.length_squared();
        if !length_squared.is_finite() || length_squared <= f32::EPSILON {
            return Err(ValidationError::ZeroVector {
                parameter: "ray direction",
            });
        }
        Ok(Self {
            origin,
            direction: direction / length_squared.sqrt(),
        })
    }

    pub const fn origin(self) -> Vec3 {
        self.origin
    }

    pub const fn direction(self) -> Vec3 {
        self.direction
    }

    pub fn point_at(self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }

    pub fn interval_in_bounds(self, bounds: Bounds3, max_distance: f32) -> Option<(f32, f32)> {
        let mut minimum = 0.0;
        let mut maximum = max_distance;
        let bounds_min = bounds.min();
        let bounds_max = bounds.max();

        if !update_interval(
            self.origin.x,
            self.direction.x,
            bounds_min.x,
            bounds_max.x,
            &mut minimum,
            &mut maximum,
        ) || !update_interval(
            self.origin.y,
            self.direction.y,
            bounds_min.y,
            bounds_max.y,
            &mut minimum,
            &mut maximum,
        ) || !update_interval(
            self.origin.z,
            self.direction.z,
            bounds_min.z,
            bounds_max.z,
            &mut minimum,
            &mut maximum,
        ) {
            return None;
        }

        (maximum >= minimum && maximum >= 0.0).then_some((minimum.max(0.0), maximum))
    }
}

fn update_interval(
    origin: f32,
    direction: f32,
    slab_min: f32,
    slab_max: f32,
    minimum: &mut f32,
    maximum: &mut f32,
) -> bool {
    if direction.abs() <= f32::EPSILON {
        return origin >= slab_min && origin <= slab_max;
    }

    let inverse = 1.0 / direction;
    let mut near = (slab_min - origin) * inverse;
    let mut far = (slab_max - origin) * inverse;
    if near > far {
        core::mem::swap(&mut near, &mut far);
    }

    *minimum = (*minimum).max(near);
    *maximum = (*maximum).min(far);
    *maximum >= *minimum
}
