//! File: domain/ui/ui_runtime/src/layout/engine/controls.rs
//! Purpose: Leaf and reusable control layout for retained UI nodes.

use ui_math::{UiRect, UiSize};

use crate::{
    ButtonNode, ComputedLayout, ComputedLayoutMap, LabelNode, NumericInputNode, SelectNode,
    TableNode, TabsNode, TextInputNode, ToggleNode, TreeNode, UiNode,
};

use super::measure::{table_size, tree_size};

pub(super) fn layout_label(
    node: &UiNode,
    label: &LabelNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let line_height = label
        .text_style
        .line_height_or_default(label.text_style.font_size * 1.2);

    let estimated_width =
        (label.text.chars().count() as f32 * label.text_style.font_size * 0.6).max(0.0);

    let constrained_size = label
        .constraints
        .constrain(UiSize::new(estimated_width, line_height));
    let size = UiSize::new(
        constrained_size.width.min(bounds.width.max(0.0)),
        constrained_size.height.min(bounds.height.max(0.0)),
    );

    let layout_bounds = UiRect::new(bounds.x, bounds.y, size.width, size.height);

    out.insert(
        node.id,
        ComputedLayout::new(layout_bounds, layout_bounds, size),
    );

    size
}
pub(super) fn layout_button(
    node: &UiNode,
    button: &ButtonNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let line_height = button
        .text_style
        .line_height_or_default(button.text_style.font_size * 1.2);

    let text_width =
        (button.label.chars().count() as f32 * button.text_style.font_size * 0.6).max(0.0);

    let measured_size = UiSize::new(
        (text_width + button.padding.horizontal()).max(button.min_size.width),
        (line_height + button.padding.vertical()).max(button.min_size.height),
    );

    let layout_width = if button.fill_width {
        bounds.width.max(0.0)
    } else {
        measured_size.width.min(bounds.width.max(0.0))
    };
    let layout_bounds = UiRect::new(
        bounds.x,
        bounds.y,
        layout_width,
        measured_size.height.min(bounds.height.max(0.0)),
    );

    let content_bounds = layout_bounds.inset(button.padding);

    out.insert(
        node.id,
        ComputedLayout::new(layout_bounds, content_bounds, measured_size),
    );

    measured_size
}
pub(super) fn layout_text_input(
    node: &UiNode,
    text_input: &TextInputNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let line_height = text_input
        .text_style
        .line_height_or_default(text_input.text_style.font_size * 1.2);
    let display_text = if text_input.value.is_empty() {
        &text_input.placeholder
    } else {
        &text_input.value
    };
    let text_width =
        (display_text.chars().count() as f32 * text_input.text_style.font_size * 0.6).max(0.0);
    let measured_size = UiSize::new(
        (text_width + text_input.padding.horizontal()).max(text_input.min_size.width),
        (line_height + text_input.padding.vertical()).max(text_input.min_size.height),
    );
    let layout_width = if text_input.fill_width {
        bounds.width.max(0.0)
    } else {
        measured_size.width.min(bounds.width.max(0.0))
    };
    let layout_bounds = UiRect::new(
        bounds.x,
        bounds.y,
        layout_width,
        measured_size.height.min(bounds.height.max(0.0)),
    );
    let content_bounds = layout_bounds.inset(text_input.padding);
    out.insert(
        node.id,
        ComputedLayout::new(layout_bounds, content_bounds, measured_size),
    );
    measured_size
}
pub(super) fn layout_toggle(
    node: &UiNode,
    toggle: &ToggleNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let line_height = toggle
        .text_style
        .line_height_or_default(toggle.text_style.font_size * 1.2);
    let label_width =
        (toggle.label.chars().count() as f32 * toggle.text_style.font_size * 0.6).max(0.0);
    let measured_size = UiSize::new(
        (label_width + toggle.padding.horizontal() + line_height).max(toggle.min_size.width),
        (line_height + toggle.padding.vertical()).max(toggle.min_size.height),
    );
    let layout_bounds = UiRect::new(
        bounds.x,
        bounds.y,
        measured_size.width.min(bounds.width.max(0.0)),
        measured_size.height.min(bounds.height.max(0.0)),
    );
    let content_bounds = layout_bounds.inset(toggle.padding);
    out.insert(
        node.id,
        ComputedLayout::new(layout_bounds, content_bounds, measured_size),
    );
    measured_size
}
pub(super) fn layout_numeric_input(
    node: &UiNode,
    numeric: &NumericInputNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let line_height = numeric
        .text_style
        .line_height_or_default(numeric.text_style.font_size * 1.2);
    let value_text = format!("{:.*}", usize::from(numeric.precision), numeric.value);
    let text_width =
        (value_text.chars().count() as f32 * numeric.text_style.font_size * 0.6).max(0.0);
    let measured_size = UiSize::new(
        (text_width + numeric.padding.horizontal() + line_height).max(numeric.min_size.width),
        (line_height + numeric.padding.vertical()).max(numeric.min_size.height),
    );
    let layout_bounds = UiRect::new(
        bounds.x,
        bounds.y,
        measured_size.width.min(bounds.width.max(0.0)),
        measured_size.height.min(bounds.height.max(0.0)),
    );
    let content_bounds = layout_bounds.inset(numeric.padding);
    out.insert(
        node.id,
        ComputedLayout::new(layout_bounds, content_bounds, measured_size),
    );
    measured_size
}
pub(super) fn layout_tabs(
    node: &UiNode,
    tabs: &TabsNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let line_height = tabs
        .text_style
        .line_height_or_default(tabs.text_style.font_size * 1.2);
    let width = tabs
        .labels
        .iter()
        .map(|label| {
            label.chars().count() as f32 * tabs.text_style.font_size * 0.6
                + tabs.padding.horizontal()
        })
        .sum::<f32>()
        .max(tabs.min_size.width);
    let measured_size = UiSize::new(
        width,
        (line_height + tabs.padding.vertical()).max(tabs.min_size.height),
    );
    let layout_bounds = UiRect::new(
        bounds.x,
        bounds.y,
        measured_size.width.min(bounds.width.max(0.0)),
        measured_size.height.min(bounds.height.max(0.0)),
    );
    let content_bounds = layout_bounds.inset(tabs.padding);
    out.insert(
        node.id,
        ComputedLayout::new(layout_bounds, content_bounds, measured_size),
    );
    measured_size
}
pub(super) fn layout_select(
    node: &UiNode,
    select: &SelectNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let line_height = select
        .text_style
        .line_height_or_default(select.text_style.font_size * 1.2);
    let display_text = select
        .selected_index
        .and_then(|index| select.options.get(index))
        .unwrap_or(&select.placeholder);
    let text_width =
        (display_text.chars().count() as f32 * select.text_style.font_size * 0.6).max(0.0);
    let measured_size = UiSize::new(
        (text_width + select.padding.horizontal() + line_height).max(select.min_size.width),
        (line_height + select.padding.vertical()).max(select.min_size.height),
    );
    let layout_bounds = UiRect::new(
        bounds.x,
        bounds.y,
        measured_size.width.min(bounds.width.max(0.0)),
        measured_size.height.min(bounds.height.max(0.0)),
    );
    let content_bounds = layout_bounds.inset(select.padding);
    out.insert(
        node.id,
        ComputedLayout::new(layout_bounds, content_bounds, measured_size),
    );
    measured_size
}
pub(super) fn layout_table(
    node: &UiNode,
    table: &TableNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let measured_size = table_size(table);
    let layout_bounds = UiRect::new(
        bounds.x,
        bounds.y,
        measured_size.width.min(bounds.width.max(0.0)),
        measured_size.height.min(bounds.height.max(0.0)),
    );
    out.insert(
        node.id,
        ComputedLayout::new(layout_bounds, layout_bounds, measured_size),
    );
    measured_size
}
pub(super) fn layout_tree(
    node: &UiNode,
    tree: &TreeNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let measured_size = tree_size(tree);
    let layout_bounds = UiRect::new(
        bounds.x,
        bounds.y,
        measured_size.width.min(bounds.width.max(0.0)),
        measured_size.height.min(bounds.height.max(0.0)),
    );
    out.insert(
        node.id,
        ComputedLayout::new(layout_bounds, layout_bounds, measured_size),
    );
    measured_size
}
