//! File: domain/ui/ui_runtime/src/input/pointer/helpers.rs
//! Purpose: Shared pointer dispatch helpers.

use ui_input::{FocusChange, InputResponse};

use crate::{
    UiInputDispatchResult, UiInputOutcome, UiInteraction, UiInteractionResults, UiInvalidation,
    UiNodeKind, UiTree, WidgetId,
};

pub(super) fn find_node(tree: &UiTree, widget_id: WidgetId) -> Option<&crate::UiNode> {
    tree.walk().find(|node| node.id == widget_id)
}

pub(super) fn is_pointer_responsive(tree: &UiTree, widget_id: WidgetId) -> bool {
    let Some(node) = find_node(tree, widget_id) else {
        return false;
    };

    match &node.kind {
        UiNodeKind::Button(button) => button.enabled,
        UiNodeKind::TextInput(text_input) => text_input.editable,
        UiNodeKind::Toggle(toggle) => toggle.enabled,
        UiNodeKind::NumericInput(numeric) => numeric.enabled,
        UiNodeKind::Select(select) => select.enabled,
        UiNodeKind::Table(table) => table.rows.iter().any(|row| row.enabled),
        UiNodeKind::Tree(tree) => tree.rows.iter().any(|row| row.enabled),
        UiNodeKind::GraphCanvas(graph_canvas) => {
            graph_canvas.capture_pointer_drag || graph_canvas.owns_wheel_zoom
        }
        UiNodeKind::Tabs(_) | UiNodeKind::ViewportSurfaceEmbed(_) | UiNodeKind::Scroll(_) => true,
        UiNodeKind::Panel(_)
        | UiNodeKind::Popup(_)
        | UiNodeKind::RadialMenu(_)
        | UiNodeKind::OverlayAdornment(_)
        | UiNodeKind::Label(_)
        | UiNodeKind::Spacer(_)
        | UiNodeKind::Divider(_)
        | UiNodeKind::Image(_)
        | UiNodeKind::ProductSurface(_)
        | UiNodeKind::Stack(_)
        | UiNodeKind::Split(_) => false,
    }
}

pub(super) fn table_row_index_at(
    table: &crate::TableNode,
    content_bounds: ui_math::UiRect,
    position: ui_math::UiPoint,
) -> Option<usize> {
    if position.y < content_bounds.y + table.row_height {
        return None;
    }
    let relative_y = position.y - content_bounds.y - table.row_height;
    let row_index = (relative_y / table.row_height).floor() as usize;
    (row_index < table.rows.len()).then_some(row_index)
}

pub(super) fn tree_row_index_at(
    tree: &crate::TreeNode,
    content_bounds: ui_math::UiRect,
    position: ui_math::UiPoint,
) -> Option<usize> {
    let relative_y = position.y - content_bounds.y;
    if relative_y < 0.0 {
        return None;
    }
    let row_index = (relative_y / tree.row_height).floor() as usize;
    (row_index < tree.rows.len()).then_some(row_index)
}

pub(super) fn push_pressed_change_if_needed(
    interactions: &mut UiInteractionResults,
    previous: Option<WidgetId>,
    current: Option<WidgetId>,
) {
    if previous != current {
        interactions.push(UiInteraction::PressedChanged { previous, current });
    }
}

pub(super) fn push_focus_change_if_needed(
    interactions: &mut UiInteractionResults,
    focus_change: FocusChange,
) {
    if !matches!(focus_change, FocusChange::None) {
        interactions.push(UiInteraction::FocusChanged(focus_change));
    }
}

fn response(target: Option<WidgetId>, response: InputResponse) -> UiInputDispatchResult {
    UiInputDispatchResult { target, response }
}

pub(super) fn outcome(
    target: Option<WidgetId>,
    input_response: InputResponse,
    interactions: UiInteractionResults,
) -> UiInputOutcome {
    UiInputOutcome {
        dispatch: response(target, input_response),
        interactions,
        invalidation: UiInvalidation::from_response(input_response),
    }
}
