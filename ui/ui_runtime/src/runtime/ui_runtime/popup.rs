//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/popup.rs
//! Purpose: Runtime popup stack helpers.

use crate::{ComputedLayoutMap, PopupDismissPolicy, UiNodeKind, UiTree, WidgetId};

pub(super) fn topmost_popup_scope(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
) -> Option<(WidgetId, WidgetId)> {
    tree.walk()
        .enumerate()
        .filter_map(|(tree_order, node)| {
            let UiNodeKind::Popup(popup) = &node.kind else {
                return None;
            };
            if !matches!(popup.dismiss_policy, PopupDismissPolicy::OutsidePointerDown) {
                return None;
            }
            layouts.contains_key(&node.id).then_some((
                popup.layer_order,
                tree_order,
                node.id,
                popup.anchor,
            ))
        })
        .max_by_key(|(layer_order, tree_order, _, _)| (*layer_order, *tree_order))
        .map(|(_, _, popup, anchor)| (popup, anchor))
}
