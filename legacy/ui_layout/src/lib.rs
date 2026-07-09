//! File: domain/ui/ui_layout/src/lib.rs
//! Crate: ui_layout

pub mod alignment;
pub mod arrange;
pub mod constraints;
pub mod contracts;
pub mod measure;
pub mod size_policy;
pub mod split;
pub mod stack;

pub use alignment::*;
pub use arrange::*;
pub use constraints::*;
pub use contracts::*;
pub use measure::*;
pub use size_policy::*;
pub use split::*;
pub use stack::*;

#[cfg(test)]
mod tests {
    use super::*;
    use ui_math::{UiRect, UiSize};

    #[test]
    fn horizontal_stack_arrange_respects_gap_and_flex_distribution() {
        let layout = StackLayout::horizontal(8.0);
        let items = vec![
            StackItem::fixed(UiSize::new(40.0, 20.0), 40.0),
            StackItem::flex(UiSize::new(10.0, 20.0), 1.0),
            StackItem::flex(UiSize::new(10.0, 20.0), 1.0),
        ];
        let arranged = layout.arrange(UiRect::new(0.0, 0.0, 160.0, 24.0), &items);
        assert_eq!(arranged.len(), 3);
        assert!((arranged[0].width - 40.0).abs() < 0.001);
        assert!((arranged[1].x - 48.0).abs() < 0.001);
        assert!((arranged[2].x - (arranged[1].x + arranged[1].width + 8.0)).abs() < 0.001);
    }
}
