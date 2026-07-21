use glam::Vec3;

use runen_sdf::primitives::{SdfBox3, SdfCapsule, SdfCylinder, SdfPlane, SdfSphere, SdfTorus};
use runen_sdf::{FieldBounds, SdfField3, ValidationError};

const EPS: f32 = 1e-4;

#[test]
fn sphere_signed_distance_safe_step_and_bounds_are_correct() {
    let sphere = SdfSphere::new(Vec3::ZERO, 1.0).expect("valid sphere");

    let center = sphere.sample(Vec3::ZERO).expect("finite sample");
    assert!((center.signed_value() + 1.0).abs() < EPS);
    assert!((center.safe_step().expect("exact step") - 1.0).abs() < EPS);

    let surface = sphere
        .sample(Vec3::new(1.0, 0.0, 0.0))
        .expect("finite sample");
    assert!(surface.signed_value().abs() < EPS);

    let FieldBounds::Bounded(bounds) = sphere.bounds() else {
        panic!("sphere should be bounded");
    };
    assert_eq!(bounds.min(), Vec3::splat(-1.0));
    assert_eq!(bounds.max(), Vec3::splat(1.0));
    assert!(sphere.capabilities().has_exact_distance());
}

#[test]
fn box_and_capsule_match_expected_samples() {
    let box3 = SdfBox3::new(Vec3::ZERO, Vec3::new(1.0, 2.0, 3.0)).expect("valid box");
    assert!((box3.sample(Vec3::ZERO).unwrap().signed_value() + 1.0).abs() < EPS);
    assert!(
        box3.sample(Vec3::new(1.0, 0.0, 0.0))
            .unwrap()
            .signed_value()
            .abs()
            < EPS
    );

    let capsule = SdfCapsule::new(Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.5)
        .expect("valid capsule");
    assert!((capsule.sample(Vec3::ZERO).unwrap().signed_value() + 0.5).abs() < EPS);

    let FieldBounds::Bounded(bounds) = capsule.bounds() else {
        panic!("capsule should be bounded");
    };
    assert_eq!(bounds.min(), Vec3::new(-0.5, -1.5, -0.5));
    assert_eq!(bounds.max(), Vec3::new(0.5, 1.5, 0.5));
}

#[test]
fn plane_torus_and_cylinder_are_exact_fields() {
    let plane = SdfPlane::from_point_normal(Vec3::ZERO, Vec3::Y).expect("valid plane");
    assert!(
        (plane
            .sample(Vec3::new(0.0, 2.0, 0.0))
            .unwrap()
            .signed_value()
            - 2.0)
            .abs()
            < EPS
    );
    assert_eq!(plane.bounds(), FieldBounds::Unbounded);

    let torus = SdfTorus::new(Vec3::ZERO, 2.0, 0.5).expect("valid torus");
    assert!(
        torus
            .sample(Vec3::new(2.5, 0.0, 0.0))
            .unwrap()
            .signed_value()
            .abs()
            < EPS
    );

    let cylinder = SdfCylinder::new(Vec3::ZERO, 1.0, 2.0).expect("valid cylinder");
    assert!((cylinder.sample(Vec3::ZERO).unwrap().signed_value() + 1.0).abs() < EPS);
    assert!(plane.capabilities().has_exact_distance());
    assert!(torus.capabilities().has_exact_distance());
    assert!(cylinder.capabilities().has_exact_distance());
}

#[test]
fn invalid_authored_primitive_state_is_rejected() {
    assert!(matches!(
        SdfSphere::new(Vec3::ZERO, -1.0),
        Err(ValidationError::Negative { .. })
    ));
    assert!(SdfBox3::new(Vec3::ZERO, Vec3::new(1.0, -1.0, 1.0)).is_err());
    assert!(SdfPlane::new(Vec3::ZERO, 0.0).is_err());
    assert!(SdfTorus::new(Vec3::ZERO, f32::INFINITY, 1.0).is_err());
}
