use glam::Vec3;

use runen_sdf::combine::{DomainWarp, Mirror, MirrorAxes, Repeat};
use runen_sdf::ops::Union;
use runen_sdf::primitives::{SdfCapsule, SdfPlane, SdfSphere};
use runen_sdf::queries::QueryOutcome;
use runen_sdf::queries::sweep::{SweepSettings, sweep_sphere};
use runen_sdf::{FieldBounds, Ray3, SampleError, SdfField3, SdfSample};

#[test]
fn degenerate_capsule_behaves_like_sphere() {
    let capsule = SdfCapsule::new(Vec3::ZERO, Vec3::ZERO, 1.0).unwrap();
    let sphere = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let point = Vec3::new(1.5, -0.25, 0.5);

    let capsule_value = capsule.sample(point).unwrap().signed_value();
    let sphere_value = sphere.sample(point).unwrap().signed_value();
    assert!((capsule_value - sphere_value).abs() < 1e-5);
}

#[test]
fn unbounded_fields_propagate_through_union_bounds() {
    let bounded = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let unbounded = SdfPlane::from_point_normal(Vec3::ZERO, Vec3::Y).unwrap();
    assert_eq!(
        Union::new(bounded, unbounded).bounds(),
        FieldBounds::Unbounded
    );
}

#[test]
fn invalid_ray_direction_is_rejected_at_construction() {
    assert!(Ray3::try_new(Vec3::ZERO, Vec3::ZERO).is_err());
    assert!(Ray3::try_new(Vec3::splat(f32::NAN), Vec3::X).is_err());
}

#[test]
fn sweep_sphere_reports_first_contact() {
    let field = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let QueryOutcome::Hit(hit) = sweep_sphere(
        &field,
        Vec3::new(-3.0, 0.0, 0.0),
        Vec3::new(3.0, 0.0, 0.0),
        0.5,
        SweepSettings::default(),
    )
    .unwrap() else {
        panic!("sweep should hit");
    };

    assert!(hit.t < 0.5);
    assert!(hit.normal.dot(Vec3::new(-1.0, 0.0, 0.0)) > 0.8);
}

#[test]
fn domain_wrappers_preserve_sign_but_drop_unproven_steps() {
    let base = SdfSphere::new(Vec3::ZERO, 0.5).unwrap();
    let repeated = Repeat::new(base, Vec3::new(2.0, 0.0, 0.0)).unwrap();
    let mirrored = Mirror::new(repeated, MirrorAxes::new(true, false, false));
    let warped = DomainWarp::new(
        mirrored,
        Vec3::new(0.1, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::ZERO,
    )
    .unwrap();

    let sample = warped.sample(Vec3::new(1.25, 0.0, 0.0)).unwrap();
    assert!(sample.signed_value().is_finite());
    assert_eq!(sample.safe_step(), None);
    assert!(!warped.capabilities().has_exact_distance());
}

#[test]
fn downstream_style_fields_and_trait_objects_use_public_contracts() {
    struct PublicField;

    impl SdfField3 for PublicField {
        fn sample(&self, point: Vec3) -> Result<SdfSample, SampleError> {
            SdfSample::signed_value_only(point.y)
        }
    }

    let field: &dyn SdfField3 = &PublicField;
    let sample = field.sample(Vec3::new(1.0, -2.0, 3.0)).unwrap();
    assert_eq!(sample.signed_value(), -2.0);
    assert_eq!(sample.safe_step(), None);
}
