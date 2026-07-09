//! File: domain/ui/ui_runtime/src/input/pointer/numeric.rs
//! Purpose: Numeric input pointer-wheel stepping.

use crate::{UiNodeKind, UiTree, WidgetId};

use super::helpers::find_node;

pub(super) fn stepped_numeric_value(
    tree: &UiTree,
    widget_id: WidgetId,
    delta_y: f32,
) -> Option<f64> {
    let node = find_node(tree, widget_id)?;
    let UiNodeKind::NumericInput(numeric) = &node.kind else {
        return None;
    };
    if delta_y.abs() <= f32::EPSILON || !numeric.enabled {
        return None;
    }
    let direction = if delta_y < 0.0 { 1.0 } else { -1.0 };
    let mut value = numeric.value + direction * numeric.step;
    if let Some(min) = numeric.min {
        value = value.max(min);
    }
    if let Some(max) = numeric.max {
        value = value.min(max);
    }
    Some(value)
}
