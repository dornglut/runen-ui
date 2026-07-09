//! File: domain/ui/ui_math/src/lib.rs
//! Crate: ui_math

pub mod axis;
pub mod insets;
pub mod point;
pub mod rect;
pub mod size;
pub mod vector;

pub use axis::{Axis, AxisDirection};
pub use insets::UiInsets;
pub use point::UiPoint;
pub use rect::UiRect;
pub use size::UiSize;
pub use vector::UiVector;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_inset_and_intersection_behave_consistently() {
        let rect = UiRect::new(10.0, 20.0, 100.0, 60.0);
        let inset = rect.inset(UiInsets::all(10.0));
        assert_eq!(inset, UiRect::new(20.0, 30.0, 80.0, 40.0));

        let other = UiRect::new(50.0, 10.0, 80.0, 50.0);
        assert_eq!(
            rect.intersect(other),
            Some(UiRect::new(50.0, 20.0, 60.0, 40.0))
        );
    }
}
