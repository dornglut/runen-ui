use glam::Vec3;

use crate::epsilon::DEFAULT_NORMAL_EPSILON;
use crate::{GradientError, SdfField3};

pub fn normal_at<F>(field: &F, point: Vec3, epsilon: f32) -> Result<Vec3, GradientError>
where
    F: SdfField3 + ?Sized,
{
    crate::gradient::estimate_normal(field, point, epsilon)
}

pub fn normal_at_default<F>(field: &F, point: Vec3) -> Result<Vec3, GradientError>
where
    F: SdfField3 + ?Sized,
{
    normal_at(field, point, DEFAULT_NORMAL_EPSILON)
}
