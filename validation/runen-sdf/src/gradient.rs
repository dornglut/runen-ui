use glam::Vec3;

use crate::SdfField3;
use crate::epsilon::DEFAULT_NORMAL_EPSILON;
use crate::error::{GradientError, ensure_finite_vec3, ensure_positive};
use crate::util::finite_difference::central_difference;

pub fn estimate_gradient<F>(field: &F, point: Vec3, epsilon: f32) -> Result<Vec3, GradientError>
where
    F: SdfField3 + ?Sized,
{
    ensure_finite_vec3(point, "gradient point")?;
    ensure_positive(epsilon, "gradient epsilon")?;

    let gradient = central_difference(
        |sample_point| -> Result<f32, GradientError> {
            Ok(field.sample(sample_point)?.signed_value())
        },
        point,
        epsilon,
    )?;
    if !gradient.is_finite() {
        return Err(GradientError::NonFiniteGradient);
    }
    Ok(gradient)
}

pub fn estimate_gradient_default<F>(field: &F, point: Vec3) -> Result<Vec3, GradientError>
where
    F: SdfField3 + ?Sized,
{
    estimate_gradient(field, point, DEFAULT_NORMAL_EPSILON)
}

pub fn estimate_normal<F>(field: &F, point: Vec3, epsilon: f32) -> Result<Vec3, GradientError>
where
    F: SdfField3 + ?Sized,
{
    let gradient = estimate_gradient(field, point, epsilon)?;
    let length_squared = gradient.length_squared();
    if !length_squared.is_finite() {
        return Err(GradientError::NonFiniteGradient);
    }
    if length_squared <= f32::EPSILON {
        return Err(GradientError::UnusableGradient);
    }
    Ok(gradient / length_squared.sqrt())
}

pub fn estimate_normal_default<F>(field: &F, point: Vec3) -> Result<Vec3, GradientError>
where
    F: SdfField3 + ?Sized,
{
    estimate_normal(field, point, DEFAULT_NORMAL_EPSILON)
}
