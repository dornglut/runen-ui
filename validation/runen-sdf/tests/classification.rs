use glam::Vec3;

use runen_sdf::primitives::SdfSphere;
use runen_sdf::queries::QueryError;
use runen_sdf::queries::classify::{PointClassification, classify_point};

#[test]
fn classify_inside_outside_and_surface() {
    let field = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();

    assert_eq!(
        classify_point(&field, Vec3::ZERO, 1e-4).unwrap(),
        PointClassification::Inside
    );
    assert_eq!(
        classify_point(&field, Vec3::X, 1e-4).unwrap(),
        PointClassification::OnSurface
    );
    assert_eq!(
        classify_point(&field, Vec3::new(2.0, 0.0, 0.0), 1e-4).unwrap(),
        PointClassification::Outside
    );
}

#[test]
fn classification_is_epsilon_aware() {
    let field = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    let point = Vec3::new(1.0005, 0.0, 0.0);

    assert_eq!(
        classify_point(&field, point, 1e-3).unwrap(),
        PointClassification::OnSurface
    );
    assert_eq!(
        classify_point(&field, point, 1e-5).unwrap(),
        PointClassification::Outside
    );
}

#[test]
fn classification_rejects_invalid_admission() {
    let field = SdfSphere::new(Vec3::ZERO, 1.0).unwrap();
    assert!(matches!(
        classify_point(&field, Vec3::ZERO, 0.0),
        Err(QueryError::Validation(_))
    ));
    assert!(matches!(
        classify_point(&field, Vec3::splat(f32::NAN), 1e-4),
        Err(QueryError::Validation(_))
    ));
}
