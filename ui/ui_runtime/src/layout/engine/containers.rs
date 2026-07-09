//! File: domain/ui/ui_runtime/src/layout/engine/containers.rs
//! Purpose: Container, spacer, divider, stack, and split layout for retained UI nodes.

use ui_layout::{
    CrossAxisAlignment, LayoutConstraints, MainAxisAlignment, SizePolicy, SplitLayout, StackItem,
    StackLayout,
};
use ui_math::{Axis, UiRect, UiSize};

use crate::{
    ComputedLayout, ComputedLayoutMap, DividerNode, PanelNode, SpacerNode, SplitNode, StackNode,
    UiNode, UiRuntimeState,
};

use super::dispatch::layout_node;
use super::measure::{divider_size_for_bounds, is_popup_node, measure_node};

pub(super) fn layout_panel(
    node: &UiNode,
    panel: &PanelNode,
    bounds: UiRect,
    state: &UiRuntimeState,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let content_bounds = bounds.inset(panel.padding);

    let normal_children = node
        .children
        .iter()
        .filter(|child| !is_popup_node(child))
        .collect::<Vec<_>>();

    let content_size = if normal_children.is_empty() {
        UiSize::ZERO
    } else if normal_children.len() == 1 {
        layout_node(normal_children[0], content_bounds, state, out)
    } else {
        let child_items = normal_children
            .iter()
            .map(|child| StackItem::auto(measure_node(child)))
            .collect::<Vec<_>>();

        let layout = StackLayout::vertical(panel.gap)
            .with_main_align(MainAxisAlignment::Start)
            .with_cross_align(CrossAxisAlignment::Stretch);

        let arranged = layout.arrange(content_bounds, &child_items);
        for (child, child_bounds) in normal_children.iter().zip(arranged) {
            layout_node(child, child_bounds, state, out);
        }

        layout.measure(
            &child_items,
            LayoutConstraints::loose(content_bounds.size()),
        )
    };

    let measured_size = UiSize::new(
        (content_size.width + panel.padding.horizontal()).max(panel.min_size.width),
        (content_size.height + panel.padding.vertical()).max(panel.min_size.height),
    );

    out.insert(
        node.id,
        ComputedLayout::new(bounds, content_bounds, measured_size),
    );

    for popup in node.children.iter().filter(|child| is_popup_node(child)) {
        layout_node(popup, content_bounds, state, out);
    }

    measured_size
}
pub(super) fn layout_stack(
    node: &UiNode,
    stack: &StackNode,
    bounds: UiRect,
    state: &UiRuntimeState,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let content_bounds = bounds.inset(stack.padding);
    let normal_children = node
        .children
        .iter()
        .filter(|child| !is_popup_node(child))
        .collect::<Vec<_>>();

    let mut child_items = Vec::with_capacity(normal_children.len());
    for (index, child) in normal_children.iter().enumerate() {
        let measured = measure_node(child);
        let policy = stack
            .child_main_policies
            .get(index)
            .copied()
            .unwrap_or(SizePolicy::Auto);

        child_items.push(StackItem {
            size: measured,
            main_policy: policy,
        });
    }

    let layout = match stack.axis {
        Axis::Vertical => StackLayout::vertical(stack.gap),
        Axis::Horizontal => StackLayout::horizontal(stack.gap),
    }
    .with_main_align(MainAxisAlignment::Start)
    .with_cross_align(CrossAxisAlignment::Stretch);

    let arranged = layout.arrange(content_bounds, &child_items);

    for (child, child_bounds) in normal_children.iter().zip(arranged) {
        layout_node(child, child_bounds, state, out);
    }

    let measured_content = layout.measure(
        &child_items,
        LayoutConstraints::loose(content_bounds.size()),
    );

    let measured_size = UiSize::new(
        measured_content.width + stack.padding.horizontal(),
        measured_content.height + stack.padding.vertical(),
    );

    out.insert(
        node.id,
        ComputedLayout::new(bounds, content_bounds, measured_size),
    );

    for popup in node.children.iter().filter(|child| is_popup_node(child)) {
        layout_node(popup, content_bounds, state, out);
    }

    measured_size
}
pub(super) fn layout_spacer(
    node: &UiNode,
    spacer: &SpacerNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let measured_size = UiSize::new(
        bounds.width.max(spacer.min_size.width),
        bounds.height.max(spacer.min_size.height),
    );
    out.insert(node.id, ComputedLayout::new(bounds, bounds, measured_size));
    measured_size
}
pub(super) fn layout_divider(
    node: &UiNode,
    divider: &DividerNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let measured_size = divider_size_for_bounds(divider, bounds);
    let layout_bounds = match divider.axis {
        Axis::Horizontal => UiRect::new(
            bounds.x,
            bounds.y + (bounds.height - measured_size.height).max(0.0) * 0.5,
            measured_size.width,
            measured_size.height,
        ),
        Axis::Vertical => UiRect::new(
            bounds.x + (bounds.width - measured_size.width).max(0.0) * 0.5,
            bounds.y + (bounds.height - measured_size.height).max(0.0) * 0.5,
            measured_size.width,
            measured_size.height,
        ),
    };
    out.insert(
        node.id,
        ComputedLayout::new(layout_bounds, layout_bounds, measured_size),
    );
    measured_size
}
pub(super) fn layout_split(
    node: &UiNode,
    split: &SplitNode,
    bounds: UiRect,
    state: &UiRuntimeState,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let layout = SplitLayout::new(split.axis, split.ratio, split.gap);

    let measured_size = bounds.size();

    match node.children.as_slice() {
        [left, right] => {
            let (left_bounds, right_bounds) = layout.arrange(bounds);
            layout_node(left, left_bounds, state, out);
            layout_node(right, right_bounds, state, out);
        }
        [left, right, handle] => {
            let (left_bounds, right_bounds) = layout.arrange(bounds);
            layout_node(left, left_bounds, state, out);
            layout_node(right, right_bounds, state, out);
            let handle_size = measure_node(handle);
            let handle_bounds = arrange_split_handle_bounds(
                split.axis,
                bounds,
                left_bounds,
                right_bounds,
                handle_size,
            );
            layout_node(handle, handle_bounds, state, out);
        }
        [only] => {
            layout_node(only, bounds, state, out);
        }
        _ => {}
    }

    out.insert(node.id, ComputedLayout::new(bounds, bounds, measured_size));

    measured_size
}
pub(super) fn arrange_split_handle_bounds(
    axis: Axis,
    split_bounds: UiRect,
    first_bounds: UiRect,
    second_bounds: UiRect,
    handle_size: UiSize,
) -> UiRect {
    let handle_width = handle_size.width.min(split_bounds.width.max(0.0));
    let handle_height = handle_size.height.min(split_bounds.height.max(0.0));
    match axis {
        Axis::Horizontal => {
            let boundary = (first_bounds.x + first_bounds.width + second_bounds.x) * 0.5;
            let x = (boundary - handle_width * 0.5).clamp(
                split_bounds.x,
                split_bounds.x + (split_bounds.width - handle_width).max(0.0),
            );
            let y = (split_bounds.y + (split_bounds.height - handle_height) * 0.5).clamp(
                split_bounds.y,
                split_bounds.y + (split_bounds.height - handle_height).max(0.0),
            );
            UiRect::new(x, y, handle_width, handle_height)
        }
        Axis::Vertical => {
            let boundary = (first_bounds.y + first_bounds.height + second_bounds.y) * 0.5;
            let x = (split_bounds.x + (split_bounds.width - handle_width) * 0.5).clamp(
                split_bounds.x,
                split_bounds.x + (split_bounds.width - handle_width).max(0.0),
            );
            let y = (boundary - handle_height * 0.5).clamp(
                split_bounds.y,
                split_bounds.y + (split_bounds.height - handle_height).max(0.0),
            );
            UiRect::new(x, y, handle_width, handle_height)
        }
    }
}
