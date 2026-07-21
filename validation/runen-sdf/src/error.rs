use glam::Vec3;
use thiserror::Error;

#[derive(Debug, Copy, Clone, PartialEq, Error)]
pub enum ValidationError {
    #[error("{parameter} must be finite")]
    NonFinite { parameter: &'static str },
    #[error("{parameter} must be non-negative (got {value})")]
    Negative { parameter: &'static str, value: f32 },
    #[error("{parameter} must be greater than zero (got {value})")]
    NonPositive { parameter: &'static str, value: f32 },
    #[error("{parameter} must not be the zero vector")]
    ZeroVector { parameter: &'static str },
    #[error("minimum must not exceed maximum")]
    InvalidRange,
    #[error("bounds require component-wise min <= max")]
    InvalidBounds,
    #[error("affine transform must be finite and invertible")]
    SingularTransform,
}

#[derive(Debug, Copy, Clone, PartialEq, Error)]
pub enum SampleError {
    #[error("sample point must be finite")]
    NonFinitePoint,
    #[error("signed field value must be finite")]
    NonFiniteSignedValue,
    #[error("safe step must be finite")]
    NonFiniteSafeStep,
    #[error("safe step must be non-negative")]
    NegativeSafeStep,
}

#[derive(Debug, Copy, Clone, PartialEq, Error)]
pub enum GradientError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Sample(#[from] SampleError),
    #[error("finite-difference gradient is non-finite")]
    NonFiniteGradient,
    #[error("gradient is too small to define a stable normal")]
    UnusableGradient,
}

pub(crate) fn ensure_finite_scalar(
    value: f32,
    parameter: &'static str,
) -> Result<(), ValidationError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ValidationError::NonFinite { parameter })
    }
}

pub(crate) fn ensure_non_negative(
    value: f32,
    parameter: &'static str,
) -> Result<(), ValidationError> {
    ensure_finite_scalar(value, parameter)?;
    if value < 0.0 {
        Err(ValidationError::Negative { parameter, value })
    } else {
        Ok(())
    }
}

pub(crate) fn ensure_positive(value: f32, parameter: &'static str) -> Result<(), ValidationError> {
    ensure_finite_scalar(value, parameter)?;
    if value <= 0.0 {
        Err(ValidationError::NonPositive { parameter, value })
    } else {
        Ok(())
    }
}

pub(crate) fn ensure_finite_vec3(
    value: Vec3,
    parameter: &'static str,
) -> Result<(), ValidationError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ValidationError::NonFinite { parameter })
    }
}

pub(crate) fn ensure_sample_point(point: Vec3) -> Result<(), SampleError> {
    if point.is_finite() {
        Ok(())
    } else {
        Err(SampleError::NonFinitePoint)
    }
}
