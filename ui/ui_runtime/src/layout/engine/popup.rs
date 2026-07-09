//! File: domain/ui/ui_runtime/src/layout/engine/popup.rs
//! Purpose: Popup and radial menu placement for retained UI nodes.

use ui_layout::{CrossAxisAlignment, MainAxisAlignment, StackItem, StackLayout};
use ui_math::{UiRect, UiSize};

use crate::{
    ComputedLayout, ComputedLayoutMap, PopupAlign, PopupFlipPolicy, PopupNode, PopupPlacement,
    PopupSide, RadialMenuAnchor, RadialMenuNode, UiNode, UiNodeKind, UiRuntimeState,
};

use super::dispatch::layout_node;
use super::measure::{is_popup_node, measure_node, measure_popup_content};

pub(super) fn layout_popup(
    node: &UiNode,
    popup: &PopupNode,
    bounds: UiRect,
    state: &UiRuntimeState,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let Some(anchor_layout) = out.get(&popup.anchor) else {
        out.insert(node.id, ComputedLayout::new(bounds, bounds, UiSize::ZERO));
        return UiSize::ZERO;
    };

    let content_size = measure_popup_content(node, popup);
    let measured_size = UiSize::new(
        (content_size.width + popup.padding.horizontal()).max(popup.min_size.width),
        (content_size.height + popup.padding.vertical()).max(popup.min_size.height),
    );
    let anchor = anchor_layout.bounds;
    let (target_x, target_y, popup_width, popup_height) = match popup.placement {
        PopupPlacement::Outside {
            preferred_side,
            align,
            flip_policy,
        } => placed_outside_popup_bounds(
            anchor,
            bounds,
            measured_size,
            popup.offset,
            preferred_side,
            align,
            flip_policy,
        ),
        _ => {
            let popup_width = measured_size.width.min(bounds.width.max(0.0));
            let popup_height = measured_size.height.min(bounds.height.max(0.0));
            let (target_x, target_y) = match popup.placement {
                PopupPlacement::BottomStart => (anchor.x, anchor.y + anchor.height + popup.offset),
                PopupPlacement::RightStart => (anchor.x + anchor.width + popup.offset, anchor.y),
                PopupPlacement::InsideTopEnd => (
                    anchor.x + anchor.width - popup_width - popup.offset,
                    anchor.y + popup.offset,
                ),
                PopupPlacement::InsideBottomStart => (
                    anchor.x + popup.offset,
                    anchor.y + anchor.height - popup_height - popup.offset,
                ),
                PopupPlacement::TopStart => (anchor.x + popup.offset, anchor.y + popup.offset),
                PopupPlacement::InsideLeft => (anchor.x + popup.offset, anchor.y),
                PopupPlacement::InsideRight => (
                    anchor.x + anchor.width - popup_width - popup.offset,
                    anchor.y,
                ),
                PopupPlacement::InsideTop => (anchor.x, anchor.y + popup.offset),
                PopupPlacement::InsideBottom => (
                    anchor.x,
                    anchor.y + anchor.height - popup_height - popup.offset,
                ),
                PopupPlacement::InsideCenter => (
                    anchor.x + (anchor.width - popup_width) * 0.5,
                    anchor.y + (anchor.height - popup_height) * 0.5,
                ),
                PopupPlacement::Outside { .. } => unreachable!(),
            };
            (target_x, target_y, popup_width, popup_height)
        }
    };
    let x = target_x.clamp(bounds.x, bounds.x + (bounds.width - popup_width).max(0.0));
    let y = target_y.clamp(bounds.y, bounds.y + (bounds.height - popup_height).max(0.0));
    let popup_bounds = UiRect::new(x, y, popup_width, popup_height);
    let content_bounds = popup_bounds.inset(popup.padding);

    let normal_children = node
        .children
        .iter()
        .filter(|child| !is_popup_node(child))
        .collect::<Vec<_>>();
    let stretch_single_scroll_child = normal_children.len() == 1
        && matches!(normal_children[0].kind, UiNodeKind::Scroll(_))
        && measured_size.height > popup_bounds.height + f32::EPSILON;
    let child_items = normal_children
        .iter()
        .map(|child| {
            if stretch_single_scroll_child {
                StackItem::flex(measure_node(child), 1.0)
            } else {
                StackItem::auto(measure_node(child))
            }
        })
        .collect::<Vec<_>>();
    let arranged = StackLayout::vertical(popup.gap)
        .with_main_align(MainAxisAlignment::Start)
        .with_cross_align(CrossAxisAlignment::Stretch)
        .arrange(content_bounds, &child_items);
    for (child, child_bounds) in normal_children.into_iter().zip(arranged) {
        layout_node(child, child_bounds, state, out);
    }

    out.insert(
        node.id,
        ComputedLayout::new(popup_bounds, content_bounds, measured_size),
    );

    measured_size
}
pub(super) fn placed_outside_popup_bounds(
    anchor: UiRect,
    bounds: UiRect,
    measured_size: UiSize,
    offset: f32,
    preferred_side: PopupSide,
    align: PopupAlign,
    flip_policy: PopupFlipPolicy,
) -> (f32, f32, f32, f32) {
    let side = outside_popup_side(
        anchor,
        bounds,
        measured_size,
        offset,
        preferred_side,
        flip_policy,
    );
    let available_main = outside_popup_available_main(anchor, bounds, offset, side);
    let popup_width = match side {
        PopupSide::Left | PopupSide::Right => measured_size
            .width
            .min(available_main)
            .min(bounds.width.max(0.0)),
        PopupSide::Top | PopupSide::Bottom => measured_size.width.min(bounds.width.max(0.0)),
    };
    let popup_height = match side {
        PopupSide::Top | PopupSide::Bottom => measured_size
            .height
            .min(available_main)
            .min(bounds.height.max(0.0)),
        PopupSide::Left | PopupSide::Right => measured_size.height.min(bounds.height.max(0.0)),
    };

    let target_x = match side {
        PopupSide::Bottom | PopupSide::Top => {
            aligned_popup_cross_position(anchor.x, anchor.width, popup_width, align)
        }
        PopupSide::Left => anchor.x - popup_width - offset,
        PopupSide::Right => anchor.x + anchor.width + offset,
    };
    let target_y = match side {
        PopupSide::Bottom => anchor.y + anchor.height + offset,
        PopupSide::Top => anchor.y - popup_height - offset,
        PopupSide::Left | PopupSide::Right => {
            aligned_popup_cross_position(anchor.y, anchor.height, popup_height, align)
        }
    };

    (target_x, target_y, popup_width, popup_height)
}
pub(super) fn outside_popup_side(
    anchor: UiRect,
    bounds: UiRect,
    measured_size: UiSize,
    offset: f32,
    preferred_side: PopupSide,
    flip_policy: PopupFlipPolicy,
) -> PopupSide {
    if matches!(flip_policy, PopupFlipPolicy::None) {
        return preferred_side;
    }

    let preferred_available = outside_popup_available_main(anchor, bounds, offset, preferred_side);
    let preferred_required = outside_popup_required_main(measured_size, preferred_side);
    if preferred_available >= preferred_required {
        return preferred_side;
    }

    let opposite = opposite_popup_side(preferred_side);
    let opposite_available = outside_popup_available_main(anchor, bounds, offset, opposite);
    if opposite_available > preferred_available {
        opposite
    } else {
        preferred_side
    }
}
pub(super) fn outside_popup_available_main(
    anchor: UiRect,
    bounds: UiRect,
    offset: f32,
    side: PopupSide,
) -> f32 {
    match side {
        PopupSide::Top => (anchor.y - bounds.y - offset).max(0.0),
        PopupSide::Bottom => {
            (bounds.y + bounds.height - (anchor.y + anchor.height) - offset).max(0.0)
        }
        PopupSide::Left => (anchor.x - bounds.x - offset).max(0.0),
        PopupSide::Right => (bounds.x + bounds.width - (anchor.x + anchor.width) - offset).max(0.0),
    }
}
pub(super) fn outside_popup_required_main(measured_size: UiSize, side: PopupSide) -> f32 {
    match side {
        PopupSide::Top | PopupSide::Bottom => measured_size.height,
        PopupSide::Left | PopupSide::Right => measured_size.width,
    }
}
pub(super) fn opposite_popup_side(side: PopupSide) -> PopupSide {
    match side {
        PopupSide::Top => PopupSide::Bottom,
        PopupSide::Bottom => PopupSide::Top,
        PopupSide::Left => PopupSide::Right,
        PopupSide::Right => PopupSide::Left,
    }
}
pub(super) fn aligned_popup_cross_position(
    anchor_cross_position: f32,
    anchor_cross_size: f32,
    popup_cross_size: f32,
    align: PopupAlign,
) -> f32 {
    match align {
        PopupAlign::Start => anchor_cross_position,
        PopupAlign::Center => anchor_cross_position + (anchor_cross_size - popup_cross_size) * 0.5,
        PopupAlign::End => anchor_cross_position + anchor_cross_size - popup_cross_size,
    }
}
pub(super) fn layout_radial_menu(
    node: &UiNode,
    radial: &RadialMenuNode,
    bounds: UiRect,
    state: &UiRuntimeState,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let outer_radius = radial.outer_radius.max(radial.inner_radius).max(1.0);
    let menu_size = UiSize::new(outer_radius * 2.0, outer_radius * 2.0);
    let Some(anchor_center) = radial_anchor_center(radial, out) else {
        out.insert(node.id, ComputedLayout::new(bounds, bounds, UiSize::ZERO));
        return UiSize::ZERO;
    };
    let x = (anchor_center.x - outer_radius).clamp(
        bounds.x,
        bounds.x + (bounds.width - menu_size.width).max(0.0),
    );
    let y = (anchor_center.y - outer_radius).clamp(
        bounds.y,
        bounds.y + (bounds.height - menu_size.height).max(0.0),
    );
    let menu_bounds = UiRect::new(x, y, menu_size.width, menu_size.height);
    let center_x = menu_bounds.x + menu_bounds.width * 0.5;
    let center_y = menu_bounds.y + menu_bounds.height * 0.5;
    let radius = ((radial.inner_radius + radial.outer_radius) * 0.5).max(0.0);
    let children = node
        .children
        .iter()
        .filter(|child| !is_popup_node(child))
        .collect::<Vec<_>>();
    let count = children.len().max(1) as f32;
    for (index, child) in children.into_iter().enumerate() {
        let angle = radial.start_angle_radians + index as f32 * std::f32::consts::TAU / count;
        let child_size = measure_node(child);
        let width = child_size.width.max(radial.item_size.width);
        let height = child_size.height.max(radial.item_size.height);
        let child_x = (center_x + angle.cos() * radius - width * 0.5).clamp(
            menu_bounds.x,
            menu_bounds.x + (menu_bounds.width - width).max(0.0),
        );
        let child_y = (center_y + angle.sin() * radius - height * 0.5).clamp(
            menu_bounds.y,
            menu_bounds.y + (menu_bounds.height - height).max(0.0),
        );
        layout_node(
            child,
            UiRect::new(child_x, child_y, width, height),
            state,
            out,
        );
    }

    out.insert(
        node.id,
        ComputedLayout::new(menu_bounds, menu_bounds, menu_size),
    );
    UiSize::ZERO
}
pub(super) fn radial_anchor_center(
    radial: &RadialMenuNode,
    layouts: &ComputedLayoutMap,
) -> Option<ui_math::UiPoint> {
    match radial.anchor {
        RadialMenuAnchor::Widget(anchor) => {
            let anchor_layout = layouts.get(&anchor)?;
            Some(ui_math::UiPoint::new(
                anchor_layout.bounds.x + anchor_layout.bounds.width * 0.5,
                anchor_layout.bounds.y + anchor_layout.bounds.height * 0.5,
            ))
        }
        RadialMenuAnchor::Point(point) => Some(point),
    }
}
