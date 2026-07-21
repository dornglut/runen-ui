use glam::Vec3;

use crate::SdfField3;

use super::project::{ProjectSettings, project_point_to_surface};
use super::{QueryError, QueryOutcome};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ClosestPointHit {
    pub point: Vec3,
    pub normal: Vec3,
    pub signed_value: f32,
    pub iterations: u32,
}

pub fn closest_point_on_surface<F>(
    field: &F,
    start: Vec3,
    settings: ProjectSettings,
) -> Result<QueryOutcome<ClosestPointHit>, QueryError>
where
    F: SdfField3 + ?Sized,
{
    Ok(match project_point_to_surface(field, start, settings)? {
        QueryOutcome::Hit(hit) => QueryOutcome::Hit(ClosestPointHit {
            point: hit.position,
            normal: hit.normal,
            signed_value: hit.signed_value,
            iterations: hit.iterations,
        }),
        QueryOutcome::Miss(reason) => QueryOutcome::Miss(reason),
    })
}
