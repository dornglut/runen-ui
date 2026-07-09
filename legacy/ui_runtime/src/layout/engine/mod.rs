//! File: domain/ui/ui_runtime/src/layout/engine/mod.rs
//! Purpose: Retained-tree layout computation for ui_runtime.

mod containers;
mod controls;
mod dispatch;
mod measure;
mod overlay;
mod popup;
mod scroll;
mod surface;

use ui_math::UiRect;

use crate::{ComputedLayoutMap, UiRuntimeState, UiTree};

pub fn compute_tree_layout(
    tree: &UiTree,
    bounds: UiRect,
    state: &UiRuntimeState,
) -> ComputedLayoutMap {
    let mut out = ComputedLayoutMap::new();
    dispatch::layout_node(&tree.root, bounds, state, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::compute_tree_layout;
    use crate::{
        ButtonNode, DividerNode, ImageNode, PanelNode, PopupAlign, PopupFlipPolicy, PopupNode,
        PopupSide, ScrollNode, SpacerNode, StackNode, UiNode, UiNodeKind, UiRuntimeState, UiTree,
        WidgetId,
    };
    use ui_layout::SizePolicy;
    use ui_math::{Axis, UiRect, UiSize};
    use ui_render_data::{UiDrawKey, UiPaint};
    use ui_theme::UiColor;

    #[test]
    fn spacer_layout_preserves_min_size_and_stack_position() {
        let spacer_id = WidgetId(2);
        let button_id = WidgetId(3);
        let tree = UiTree::new(UiNode::with_children(
            WidgetId(1),
            UiNodeKind::Stack(StackNode::vertical(4.0)),
            vec![
                UiNode::new(
                    spacer_id,
                    UiNodeKind::Spacer(SpacerNode::new(UiSize::new(20.0, 12.0))),
                ),
                UiNode::new(
                    button_id,
                    UiNodeKind::Button(ButtonNode::new(
                        "Apply",
                        ui_text::TextStyle::default(),
                        ui_theme::ThemeTokens::default(),
                    )),
                ),
            ],
        ));

        let layouts = compute_tree_layout(
            &tree,
            UiRect::new(0.0, 0.0, 160.0, 80.0),
            &UiRuntimeState::default(),
        );

        let spacer = layouts.get(&spacer_id).expect("spacer layout should exist");
        let button = layouts.get(&button_id).expect("button layout should exist");

        assert_eq!(spacer.measured_size, UiSize::new(160.0, 12.0));
        assert_eq!(spacer.bounds.height, 12.0);
        assert!((button.bounds.y - 16.0).abs() < 0.001);
    }

    #[test]
    fn popup_layout_anchors_without_consuming_stack_space() {
        let anchor_id = WidgetId(2);
        let content_id = WidgetId(3);
        let popup_id = WidgetId(4);
        let popup_item_id = WidgetId(5);
        let theme = ui_theme::ThemeTokens::default();
        let tree = UiTree::new(UiNode::with_children(
            WidgetId(1),
            UiNodeKind::Panel(PanelNode::new(theme.clone())),
            vec![UiNode::with_children(
                WidgetId(10),
                UiNodeKind::Stack(StackNode::vertical(4.0)),
                vec![
                    UiNode::new(
                        anchor_id,
                        UiNodeKind::Button(ButtonNode::new(
                            "File",
                            ui_text::TextStyle::default(),
                            theme.clone(),
                        )),
                    ),
                    UiNode::new(
                        content_id,
                        UiNodeKind::Spacer(SpacerNode::new(UiSize::new(40.0, 30.0))),
                    ),
                    UiNode::with_children(
                        popup_id,
                        UiNodeKind::Popup(PopupNode::anchored_bottom_start(anchor_id, theme)),
                        vec![UiNode::new(
                            popup_item_id,
                            UiNodeKind::Button(ButtonNode::new(
                                "Save",
                                ui_text::TextStyle::default(),
                                ui_theme::ThemeTokens::default(),
                            )),
                        )],
                    ),
                ],
            )],
        ));

        let layouts = compute_tree_layout(
            &tree,
            UiRect::new(0.0, 0.0, 220.0, 160.0),
            &UiRuntimeState::default(),
        );

        let anchor = layouts.get(&anchor_id).expect("anchor layout should exist");
        let content = layouts
            .get(&content_id)
            .expect("content layout should exist");
        let popup = layouts.get(&popup_id).expect("popup layout should exist");

        assert!((content.bounds.y - (anchor.bounds.y + anchor.bounds.height + 4.0)).abs() < 0.001);
        assert!(popup.bounds.y >= anchor.bounds.y + anchor.bounds.height);
        assert!(popup.bounds.y < content.bounds.y + content.bounds.height);
    }

    #[test]
    fn outside_popup_flips_above_when_bottom_space_is_insufficient() {
        let anchor_id = WidgetId(2);
        let popup_id = WidgetId(3);
        let theme = ui_theme::ThemeTokens::default();
        let tree = UiTree::new(UiNode::with_children(
            WidgetId(1),
            UiNodeKind::Panel(PanelNode::new(theme.clone())),
            vec![UiNode::with_children(
                WidgetId(10),
                UiNodeKind::Stack(StackNode::vertical(4.0)),
                vec![
                    UiNode::new(
                        WidgetId(11),
                        UiNodeKind::Spacer(SpacerNode::new(UiSize::new(20.0, 112.0))),
                    ),
                    UiNode::new(
                        anchor_id,
                        UiNodeKind::Button(ButtonNode::new(
                            "Menu",
                            ui_text::TextStyle::default(),
                            theme.clone(),
                        )),
                    ),
                    UiNode::with_children(
                        popup_id,
                        UiNodeKind::Popup(PopupNode::anchored_outside(
                            anchor_id,
                            PopupSide::Bottom,
                            PopupAlign::Start,
                            PopupFlipPolicy::FlipToFit,
                            theme.clone(),
                        )),
                        vec![UiNode::new(
                            WidgetId(12),
                            UiNodeKind::Button(ButtonNode::new(
                                "Item",
                                ui_text::TextStyle::default(),
                                theme,
                            )),
                        )],
                    ),
                ],
            )],
        ));

        let layouts = compute_tree_layout(
            &tree,
            UiRect::new(0.0, 0.0, 220.0, 160.0),
            &UiRuntimeState::default(),
        );
        let anchor = layouts.get(&anchor_id).expect("anchor layout should exist");
        let popup = layouts.get(&popup_id).expect("popup layout should exist");

        assert!(
            popup.bounds.y + popup.bounds.height <= anchor.bounds.y,
            "bottom-preferred menu should flip above instead of covering its anchor"
        );
    }

    #[test]
    fn outside_popup_stays_below_when_bottom_space_fits() {
        let anchor_id = WidgetId(2);
        let popup_id = WidgetId(3);
        let theme = ui_theme::ThemeTokens::default();
        let tree = UiTree::new(UiNode::with_children(
            WidgetId(1),
            UiNodeKind::Panel(PanelNode::new(theme.clone())),
            vec![
                UiNode::new(
                    anchor_id,
                    UiNodeKind::Button(ButtonNode::new(
                        "Menu",
                        ui_text::TextStyle::default(),
                        theme.clone(),
                    )),
                ),
                UiNode::with_children(
                    popup_id,
                    UiNodeKind::Popup(PopupNode::anchored_outside(
                        anchor_id,
                        PopupSide::Bottom,
                        PopupAlign::Start,
                        PopupFlipPolicy::FlipToFit,
                        theme.clone(),
                    )),
                    vec![UiNode::new(
                        WidgetId(12),
                        UiNodeKind::Button(ButtonNode::new(
                            "Item",
                            ui_text::TextStyle::default(),
                            theme,
                        )),
                    )],
                ),
            ],
        ));

        let layouts = compute_tree_layout(
            &tree,
            UiRect::new(0.0, 0.0, 220.0, 160.0),
            &UiRuntimeState::default(),
        );
        let anchor = layouts.get(&anchor_id).expect("anchor layout should exist");
        let popup = layouts.get(&popup_id).expect("popup layout should exist");

        assert!(popup.bounds.y >= anchor.bounds.y + anchor.bounds.height);
    }

    #[test]
    fn outside_popup_caps_oversized_menu_to_larger_available_side() {
        let anchor_id = WidgetId(2);
        let popup_id = WidgetId(3);
        let theme = ui_theme::ThemeTokens::default();
        let popup_items = (0..12)
            .map(|index| {
                UiNode::new(
                    WidgetId(20 + index),
                    UiNodeKind::Button(ButtonNode::new(
                        format!("Item {index}"),
                        ui_text::TextStyle::default(),
                        theme.clone(),
                    )),
                )
            })
            .collect::<Vec<_>>();
        let tree = UiTree::new(UiNode::with_children(
            WidgetId(1),
            UiNodeKind::Panel(PanelNode::new(theme.clone())),
            vec![UiNode::with_children(
                WidgetId(10),
                UiNodeKind::Stack(StackNode::vertical(4.0)),
                vec![
                    UiNode::new(
                        WidgetId(11),
                        UiNodeKind::Spacer(SpacerNode::new(UiSize::new(20.0, 96.0))),
                    ),
                    UiNode::new(
                        anchor_id,
                        UiNodeKind::Button(ButtonNode::new(
                            "Menu",
                            ui_text::TextStyle::default(),
                            theme.clone(),
                        )),
                    ),
                    UiNode::with_children(
                        popup_id,
                        UiNodeKind::Popup(PopupNode::anchored_outside(
                            anchor_id,
                            PopupSide::Bottom,
                            PopupAlign::Start,
                            PopupFlipPolicy::FlipToFit,
                            theme,
                        )),
                        popup_items,
                    ),
                ],
            )],
        ));

        let bounds = UiRect::new(0.0, 0.0, 220.0, 180.0);
        let layouts = compute_tree_layout(&tree, bounds, &UiRuntimeState::default());
        let anchor = layouts.get(&anchor_id).expect("anchor layout should exist");
        let popup = layouts.get(&popup_id).expect("popup layout should exist");

        assert!(popup.bounds.y >= bounds.y);
        assert!(popup.bounds.y + popup.bounds.height <= anchor.bounds.y);
        assert!(popup.bounds.height < popup.measured_size.height);
    }

    #[test]
    fn popup_menu_stretches_scroll_list_items_after_clamp() {
        let anchor_id = WidgetId(2);
        let popup_id = WidgetId(3);
        let scroll_id = WidgetId(4);
        let list_id = WidgetId(5);
        let theme = ui_theme::ThemeTokens::default();
        let text_style = ui_text::TextStyle::default();
        let mut menu_items = Vec::new();
        for index in 0..10 {
            let mut button = ButtonNode::new(
                format!("Long menu item label {index}"),
                text_style.clone(),
                theme.clone(),
            );
            button.fill_width = true;
            menu_items.push(UiNode::new(
                WidgetId(20 + index),
                UiNodeKind::Button(button),
            ));
        }

        let tree = UiTree::new(UiNode::with_children(
            WidgetId(1),
            UiNodeKind::Panel(PanelNode::new(theme.clone())),
            vec![
                UiNode::new(
                    anchor_id,
                    UiNodeKind::Button(ButtonNode::new("Menu", text_style, theme.clone())),
                ),
                UiNode::with_children(
                    popup_id,
                    UiNodeKind::Popup(PopupNode::anchored_outside(
                        anchor_id,
                        PopupSide::Bottom,
                        PopupAlign::Start,
                        PopupFlipPolicy::FlipToFit,
                        theme.clone(),
                    )),
                    vec![UiNode::with_children(
                        scroll_id,
                        UiNodeKind::Scroll(ScrollNode::vertical(theme.clone())),
                        vec![UiNode::with_children(
                            list_id,
                            UiNodeKind::Stack(StackNode::vertical(theme.spacing.xs)),
                            menu_items,
                        )],
                    )],
                ),
            ],
        ));

        let bounds = UiRect::new(0.0, 0.0, 180.0, 128.0);
        let layouts = compute_tree_layout(&tree, bounds, &UiRuntimeState::default());
        let popup = layouts.get(&popup_id).expect("popup should lay out");
        let scroll = layouts
            .get(&scroll_id)
            .expect("scroll fallback should lay out");
        let list = layouts.get(&list_id).expect("menu list should lay out");
        let first_item = layouts
            .get(&WidgetId(20))
            .expect("first menu item should lay out");

        assert!(
            popup.bounds.width <= bounds.width,
            "popup width should clamp to available frame width"
        );
        assert!(
            popup.bounds.height < popup.measured_size.height,
            "popup should retain measured height while clamping visible height"
        );
        assert_eq!(
            scroll.bounds, popup.content_bounds,
            "single scroll child should stretch to popup content bounds after clamp"
        );
        assert_eq!(
            list.bounds.width, scroll.content_bounds.width,
            "menu list should fill the clamped scroll viewport width"
        );
        assert_eq!(
            first_item.bounds.width, list.content_bounds.width,
            "fill-width menu items should stretch to measured menu width"
        );
        assert!(
            list.bounds.height > scroll.content_bounds.height,
            "menu list should preserve overflow for scroll fallback"
        );
    }

    #[test]
    fn divider_layout_respects_axis_thickness_and_fixed_length() {
        let horizontal_id = WidgetId(10);
        let vertical_id = WidgetId(11);
        let color = UiColor::new(0.2, 0.3, 0.4, 1.0);
        let tree = UiTree::new(UiNode::with_children(
            WidgetId(1),
            UiNodeKind::Stack(StackNode::vertical(2.0)),
            vec![
                UiNode::new(
                    horizontal_id,
                    UiNodeKind::Divider(DividerNode::new(
                        Axis::Horizontal,
                        2.0,
                        SizePolicy::Fixed(32.0),
                        color,
                    )),
                ),
                UiNode::new(
                    vertical_id,
                    UiNodeKind::Divider(DividerNode::new(
                        Axis::Vertical,
                        3.0,
                        SizePolicy::Fixed(24.0),
                        color,
                    )),
                ),
            ],
        ));

        let layouts = compute_tree_layout(
            &tree,
            UiRect::new(0.0, 0.0, 120.0, 80.0),
            &UiRuntimeState::default(),
        );

        let horizontal = layouts
            .get(&horizontal_id)
            .expect("horizontal divider layout should exist");
        let vertical = layouts
            .get(&vertical_id)
            .expect("vertical divider layout should exist");

        assert_eq!(horizontal.measured_size, UiSize::new(32.0, 2.0));
        assert_eq!(horizontal.bounds.height, 2.0);
        assert_eq!(vertical.measured_size, UiSize::new(3.0, 24.0));
        assert_eq!(vertical.bounds.width, 3.0);
    }

    #[test]
    fn image_layout_preserves_min_size() {
        let image_id = WidgetId(5);
        let tree = UiTree::new(UiNode::new(
            image_id,
            UiNodeKind::Image(ImageNode::new(
                UiDrawKey::new(7, Some(8)),
                UiRect::new(0.0, 0.0, 1.0, 1.0),
                UiPaint::WHITE,
                UiSize::new(48.0, 32.0),
            )),
        ));

        let layouts = compute_tree_layout(
            &tree,
            UiRect::new(4.0, 6.0, 24.0, 16.0),
            &UiRuntimeState::default(),
        );

        let image = layouts.get(&image_id).expect("image layout should exist");
        assert_eq!(image.bounds, UiRect::new(4.0, 6.0, 24.0, 16.0));
        assert_eq!(image.measured_size, UiSize::new(48.0, 32.0));
    }

    #[test]
    fn scroll_layout_preserves_nonzero_content_viewport_in_tight_bounds() {
        let scroll_id = WidgetId(21);
        let tree = UiTree::new(UiNode::with_children(
            scroll_id,
            UiNodeKind::Scroll(ScrollNode::vertical(ui_theme::ThemeTokens::default())),
            vec![UiNode::with_children(
                WidgetId(22),
                UiNodeKind::Stack(StackNode::vertical(2.0)),
                vec![
                    UiNode::new(
                        WidgetId(23),
                        UiNodeKind::Button(ButtonNode::new(
                            "one",
                            ui_text::TextStyle::default(),
                            ui_theme::ThemeTokens::default(),
                        )),
                    ),
                    UiNode::new(
                        WidgetId(24),
                        UiNodeKind::Button(ButtonNode::new(
                            "two",
                            ui_text::TextStyle::default(),
                            ui_theme::ThemeTokens::default(),
                        )),
                    ),
                ],
            )],
        ));

        let layouts = compute_tree_layout(
            &tree,
            UiRect::new(0.0, 0.0, 80.0, 6.0),
            &UiRuntimeState::default(),
        );
        let scroll = layouts.get(&scroll_id).expect("scroll layout should exist");

        assert!(
            scroll.content_bounds.height > 0.0,
            "tight overflow layouts should keep a non-zero content viewport for clipping",
        );
    }
}
