use glam::Vec3;

use runen_sdf::normal::normal_at_default;
use runen_sdf::primitives::{SdfPlane, SdfSphere};

#[test]
fn sphere_normal_points_outward() {
    let field = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let normal = normal_at_default(&field, Vec3::X).unwrap();
    assert!(normal.dot(Vec3::X) > 0.99);
}

#[test]
fn plane_normal_is_stable() {
    let field = SdfPlane::from_point_normal(Vec3::ZERO, Vec3::Y).unwrap();
    let normal = normal_at_default(&field, Vec3::new(2.0, 3.0, -1.0)).unwrap();
    assert!(normal.dot(Vec3::Y) > 0.99);
}

#[test]
fn nearby_points_have_consistent_normals_on_sphere() {
    let field = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let first = normal_at_default(&field, Vec3::Y).unwrap();
    let second = normal_at_default(&field, Vec3::new(0.01, 1.0, 0.0).normalize()).unwrap();
    assert!(first.dot(second) > 0.99);
}
