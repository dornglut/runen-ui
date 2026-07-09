//! File: domain/ui/ui_runtime/src/layout/engine/overlay.rs
//! Purpose: Overlay adornment measurement and placement for retained UI nodes.

use ui_layout::{CrossAxisAlignment, MainAxisAlignment, StackItem, StackLayout};
use ui_math::{UiRect, UiSize};

use crate::{
    ComputedLayout, ComputedLayoutMap, OverlayAdornmentNode, PopupPlacement, UiNode, UiRuntimeState,
};

use super::dispatch::layout_node;
use super::measure::{is_popup_node, measure_node, measure_overlay_adornment_content};

pub(super) fn layout_overlay_adornment(
    node: &UiNode,
    adornment: &OverlayAdornmentNode,
    bounds: UiRect,
    state: &UiRuntimeState,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let Some(anchor_layout) = out.get(&adornment.anchor) else {
        out.insert(node.id, ComputedLayout::new(bounds, bounds, UiSize::ZERO));
        return UiSize::ZERO;
    };

    let content_size = measure_overlay_adornment_content(node);
    let measured_size = UiSize::new(
        content_size.width.max(adornment.min_size.width),
        content_size.height.max(adornment.min_size.height),
    );
    let anchor = anchor_layout.bounds;
    let (target_x, target_y, adornment_width, adornment_height) = match adornment.placement {
        PopupPlacement::InsideLeft => (
            anchor.x + adornment.offset,
            anchor.y,
            measured_size
                .width
                .min(anchor.width)
                .min(bounds.width.max(0.0)),
            anchor.height.min(bounds.height.max(0.0)),
        ),
        PopupPlacement::InsideRight => (
            anchor.x + anchor.width - measured_size.width - adornment.offset,
            anchor.y,
            measured_size
                .width
                .min(anchor.width)
                .min(bounds.width.max(0.0)),
            anchor.height.min(bounds.height.max(0.0)),
        ),
        PopupPlacement::InsideTop => (
            anchor.x,
            anchor.y + adornment.offset,
            anchor.width.min(bounds.width.max(0.0)),
            measured_size
                .height
                .min(anchor.height)
                .min(bounds.height.max(0.0)),
        ),
        PopupPlacement::InsideBottom => (
            anchor.x,
            anchor.y + anchor.height - measured_size.height - adornment.offset,
            anchor.width.min(bounds.width.max(0.0)),
            measured_size
                .height
                .min(anchor.height)
                .min(bounds.height.max(0.0)),
        ),
        _ => {
            let adornment_width = measured_size.width.min(bounds.width.max(0.0));
            let adornment_height = measured_size.height.min(bounds.height.max(0.0));
            let (target_x, target_y) = match adornment.placement {
                PopupPlacement::BottomStart => {
                    (anchor.x, anchor.y + anchor.height + adornment.offset)
                }
                PopupPlacement::RightStart => {
                    (anchor.x + anchor.width + adornment.offset, anchor.y)
                }
                PopupPlacement::InsideTopEnd => (
                    anchor.x + anchor.width - adornment_width - adornment.offset,
                    anchor.y + adornment.offset,
                ),
                PopupPlacement::InsideBottomStart => (
                    anchor.x + adornment.offset,
                    anchor.y + anchor.height - adornment_height - adornment.offset,
                ),
                PopupPlacement::TopStart => {
                    (anchor.x + adornment.offset, anchor.y + adornment.offset)
                }
                PopupPlacement::InsideCenter => (
                    anchor.x + (anchor.width - adornment_width) * 0.5,
                    anchor.y + (anchor.height - adornment_height) * 0.5,
                ),
                PopupPlacement::InsideLeft
                | PopupPlacement::InsideRight
                | PopupPlacement::InsideTop
                | PopupPlacement::InsideBottom
                | PopupPlacement::Outside { .. } => unreachable!(),
            };
            (target_x, target_y, adornment_width, adornment_height)
        }
    };
    let x = target_x.clamp(
        bounds.x,
        bounds.x + (bounds.width - adornment_width).max(0.0),
    );
    let y = target_y.clamp(
        bounds.y,
        bounds.y + (bounds.height - adornment_height).max(0.0),
    );
    let adornment_bounds = UiRect::new(x, y, adornment_width, adornment_height);
    let child_items = node
        .children
        .iter()
        .filter(|child| !is_popup_node(child))
        .map(|child| StackItem::auto(measure_node(child)))
        .collect::<Vec<_>>();
    let children = node
        .children
        .iter()
        .filter(|child| !is_popup_node(child))
        .collect::<Vec<_>>();
    if adornment.stretch_child && children.len() == 1 {
        layout_node(children[0], adornment_bounds, state, out);
    } else {
        let arranged = StackLayout::vertical(0.0)
            .with_main_align(MainAxisAlignment::Start)
            .with_cross_align(CrossAxisAlignment::Stretch)
            .arrange(adornment_bounds, &child_items);
        for (child, child_bounds) in children.into_iter().zip(arranged) {
            layout_node(child, child_bounds, state, out);
        }
    }

    out.insert(
        node.id,
        ComputedLayout::new(adornment_bounds, adornment_bounds, measured_size),
    );
    UiSize::ZERO
}
