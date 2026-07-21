use glam::Vec3;

use crate::SdfField3;
use crate::error::{ensure_finite_vec3, ensure_positive};

use super::QueryError;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PointClassification {
    Inside,
    OnSurface,
    Outside,
}

pub fn classify_point<F>(
    field: &F,
    point: Vec3,
    epsilon: f32,
) -> Result<PointClassification, QueryError>
where
    F: SdfField3 + ?Sized,
{
    ensure_finite_vec3(point, "classification point")?;
    ensure_positive(epsilon, "classification epsilon")?;
    let signed_value = field.sample(point)?.signed_value();
    Ok(if signed_value < -epsilon {
        PointClassification::Inside
    } else if signed_value > epsilon {
        PointClassification::Outside
    } else {
        PointClassification::OnSurface
    })
}

pub fn is_inside<F>(field: &F, point: Vec3, epsilon: f32) -> Result<bool, QueryError>
where
    F: SdfField3 + ?Sized,
{
    Ok(classify_point(field, point, epsilon)? == PointClassification::Inside)
}

pub fn is_outside<F>(field: &F, point: Vec3, epsilon: f32) -> Result<bool, QueryError>
where
    F: SdfField3 + ?Sized,
{
    Ok(classify_point(field, point, epsilon)? == PointClassification::Outside)
}
