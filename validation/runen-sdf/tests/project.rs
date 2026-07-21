use glam::Vec3;

use runen_sdf::primitives::{SdfPlane, SdfSphere};
use runen_sdf::queries::project::{ProjectSettings, project_point_to_surface};
use runen_sdf::queries::{QueryError, QueryOutcome, QueryTermination};
use runen_sdf::{SampleError, SdfField3, SdfSample};

#[test]
fn project_from_outside_and_inside_converges_to_surface() {
    let field = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();

    let QueryOutcome::Hit(outside) =
        project_point_to_surface(&field, Vec3::new(2.5, 0.0, 0.0), ProjectSettings::default())
            .unwrap()
    else {
        panic!("outside projection should hit");
    };
    assert!(outside.position.distance(Vec3::X) < 1e-3);
    assert!(outside.signed_value.abs() < 1e-3);

    let QueryOutcome::Hit(inside) =
        project_point_to_surface(&field, Vec3::new(0.2, 0.0, 0.0), ProjectSettings::default())
            .unwrap()
    else {
        panic!("inside projection should hit");
    };
    assert!((inside.position.length() - 1.0).abs() < 1e-3);
}

#[test]
fn projection_reports_convergence_budget_exhaustion() {
    let field = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let settings = ProjectSettings::try_new(1, 1e-4, 1e-3, 0.01).unwrap();

    assert_eq!(
        project_point_to_surface(&field, Vec3::new(10.0, 0.0, 0.0), settings).unwrap(),
        QueryOutcome::Miss(QueryTermination::ConvergenceBudgetExhausted)
    );
}

#[test]
fn project_on_plane_is_accurate() {
    let field = SdfPlane::from_point_normal(Vec3::ZERO, Vec3::Y).unwrap();
    let QueryOutcome::Hit(hit) =
        project_point_to_surface(&field, Vec3::new(1.0, 3.0, 2.0), ProjectSettings::default())
            .unwrap()
    else {
        panic!("plane projection should hit");
    };
    assert!(hit.position.distance(Vec3::new(1.0, 0.0, 2.0)) < 1e-3);
}

#[test]
fn projection_rejects_sign_only_fields() {
    struct SignOnlySphere;

    impl SdfField3 for SignOnlySphere {
        fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
            SdfSample::signed_value_only(point.length() - 1.0)
        }
    }

    assert!(matches!(
        project_point_to_surface(
            &SignOnlySphere,
            Vec3::new(2.0, 0.0, 0.0),
            ProjectSettings::default(),
        ),
        Err(QueryError::UnsupportedCapability { .. })
    ));
}
