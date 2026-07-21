use glam::Vec3;

use crate::SdfField3;
use crate::epsilon::{DEFAULT_MAX_PROJECT_STEPS, DEFAULT_NORMAL_EPSILON, DEFAULT_PROJECT_EPSILON};
use crate::error::{ValidationError, ensure_finite_vec3, ensure_positive};
use crate::gradient::estimate_normal;

use super::{QueryError, QueryOutcome, QueryTermination, require_exact_distance};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ProjectSettings {
    max_steps: u32,
    surface_epsilon: f32,
    normal_epsilon: f32,
    max_step: f32,
}

impl ProjectSettings {
    pub fn try_new(
        max_steps: u32,
        surface_epsilon: f32,
        normal_epsilon: f32,
        max_step: f32,
    ) -> Result<Self, ValidationError> {
        if max_steps == 0 {
            return Err(ValidationError::NonPositive {
                parameter: "project max steps",
                value: 0.0,
            });
        }
        ensure_positive(surface_epsilon, "project surface epsilon")?;
        ensure_positive(normal_epsilon, "project normal epsilon")?;
        ensure_positive(max_step, "project max step")?;
        Ok(Self {
            max_steps,
            surface_epsilon,
            normal_epsilon,
            max_step,
        })
    }

    pub const fn max_steps(self) -> u32 {
        self.max_steps
    }

    pub const fn surface_epsilon(self) -> f32 {
        self.surface_epsilon
    }

    pub const fn normal_epsilon(self) -> f32 {
        self.normal_epsilon
    }

    pub const fn max_step(self) -> f32 {
        self.max_step
    }
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            max_steps: DEFAULT_MAX_PROJECT_STEPS,
            surface_epsilon: DEFAULT_PROJECT_EPSILON,
            normal_epsilon: DEFAULT_NORMAL_EPSILON,
            max_step: 1.0,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ProjectHit {
    pub position: Vec3,
    pub normal: Vec3,
    pub signed_value: f32,
    pub iterations: u32,
}

pub fn project_point_to_surface<F>(
    field: &F,
    start: Vec3,
    settings: ProjectSettings,
) -> Result<QueryOutcome<ProjectHit>, QueryError>
where
    F: SdfField3 + ?Sized,
{
    require_exact_distance(field)?;
    ensure_finite_vec3(start, "project start")?;

    let mut point = start;
    for iteration in 0..settings.max_steps {
        let sample = field.sample(point)?;
        let signed_value = sample.signed_value();
        if signed_value.abs() <= settings.surface_epsilon {
            let normal = estimate_normal(field, point, settings.normal_epsilon)?;
            return Ok(QueryOutcome::Hit(ProjectHit {
                position: point,
                normal,
                signed_value,
                iterations: iteration + 1,
            }));
        }

        let normal = estimate_normal(field, point, settings.normal_epsilon)?;
        let step = (-signed_value).clamp(-settings.max_step, settings.max_step);
        if step.abs() <= f32::EPSILON {
            return Ok(QueryOutcome::Miss(QueryTermination::InsufficientProgress));
        }
        point += normal * step;
        if !point.is_finite() {
            return Err(QueryError::Validation(ValidationError::NonFinite {
                parameter: "projected point",
            }));
        }
    }

    Ok(QueryOutcome::Miss(
        QueryTermination::ConvergenceBudgetExhausted,
    ))
}
