use glam::Vec3;

use runen_sdf::primitives::SdfSphere;
use runen_sdf::queries::raymarch::{RaymarchSettings, raymarch_first_hit};
use runen_sdf::queries::{QueryError, QueryOutcome, QueryTermination};
use runen_sdf::{FieldBounds, Ray3, SampleError, SdfField3, SdfSample};

#[test]
fn raymarch_hits_sphere_without_overshoot() {
    let field = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let ray = Ray3::try_new(Vec3::new(-3.0, 0.0, 0.0), Vec3::X).unwrap();
    let settings = RaymarchSettings::try_new(128, 10.0, 1e-3).unwrap();

    let QueryOutcome::Hit(hit) = raymarch_first_hit(&field, &ray, settings).unwrap() else {
        panic!("ray should hit sphere");
    };
    assert!((hit.distance_along_ray - 2.0).abs() < 1e-2);
    assert!(hit.position.distance(Vec3::new(-1.0, 0.0, 0.0)) < 1e-2);
    assert!(hit.distance_along_ray <= 2.0 + 1e-3);
}

#[test]
fn raymarch_distinguishes_bounds_and_distance_misses() {
    let field = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let miss_ray = Ray3::try_new(Vec3::new(-3.0, 3.0, 0.0), Vec3::X).unwrap();
    assert_eq!(
        raymarch_first_hit(
            &field,
            &miss_ray,
            RaymarchSettings::try_new(128, 10.0, 1e-3).unwrap(),
        )
        .unwrap(),
        QueryOutcome::Miss(QueryTermination::OutsideBounds)
    );

    let short_ray = Ray3::try_new(Vec3::new(-3.0, 0.0, 0.0), Vec3::X).unwrap();
    assert_eq!(
        raymarch_first_hit(
            &field,
            &short_ray,
            RaymarchSettings::try_new(128, 1.5, 1e-3).unwrap(),
        )
        .unwrap(),
        QueryOutcome::Miss(QueryTermination::MaxDistanceReached)
    );
}

#[test]
fn raymarch_reports_step_budget_exhaustion() {
    struct UnboundedSphere;

    impl SdfField3 for UnboundedSphere {
        fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
            SdfSample::exact_signed_distance(point.length() - 1.0)
        }

        fn bounds(&self) -> FieldBounds {
            FieldBounds::Unbounded
        }
    }

    let ray = Ray3::try_new(Vec3::new(-3.0, 0.0, 0.0), Vec3::X).unwrap();
    assert_eq!(
        raymarch_first_hit(
            &UnboundedSphere,
            &ray,
            RaymarchSettings::try_new(1, 10.0, 1e-3).unwrap(),
        )
        .unwrap(),
        QueryOutcome::Miss(QueryTermination::StepBudgetExhausted)
    );
}

#[test]
fn raymarch_hits_immediately_when_starting_inside() {
    let field = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let ray = Ray3::try_new(Vec3::ZERO, Vec3::X).unwrap();
    let QueryOutcome::Hit(hit) = raymarch_first_hit(
        &field,
        &ray,
        RaymarchSettings::try_new(128, 10.0, 1e-3).unwrap(),
    )
    .unwrap() else {
        panic!("inside point should register a hit");
    };
    assert!(hit.distance_along_ray <= f32::EPSILON);
}

#[test]
fn raymarch_rejects_fields_without_safe_steps() {
    struct SignOnlyField;

    impl SdfField3 for SignOnlyField {
        fn sample(&self, _point: Vec3) -> Result<SdfSample, SampleError> {
            SdfSample::signed_value_only(1.0)
        }
    }

    let ray = Ray3::try_new(Vec3::ZERO, Vec3::X).unwrap();
    assert!(matches!(
        raymarch_first_hit(
            &SignOnlyField,
            &ray,
            RaymarchSettings::try_new(8, 10.0, 1e-3).unwrap(),
        ),
        Err(QueryError::UnsupportedCapability { .. })
    ));
}
