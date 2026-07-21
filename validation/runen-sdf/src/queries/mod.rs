pub mod classify;
pub mod closest_point;
pub mod project;
pub mod raymarch;
pub mod sweep;

use thiserror::Error;

use crate::{GradientError, SampleError, SdfField3, ValidationError};

pub use classify::PointClassification;
pub use closest_point::{ClosestPointHit, closest_point_on_surface};
pub use project::{ProjectHit, ProjectSettings, project_point_to_surface};
pub use raymarch::{RayHit, RaymarchSettings, raymarch_first_hit};
pub use sweep::{SweepHit, SweepSettings, sweep_sphere};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum QueryOutcome<T> {
    Hit(T),
    Miss(QueryTermination),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum QueryTermination {
    OutsideBounds,
    SurfaceRuledOut,
    MaxDistanceReached,
    StepBudgetExhausted,
    ConvergenceBudgetExhausted,
    InsufficientProgress,
}

#[derive(Debug, Copy, Clone, PartialEq, Error)]
pub enum QueryError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Sample(#[from] SampleError),
    #[error(transparent)]
    Gradient(#[from] GradientError),
    #[error("field does not support {capability}")]
    UnsupportedCapability { capability: &'static str },
}

pub(crate) fn require_exact_distance<F>(field: &F) -> Result<(), QueryError>
where
    F: SdfField3 + ?Sized,
{
    if field.capabilities().has_exact_distance() {
        Ok(())
    } else {
        Err(QueryError::UnsupportedCapability {
            capability: "exact signed distance",
        })
    }
}
