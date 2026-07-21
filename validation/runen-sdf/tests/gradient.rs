use glam::Vec3;

use runen_sdf::gradient::{estimate_gradient, estimate_normal};
use runen_sdf::primitives::{SdfPlane, SdfSphere};
use runen_sdf::{GradientError, SampleError, SdfField3, SdfSample};

const EPS: f32 = 1e-2;

#[test]
fn sphere_gradient_points_outward() {
    let sphere = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let gradient = estimate_gradient(&sphere, Vec3::X, 1e-3).unwrap();
    assert!(gradient.dot(Vec3::X) > 0.99);
}

#[test]
fn estimated_normals_are_unit_length_for_regular_points() {
    let sphere = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let normal = estimate_normal(&sphere, Vec3::X, 1e-3).unwrap();
    assert!((normal.length() - 1.0).abs() < EPS);
    assert!(normal.dot(Vec3::X) > 0.99);
}

#[test]
fn finite_difference_is_stable_across_small_epsilon_changes() {
    let sphere = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let point = Vec3::new(0.6, 0.5, 0.2).normalize();
    let first = estimate_gradient(&sphere, point, 1e-3).unwrap().normalize();
    let second = estimate_gradient(&sphere, point, 2e-3).unwrap().normalize();
    assert!(first.dot(second) > 0.99);
}

#[test]
fn plane_gradient_aligns_with_plane_normal() {
    let plane = SdfPlane::from_point_normal(Vec3::ZERO, Vec3::Y).unwrap();
    let normal = estimate_normal(&plane, Vec3::new(3.0, 2.0, -1.0), 1e-3).unwrap();
    assert!(normal.dot(Vec3::Y) > 0.99);
}

#[test]
fn unusable_gradient_is_reported_instead_of_fabricating_world_up() {
    struct ConstantField;

    impl SdfField3 for ConstantField {
        fn sample(&self, _point: Vec3) -> Result<SdfSample, SampleError> {
            SdfSample::signed_value_only(1.0)
        }
    }

    assert_eq!(
        estimate_normal(&ConstantField, Vec3::ZERO, 1e-3),
        Err(GradientError::UnusableGradient)
    );
    assert!(estimate_gradient(&ConstantField, Vec3::ZERO, 0.0).is_err());
}
