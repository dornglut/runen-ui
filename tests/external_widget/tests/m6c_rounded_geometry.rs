use runenui_core::{LogicalLength, LogicalPoint, LogicalRect, Radius, SceneShape};

fn length(value: f32) -> LogicalLength {
    LogicalLength::new(value).unwrap_or_else(|_| unreachable!("test radius is valid"))
}

fn rect(width: f32, height: f32) -> LogicalRect {
    LogicalRect::try_new(0.0, 0.0, width, height)
        .unwrap_or_else(|_| unreachable!("test rectangle is valid"))
}

fn point(x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
}

#[test]
fn rounded_shape_zero_equal_and_unequal_oversized_radii_share_one_normalization_rule() {
    let square = rect(10.0, 10.0);
    let zero = SceneShape::rounded_rect(square, Radius::ZERO);
    assert!(zero.contains(point(0.0, 0.0)));
    assert!(zero.contains(point(9.999, 9.999)));
    assert!(!zero.contains(point(10.0, 5.0)));

    let equal_oversized = SceneShape::rounded_rect(square, Radius::all(length(10.0)));
    assert!(equal_oversized.contains(point(0.0, 5.0)));
    assert!(!equal_oversized.contains(point(0.0, 0.0)));

    let unequal_oversized = SceneShape::rounded_rect(
        rect(10.0, 8.0),
        Radius::new(length(8.0), length(4.0), length(6.0), length(2.0)),
    );
    assert!(unequal_oversized.contains(point(6.5, 0.0)));
    assert!(!unequal_oversized.contains(point(6.0, 0.0)));
}
