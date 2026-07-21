use glam::Vec3;

use runen_sdf::ops::{Intersect, SmoothIntersect, SmoothSubtract, SmoothUnion, Subtract, Union};
use runen_sdf::primitives::{SdfPlane, SdfSphere};
use runen_sdf::{Bounds3, FieldBounds, SdfField3};

const EPS: f32 = 1e-4;

#[test]
fn hard_boolean_ops_preserve_sign_and_conservative_steps() {
    let left = SdfSphere::new(Vec3::ZERO, 2.0).unwrap();
    let right = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let point = Vec3::new(1.5, 0.0, 0.0);

    let union = Union::new(left, right).sample(point).unwrap();
    let intersect = Intersect::new(left, right).sample(point).unwrap();
    let subtract = Subtract::new(left, right).sample(point).unwrap();

    assert!(union.signed_value() < 0.0);
    assert!(intersect.signed_value() > 0.0);
    assert!(subtract.signed_value() < 0.0);
    assert!(union.safe_step().is_some());
    assert!(intersect.safe_step().is_some());
    assert!(subtract.safe_step().is_some());
    assert!(!Union::new(left, right).capabilities().has_exact_distance());
}

#[test]
fn field_bounds_distinguish_empty_unbounded_and_bounded() {
    let a = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let b = SdfSphere::new(Vec3::new(4.0, 0.0, 0.0), 1.0).unwrap();

    assert_eq!(Intersect::new(a, b).bounds(), FieldBounds::Empty);

    let union = Union::new(a, b);
    let FieldBounds::Bounded(bounds) = union.bounds() else {
        panic!("union should be bounded");
    };
    assert_eq!(bounds.min(), Vec3::new(-1.0, -1.0, -1.0));
    assert_eq!(bounds.max(), Vec3::new(5.0, 1.0, 1.0));

    let plane = SdfPlane::from_point_normal(Vec3::ZERO, Vec3::Y).unwrap();
    assert_eq!(Union::new(a, plane).bounds(), FieldBounds::Unbounded);

    let left = FieldBounds::bounded(Bounds3::try_new(Vec3::ZERO, Vec3::ONE).unwrap());
    let right = FieldBounds::bounded(Bounds3::try_new(Vec3::splat(2.0), Vec3::splat(3.0)).unwrap());
    assert_eq!(left.intersection(right), FieldBounds::Empty);
}

#[test]
fn zero_smoothness_matches_hard_ops_and_preserves_step() {
    let left = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let right = SdfSphere::new(Vec3::new(1.0, 0.0, 0.0), 1.0).unwrap();
    let point = Vec3::new(0.25, 0.0, 0.0);

    let hard_union = Union::new(left, right).sample(point).unwrap();
    let smooth_union = SmoothUnion::new(left, right, 0.0)
        .unwrap()
        .sample(point)
        .unwrap();
    assert!((hard_union.signed_value() - smooth_union.signed_value()).abs() < EPS);
    assert_eq!(hard_union.safe_step(), smooth_union.safe_step());

    let hard_intersect = Intersect::new(left, right).sample(point).unwrap();
    let smooth_intersect = SmoothIntersect::new(left, right, 0.0)
        .unwrap()
        .sample(point)
        .unwrap();
    assert!((hard_intersect.signed_value() - smooth_intersect.signed_value()).abs() < EPS);

    let hard_subtract = Subtract::new(left, right).sample(point).unwrap();
    let smooth_subtract = SmoothSubtract::new(left, right, 0.0)
        .unwrap()
        .sample(point)
        .unwrap();
    assert!((hard_subtract.signed_value() - smooth_subtract.signed_value()).abs() < EPS);
}

#[test]
fn positive_smoothing_removes_unproven_tracing_capability() {
    let left = SdfSphere::new(Vec3::new(-0.5, 0.0, 0.0), 1.0).unwrap();
    let right = SdfSphere::new(Vec3::new(0.5, 0.0, 0.0), 1.0).unwrap();
    let smooth = SmoothUnion::new(left, right, 0.5).unwrap();
    let sample = smooth.sample(Vec3::ZERO).unwrap();

    assert!(sample.signed_value() < 0.0);
    assert_eq!(sample.safe_step(), None);
    assert!(!smooth.capabilities().has_exact_distance());
}
