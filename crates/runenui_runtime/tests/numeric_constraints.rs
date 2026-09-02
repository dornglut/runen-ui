use runenui_core::{LogicalLength, LogicalLengthError};
use runenui_runtime::{AxisConstraints, AxisLimit};

fn length(value: f32) -> LogicalLength {
    LogicalLength::new(value).unwrap_or_default()
}

#[test]
fn finite_nan_infinity_negative_and_boundaries_are_explicit() {
    assert_eq!(LogicalLength::new(12.0), Ok(length(12.0)));
    assert_eq!(
        LogicalLength::new(f32::NAN),
        Err(LogicalLengthError::NotFinite)
    );
    assert_eq!(
        LogicalLength::new(f32::INFINITY),
        Err(LogicalLengthError::NotFinite)
    );
    assert_eq!(
        LogicalLength::new(f32::NEG_INFINITY),
        Err(LogicalLengthError::NotFinite)
    );
    assert_eq!(LogicalLength::new(-1.0), Err(LogicalLengthError::Negative));
    assert_eq!(LogicalLength::new(f32::MAX), Ok(LogicalLength::MAX));
}

#[test]
fn inverted_constraints_normalize_and_arithmetic_saturates() {
    let constraints = AxisConstraints::new(length(20.0), AxisLimit::finite(length(10.0)));
    assert!(constraints.is_tight());
    assert_eq!(constraints.min(), length(20.0));
    assert_eq!(constraints.max().as_finite(), Some(length(20.0)));
    assert_eq!(constraints.constrain(length(5.0)), length(20.0));
    assert_eq!(
        LogicalLength::MAX.saturating_add(LogicalLength::MAX),
        LogicalLength::MAX
    );
}
