use glam::{Affine3A, Quat, Vec3};

use runen_sdf::primitives::{SdfBox3, SdfSphere};
use runen_sdf::transform::{Affine, Rotate, Scale, Translate};
use runen_sdf::{FieldBounds, SdfField3};

const EPS: f32 = 1e-4;

#[test]
fn translation_and_rotation_preserve_exact_distance() {
    let translated = Translate::new(
        SdfSphere::new(Vec3::ZERO, 1.0).unwrap(),
        Vec3::new(2.0, 0.0, 0.0),
    )
    .unwrap();
    assert!(
        (translated
            .sample(Vec3::new(2.0, 0.0, 0.0))
            .unwrap()
            .signed_value()
            + 1.0)
            .abs()
            < EPS
    );
    assert!(translated.capabilities().has_exact_distance());

    let box3 = SdfBox3::new(Vec3::new(2.0, 0.0, 0.0), Vec3::splat(0.5)).unwrap();
    let rotated = Rotate::new(box3, Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)).unwrap();
    assert!(
        rotated
            .sample(Vec3::new(0.0, 2.0, 0.0))
            .unwrap()
            .signed_value()
            < 0.0
    );
    assert!(rotated.capabilities().has_exact_distance());
}

#[test]
fn uniform_scale_scales_value_step_and_bounds() {
    let scaled = Scale::new(SdfSphere::new(Vec3::ZERO, 1.0).unwrap(), 2.0).unwrap();
    let center = scaled.sample(Vec3::ZERO).unwrap();
    assert!((center.signed_value() + 2.0).abs() < EPS);
    assert!((center.safe_step().unwrap() - 2.0).abs() < EPS);

    let FieldBounds::Bounded(bounds) = scaled.bounds() else {
        panic!("scaled sphere should be bounded");
    };
    assert_eq!(bounds.min(), Vec3::splat(-2.0));
    assert_eq!(bounds.max(), Vec3::splat(2.0));
    assert!(scaled.capabilities().has_exact_distance());
}

#[test]
fn affine_transform_uses_conservative_step_and_downgrades_exactness() {
    let transform = Affine3A::from_scale_rotation_translation(
        Vec3::new(2.0, 1.0, 0.5),
        Quat::IDENTITY,
        Vec3::new(1.0, 2.0, 3.0),
    );
    let affine = Affine::new(SdfSphere::new(Vec3::ZERO, 1.0).unwrap(), transform).unwrap();
    let sample = affine.sample(Vec3::new(1.0, 2.0, 3.0)).unwrap();

    assert!(sample.signed_value() < 0.0);
    assert!(sample.safe_step().is_some());
    assert!(!affine.capabilities().has_exact_distance());

    let FieldBounds::Bounded(bounds) = affine.bounds() else {
        panic!("affine sphere should remain bounded");
    };
    assert!(bounds.contains_point(Vec3::new(1.0, 2.0, 3.0)));
}

#[test]
fn invalid_transforms_are_rejected() {
    let sphere = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    assert!(Scale::new(sphere, 0.0).is_err());
    assert!(Rotate::new(sphere, Quat::from_xyzw(0.0, 0.0, 0.0, 0.0)).is_err());
    assert!(Affine::new(sphere, Affine3A::from_scale(Vec3::ZERO)).is_err());
    assert!(Translate::new(sphere, Vec3::splat(f32::NAN)).is_err());
}
