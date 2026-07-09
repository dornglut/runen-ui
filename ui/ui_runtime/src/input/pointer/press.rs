//! File: domain/ui/ui_runtime/src/input/pointer/press.rs
//! Purpose: Pointer release activation mapping.

use crate::{ComputedLayoutMap, UiInteraction, UiNodeKind, UiTree, WidgetId};

use super::helpers::{find_node, table_row_index_at, tree_row_index_at};

pub(super) fn activation_for_release(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    pressed_target: Option<WidgetId>,
    release_target: Option<WidgetId>,
    release_position: ui_math::UiPoint,
) -> Option<UiInteraction> {
    let widget_id = pressed_target?;

    if Some(widget_id) != release_target {
        return None;
    }

    let node = find_node(tree, widget_id)?;

    match &node.kind {
        UiNodeKind::Button(button) => button
            .enabled
            .then_some(UiInteraction::Activated(widget_id)),
        UiNodeKind::Toggle(toggle) => toggle.enabled.then_some(UiInteraction::Toggled {
            target: widget_id,
            checked: !toggle.checked,
        }),
        UiNodeKind::Tabs(tabs) => {
            let layout = layouts.get(&widget_id)?;
            if tabs.labels.is_empty() || layout.bounds.width <= f32::EPSILON {
                return None;
            }
            let relative_x = (release_position.x - layout.bounds.x).clamp(0.0, layout.bounds.width);
            let segment =
                ((relative_x / layout.bounds.width) * tabs.labels.len() as f32).floor() as usize;
            let index = segment.min(tabs.labels.len() - 1);
            Some(UiInteraction::TabSelected {
                target: widget_id,
                index,
            })
        }
        UiNodeKind::Select(select) => {
            if !select.enabled || select.options.is_empty() {
                return None;
            }
            let next_index = select
                .selected_index
                .map(|index| (index + 1) % select.options.len())
                .unwrap_or(0);
            Some(UiInteraction::SelectChanged {
                target: widget_id,
                index: next_index,
            })
        }
        UiNodeKind::Table(table) => {
            let layout = layouts.get(&widget_id)?;
            let row_index = table_row_index_at(table, layout.content_bounds, release_position)?;
            table.rows.get(row_index).and_then(|row| {
                row.enabled.then_some(UiInteraction::TableRowSelected {
                    target: widget_id,
                    row_index,
                })
            })
        }
        UiNodeKind::Tree(tree) => {
            let layout = layouts.get(&widget_id)?;
            let row_index = tree_row_index_at(tree, layout.content_bounds, release_position)?;
            let row = tree.rows.get(row_index)?;
            if !row.enabled {
                return None;
            }
            let relative_x = release_position.x - layout.content_bounds.x;
            let toggle_end = row.depth as f32 * tree.indent_width + tree.indent_width;
            if row.has_children && relative_x <= toggle_end {
                return Some(UiInteraction::TreeRowToggled {
                    target: widget_id,
                    row_index,
                    expanded: !row.expanded,
                });
            }
            Some(UiInteraction::TreeRowSelected {
                target: widget_id,
                row_index,
            })
        }
        UiNodeKind::Panel(_)
        | UiNodeKind::Popup(_)
        | UiNodeKind::RadialMenu(_)
        | UiNodeKind::OverlayAdornment(_)
        | UiNodeKind::Label(_)
        | UiNodeKind::TextInput(_)
        | UiNodeKind::NumericInput(_)
        | UiNodeKind::GraphCanvas(_)
        | UiNodeKind::Spacer(_)
        | UiNodeKind::Divider(_)
        | UiNodeKind::Image(_)
        | UiNodeKind::ProductSurface(_)
        | UiNodeKind::ViewportSurfaceEmbed(_)
        | UiNodeKind::Scroll(_)
        | UiNodeKind::Stack(_)
        | UiNodeKind::Split(_) => None,
    }
}
