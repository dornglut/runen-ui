#![forbid(unsafe_code)]

use glam::Vec3;
use runen_sdf::{FieldBounds, SampleError, SdfField3, SdfSample};

#[derive(Debug, Copy, Clone)]
pub struct ExternalPlane {
    height: f32,
}

impl ExternalPlane {
    pub fn new(height: f32) -> Option<Self> {
        height.is_finite().then_some(Self { height })
    }
}

impl SdfField3 for ExternalPlane {
    fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
        SdfSample::signed_value_only(point.y - self.height)
    }

    fn bounds(&self) -> FieldBounds {
        FieldBounds::Unbounded
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;
    use runen_sdf::primitives::SdfSphere;
    use runen_sdf::queries::project::{ProjectSettings, project_point_to_surface};
    use runen_sdf::queries::raymarch::{RaymarchSettings, raymarch_first_hit};
    use runen_sdf::queries::{QueryError, QueryOutcome};
    use runen_sdf::{Ray3, SdfField3};

    use super::ExternalPlane;

    #[test]
    fn external_field_implements_and_uses_public_contracts() {
        let plane = ExternalPlane::new(2.0).expect("finite height");
        let field: &dyn SdfField3 = &plane;
        let sample = field
            .sample(Vec3::new(1.0, -3.0, 4.0))
            .expect("finite public sample");

        assert_eq!(sample.signed_value(), -5.0);
        assert_eq!(sample.safe_step(), None);
        assert!(!field.capabilities().has_exact_distance());
    }

    #[test]
    fn external_field_rejects_non_finite_construction() {
        assert!(ExternalPlane::new(f32::NAN).is_none());
        assert!(ExternalPlane::new(f32::INFINITY).is_none());
    }

    #[test]
    fn tracing_rejects_external_sign_only_field() {
        let plane = ExternalPlane::new(0.0).expect("finite height");
        let ray = Ray3::try_new(Vec3::Y, -Vec3::Y).expect("valid ray");
        let result = raymarch_first_hit(&plane, &ray, RaymarchSettings::default());

        assert!(matches!(
            result,
            Err(QueryError::UnsupportedCapability { .. })
        ));
    }

    #[test]
    fn downstream_consumer_uses_successful_public_query() {
        let sphere = SdfSphere::new(Vec3::ZERO, 1.0).expect("valid sphere");
        let result = project_point_to_surface(
            &sphere,
            Vec3::new(2.0, 0.0, 0.0),
            ProjectSettings::default(),
        )
        .expect("query should execute");

        let QueryOutcome::Hit(hit) = result else {
            panic!("public projection should hit");
        };
        assert!(hit.position.distance(Vec3::X) < 1e-3);
    }
}
