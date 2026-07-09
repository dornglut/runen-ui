//! File: domain/ui/ui_evaluator/src/lib.rs
//! Crate: ui_evaluator

mod context;
mod evaluator;
mod output;
mod passes;
mod state_binding;

pub use context::UiEvaluationContext;
pub use evaluator::UiEvaluator;
pub use output::UiOutput;
pub use passes::{
    AccessibilityEvaluationPass, BindingEvaluationPass, ControlEvaluationPass, InputEvaluationPass,
    InspectionEvaluationPass, InteractionEvaluationPass, LayoutEvaluationPass, StateEvaluationPass,
    StateEvaluationRow, StyleEvaluationPass, VisualEvaluationPass,
};

#[cfg(test)]
mod tests;
