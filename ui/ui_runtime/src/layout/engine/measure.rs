//! File: domain/ui/ui_runtime/src/layout/engine/measure.rs
//! Purpose: Shared retained UI measurement helpers for layout dispatchers.

use ui_layout::{
    CrossAxisAlignment, LayoutConstraints, MainAxisAlignment, SizePolicy, StackItem, StackLayout,
};
use ui_math::{Axis, UiRect, UiSize};

use crate::{DividerNode, PopupNode, SplitNode, TableNode, TreeNode, UiNode, UiNodeKind};

pub(super) fn measure_node(node: &UiNode) -> UiSize {
    match &node.kind {
        UiNodeKind::Panel(panel) => {
            let mut child_size = UiSize::ZERO;
            for child in node.children.iter().filter(|child| !is_popup_node(child)) {
                let measured = measure_node(child);
                child_size.width = child_size.width.max(measured.width);
                child_size.height = child_size.height.max(measured.height);
            }

            UiSize::new(
                (child_size.width + panel.padding.horizontal()).max(panel.min_size.width),
                (child_size.height + panel.padding.vertical()).max(panel.min_size.height),
            )
        }
        UiNodeKind::Popup(_) | UiNodeKind::RadialMenu(_) | UiNodeKind::OverlayAdornment(_) => {
            UiSize::ZERO
        }
        UiNodeKind::Label(label) => {
            let line_height = label
                .text_style
                .line_height_or_default(label.text_style.font_size * 1.2);

            let estimated_width =
                (label.text.chars().count() as f32 * label.text_style.font_size * 0.6).max(0.0);

            label
                .constraints
                .constrain(UiSize::new(estimated_width, line_height))
        }
        UiNodeKind::Button(button) => {
            let line_height = button
                .text_style
                .line_height_or_default(button.text_style.font_size * 1.2);

            let text_width =
                (button.label.chars().count() as f32 * button.text_style.font_size * 0.6).max(0.0);

            UiSize::new(
                (text_width + button.padding.horizontal()).max(button.min_size.width),
                (line_height + button.padding.vertical()).max(button.min_size.height),
            )
        }
        UiNodeKind::TextInput(text_input) => {
            let line_height = text_input
                .text_style
                .line_height_or_default(text_input.text_style.font_size * 1.2);
            let display_text = if text_input.value.is_empty() {
                &text_input.placeholder
            } else {
                &text_input.value
            };
            let text_width =
                (display_text.chars().count() as f32 * text_input.text_style.font_size * 0.6)
                    .max(0.0);
            UiSize::new(
                (text_width + text_input.padding.horizontal()).max(text_input.min_size.width),
                (line_height + text_input.padding.vertical()).max(text_input.min_size.height),
            )
        }
        UiNodeKind::Toggle(toggle) => {
            let line_height = toggle
                .text_style
                .line_height_or_default(toggle.text_style.font_size * 1.2);
            let label_width =
                (toggle.label.chars().count() as f32 * toggle.text_style.font_size * 0.6).max(0.0);
            UiSize::new(
                (label_width + toggle.padding.horizontal() + line_height)
                    .max(toggle.min_size.width),
                (line_height + toggle.padding.vertical()).max(toggle.min_size.height),
            )
        }
        UiNodeKind::NumericInput(numeric) => {
            let line_height = numeric
                .text_style
                .line_height_or_default(numeric.text_style.font_size * 1.2);
            let value_text = format!("{:.*}", usize::from(numeric.precision), numeric.value);
            let text_width =
                (value_text.chars().count() as f32 * numeric.text_style.font_size * 0.6).max(0.0);
            UiSize::new(
                (text_width + numeric.padding.horizontal() + line_height)
                    .max(numeric.min_size.width),
                (line_height + numeric.padding.vertical()).max(numeric.min_size.height),
            )
        }
        UiNodeKind::Tabs(tabs) => {
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
            UiSize::new(
                width,
                (line_height + tabs.padding.vertical()).max(tabs.min_size.height),
            )
        }
        UiNodeKind::Select(select) => {
            let line_height = select
                .text_style
                .line_height_or_default(select.text_style.font_size * 1.2);
            let display_text = select
                .selected_index
                .and_then(|index| select.options.get(index))
                .unwrap_or(&select.placeholder);
            let text_width =
                (display_text.chars().count() as f32 * select.text_style.font_size * 0.6).max(0.0);
            UiSize::new(
                (text_width + select.padding.horizontal() + line_height).max(select.min_size.width),
                (line_height + select.padding.vertical()).max(select.min_size.height),
            )
        }
        UiNodeKind::Table(table) => table_size(table),
        UiNodeKind::Tree(tree) => tree_size(tree),
        UiNodeKind::Spacer(spacer) => spacer.min_size,
        UiNodeKind::Divider(divider) => divider_intrinsic_size(divider),
        UiNodeKind::Image(image) => image.min_size,
        UiNodeKind::ProductSurface(surface) => surface.min_size,
        UiNodeKind::GraphCanvas(graph_canvas) => graph_canvas.min_size,
        UiNodeKind::ViewportSurfaceEmbed(embed) => embed.min_size,
        UiNodeKind::Scroll(_) => node
            .children
            .first()
            .map(measure_node)
            .unwrap_or(UiSize::ZERO),
        UiNodeKind::Stack(stack) => {
            let mut items = Vec::with_capacity(node.children.len());
            for (index, child) in node
                .children
                .iter()
                .filter(|child| !is_popup_node(child))
                .enumerate()
            {
                let size = measure_node(child);
                let policy = stack
                    .child_main_policies
                    .get(index)
                    .copied()
                    .unwrap_or(SizePolicy::Auto);

                items.push(StackItem {
                    size,
                    main_policy: policy,
                });
            }

            let layout = match stack.axis {
                Axis::Vertical => StackLayout::vertical(stack.gap),
                Axis::Horizontal => StackLayout::horizontal(stack.gap),
            }
            .with_main_align(MainAxisAlignment::Start)
            .with_cross_align(CrossAxisAlignment::Stretch);

            let measured = layout.measure(
                &items,
                LayoutConstraints::loose(UiSize::new(f32::MAX, f32::MAX)),
            );

            UiSize::new(
                measured.width + stack.padding.horizontal(),
                measured.height + stack.padding.vertical(),
            )
        }
        UiNodeKind::Split(split) => measure_split(node, split),
    }
}
pub(super) fn measure_popup_content(node: &UiNode, popup: &PopupNode) -> UiSize {
    let mut width = 0.0_f32;
    let mut height = 0.0_f32;
    let mut item_count = 0_usize;
    for child in node.children.iter().filter(|child| !is_popup_node(child)) {
        let measured = measure_node(child);
        width = width.max(measured.width);
        height += measured.height;
        item_count += 1;
    }
    if item_count > 1 {
        height += popup.gap * (item_count - 1) as f32;
    }
    UiSize::new(width, height)
}
pub(super) fn measure_overlay_adornment_content(node: &UiNode) -> UiSize {
    let mut width = 0.0_f32;
    let mut height = 0.0_f32;
    for child in node.children.iter().filter(|child| !is_popup_node(child)) {
        let measured = measure_node(child);
        width = width.max(measured.width);
        height += measured.height;
    }
    UiSize::new(width, height)
}
pub(super) fn is_popup_node(node: &UiNode) -> bool {
    matches!(
        node.kind,
        UiNodeKind::Popup(_) | UiNodeKind::RadialMenu(_) | UiNodeKind::OverlayAdornment(_)
    )
}
pub(super) fn table_size(table: &TableNode) -> UiSize {
    let width = table
        .columns
        .iter()
        .map(|column| column.min_width)
        .sum::<f32>()
        .max(table.min_size.width);
    let height = (table.rows.len() as f32 + 1.0) * table.row_height;
    UiSize::new(width, height.max(table.min_size.height))
}
pub(super) fn tree_size(tree: &TreeNode) -> UiSize {
    let height = tree.rows.len().max(1) as f32 * tree.row_height;
    UiSize::new(tree.min_size.width, height.max(tree.min_size.height))
}
pub(super) fn divider_size_for_bounds(divider: &DividerNode, bounds: UiRect) -> UiSize {
    let length = match divider.length_policy {
        SizePolicy::Auto | SizePolicy::Flex(_) => match divider.axis {
            Axis::Horizontal => bounds.width.max(0.0),
            Axis::Vertical => bounds.height.max(0.0),
        },
        SizePolicy::Fixed(value) => value.max(0.0),
    };
    let thickness = divider.thickness.max(0.0);

    match divider.axis {
        Axis::Horizontal => UiSize::new(length, thickness),
        Axis::Vertical => UiSize::new(thickness, length),
    }
}
pub(super) fn divider_intrinsic_size(divider: &DividerNode) -> UiSize {
    let length = match divider.length_policy {
        SizePolicy::Fixed(value) => value.max(0.0),
        SizePolicy::Auto | SizePolicy::Flex(_) => 0.0,
    };
    let thickness = divider.thickness.max(0.0);

    match divider.axis {
        Axis::Horizontal => UiSize::new(length, thickness),
        Axis::Vertical => UiSize::new(thickness, length),
    }
}
pub(super) fn measure_split(node: &UiNode, split: &SplitNode) -> UiSize {
    match node.children.as_slice() {
        [left, right] => {
            let left_size = measure_node(left);
            let right_size = measure_node(right);
            match split.axis {
                Axis::Horizontal => UiSize::new(
                    left_size.width + split.gap.max(0.0) + right_size.width,
                    left_size.height.max(right_size.height),
                ),
                Axis::Vertical => UiSize::new(
                    left_size.width.max(right_size.width),
                    left_size.height + split.gap.max(0.0) + right_size.height,
                ),
            }
        }
        [left, right, _] => {
            let left_size = measure_node(left);
            let right_size = measure_node(right);
            match split.axis {
                Axis::Horizontal => UiSize::new(
                    left_size.width + split.gap.max(0.0) + right_size.width,
                    left_size.height.max(right_size.height),
                ),
                Axis::Vertical => UiSize::new(
                    left_size.width.max(right_size.width),
                    left_size.height + split.gap.max(0.0) + right_size.height,
                ),
            }
        }
        [only] => measure_node(only),
        _ => UiSize::ZERO,
    }
}
