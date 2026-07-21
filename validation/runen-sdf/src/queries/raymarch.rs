use glam::Vec3;

use crate::epsilon::{
    DEFAULT_MAX_RAYMARCH_STEPS, DEFAULT_RAY_HIT_EPSILON, DEFAULT_RAY_MAX_DISTANCE,
};
use crate::error::{ValidationError, ensure_non_negative, ensure_positive};
use crate::{FieldBounds, Ray3, SdfField3};

use super::{QueryError, QueryOutcome, QueryTermination};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RaymarchSettings {
    max_steps: u32,
    max_distance: f32,
    surface_epsilon: f32,
}

impl RaymarchSettings {
    pub fn try_new(
        max_steps: u32,
        max_distance: f32,
        surface_epsilon: f32,
    ) -> Result<Self, ValidationError> {
        if max_steps == 0 {
            return Err(ValidationError::NonPositive {
                parameter: "raymarch max steps",
                value: 0.0,
            });
        }
        ensure_non_negative(max_distance, "raymarch max distance")?;
        ensure_positive(surface_epsilon, "raymarch surface epsilon")?;
        Ok(Self {
            max_steps,
            max_distance,
            surface_epsilon,
        })
    }

    pub const fn max_steps(self) -> u32 {
        self.max_steps
    }

    pub const fn max_distance(self) -> f32 {
        self.max_distance
    }

    pub const fn surface_epsilon(self) -> f32 {
        self.surface_epsilon
    }
}

impl Default for RaymarchSettings {
    fn default() -> Self {
        Self {
            max_steps: DEFAULT_MAX_RAYMARCH_STEPS,
            max_distance: DEFAULT_RAY_MAX_DISTANCE,
            surface_epsilon: DEFAULT_RAY_HIT_EPSILON,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RayHit {
    pub distance_along_ray: f32,
    pub position: Vec3,
    pub signed_value: f32,
    pub steps: u32,
}

pub fn raymarch_first_hit<F>(
    field: &F,
    ray: &Ray3,
    settings: RaymarchSettings,
) -> Result<QueryOutcome<RayHit>, QueryError>
where
    F: SdfField3 + ?Sized,
{
    let (mut distance, limit, bounded) = match field.bounds() {
        FieldBounds::Empty => {
            return Ok(QueryOutcome::Miss(QueryTermination::SurfaceRuledOut));
        }
        FieldBounds::Unbounded => (0.0, settings.max_distance, false),
        FieldBounds::Bounded(bounds) => {
            let Some((entry, exit)) = ray.interval_in_bounds(bounds, f32::MAX) else {
                return Ok(QueryOutcome::Miss(QueryTermination::OutsideBounds));
            };
            if entry > settings.max_distance {
                return Ok(QueryOutcome::Miss(QueryTermination::MaxDistanceReached));
            }
            (
                entry,
                exit.min(settings.max_distance),
                exit <= settings.max_distance,
            )
        }
    };

    for step_index in 0..settings.max_steps {
        if distance > limit {
            return Ok(QueryOutcome::Miss(if bounded {
                QueryTermination::OutsideBounds
            } else {
                QueryTermination::MaxDistanceReached
            }));
        }

        let position = ray.point_at(distance);
        let sample = field.sample(position)?;
        if sample.signed_value() <= settings.surface_epsilon {
            return Ok(QueryOutcome::Hit(RayHit {
                distance_along_ray: distance,
                position,
                signed_value: sample.signed_value(),
                steps: step_index + 1,
            }));
        }

        let Some(safe_step) = sample.safe_step() else {
            return Err(QueryError::UnsupportedCapability {
                capability: "conservative sphere-tracing step",
            });
        };
        if safe_step <= f32::EPSILON {
            return Ok(QueryOutcome::Miss(QueryTermination::InsufficientProgress));
        }

        let next = distance + safe_step;
        if !next.is_finite() {
            return Ok(QueryOutcome::Miss(QueryTermination::MaxDistanceReached));
        }
        distance = next;
    }

    Ok(QueryOutcome::Miss(QueryTermination::StepBudgetExhausted))
}
