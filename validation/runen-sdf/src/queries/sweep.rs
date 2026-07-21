use glam::Vec3;

use crate::SdfField3;
use crate::epsilon::{DEFAULT_CLASSIFY_EPSILON, DEFAULT_MAX_SWEEP_STEPS, DEFAULT_NORMAL_EPSILON};
use crate::error::{ValidationError, ensure_finite_vec3, ensure_non_negative, ensure_positive};
use crate::gradient::estimate_normal;

use super::{QueryError, QueryOutcome, QueryTermination, require_exact_distance};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SweepSettings {
    max_steps: u32,
    surface_epsilon: f32,
    normal_epsilon: f32,
}

impl SweepSettings {
    pub fn try_new(
        max_steps: u32,
        surface_epsilon: f32,
        normal_epsilon: f32,
    ) -> Result<Self, ValidationError> {
        if max_steps == 0 {
            return Err(ValidationError::NonPositive {
                parameter: "sweep max steps",
                value: 0.0,
            });
        }
        ensure_positive(surface_epsilon, "sweep surface epsilon")?;
        ensure_positive(normal_epsilon, "sweep normal epsilon")?;
        Ok(Self {
            max_steps,
            surface_epsilon,
            normal_epsilon,
        })
    }
}

impl Default for SweepSettings {
    fn default() -> Self {
        Self {
            max_steps: DEFAULT_MAX_SWEEP_STEPS,
            surface_epsilon: DEFAULT_CLASSIFY_EPSILON,
            normal_epsilon: DEFAULT_NORMAL_EPSILON,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SweepHit {
    pub t: f32,
    pub position: Vec3,
    pub normal: Vec3,
    pub penetration: f32,
    pub steps: u32,
}

pub fn sweep_sphere<F>(
    field: &F,
    start: Vec3,
    end: Vec3,
    radius: f32,
    settings: SweepSettings,
) -> Result<QueryOutcome<SweepHit>, QueryError>
where
    F: SdfField3 + ?Sized,
{
    require_exact_distance(field)?;
    ensure_finite_vec3(start, "sweep start")?;
    ensure_finite_vec3(end, "sweep end")?;
    ensure_non_negative(radius, "sweep radius")?;

    let delta = end - start;
    let path_length = delta.length();
    if !path_length.is_finite() {
        return Err(QueryError::Validation(ValidationError::NonFinite {
            parameter: "sweep path length",
        }));
    }

    if path_length <= f32::EPSILON {
        return sample_contact(field, start, radius, 0.0, 1, settings);
    }

    let direction = delta / path_length;
    let mut travelled = 0.0;
    for step_index in 0..settings.max_steps {
        let position = start + direction * travelled;
        let sample = field.sample(position)?;
        let clearance = sample.signed_value() - radius;
        if clearance <= settings.surface_epsilon {
            let normal = estimate_normal(field, position, settings.normal_epsilon)?;
            return Ok(QueryOutcome::Hit(SweepHit {
                t: travelled / path_length,
                position,
                normal,
                penetration: (-clearance).max(0.0),
                steps: step_index + 1,
            }));
        }
        if clearance <= f32::EPSILON {
            return Ok(QueryOutcome::Miss(QueryTermination::InsufficientProgress));
        }

        travelled += clearance;
        if !travelled.is_finite() || travelled > path_length {
            return Ok(QueryOutcome::Miss(QueryTermination::SurfaceRuledOut));
        }
    }

    Ok(QueryOutcome::Miss(QueryTermination::StepBudgetExhausted))
}

fn sample_contact<F>(
    field: &F,
    position: Vec3,
    radius: f32,
    t: f32,
    steps: u32,
    settings: SweepSettings,
) -> Result<QueryOutcome<SweepHit>, QueryError>
where
    F: SdfField3 + ?Sized,
{
    let sample = field.sample(position)?;
    let clearance = sample.signed_value() - radius;
    if clearance > settings.surface_epsilon {
        return Ok(QueryOutcome::Miss(QueryTermination::SurfaceRuledOut));
    }
    let normal = estimate_normal(field, position, settings.normal_epsilon)?;
    Ok(QueryOutcome::Hit(SweepHit {
        t,
        position,
        normal,
        penetration: (-clearance).max(0.0),
        steps,
    }))
}
