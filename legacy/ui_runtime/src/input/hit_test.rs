//! File: domain/ui/ui_runtime/src/input/hit_test.rs
//! Purpose: Hit testing against computed layout records.

use ui_math::UiPoint;

use crate::{ComputedLayout, ComputedLayoutMap, UiNode, UiNodeKind, UiTree, WidgetId};

pub fn hit_test_widget(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    point: UiPoint,
) -> Option<WidgetId> {
    hit_test_node(&tree.root, layouts, point, None)
}

fn hit_test_node(
    node: &UiNode,
    layouts: &ComputedLayoutMap,
    point: UiPoint,
    inherited_clip: Option<ui_math::UiRect>,
) -> Option<WidgetId> {
    let layout = layouts.get(&node.id)?;
    let clip = combine_clip(inherited_clip, clip_rect_for_node(node, layout))?;

    if !layout.bounds.contains(point) {
        return None;
    }
    if let Some(clip_rect) = clip
        && !clip_rect.contains(point)
    {
        return None;
    }

    let mut children = node.children.iter().enumerate().collect::<Vec<_>>();
    children.sort_by_key(|(index, child)| (node_layer_order(child), *index));
    for (_, child) in children.into_iter().rev() {
        if let Some(hit) = hit_test_node(child, layouts, point, clip) {
            return Some(hit);
        }
    }

    if let Some(hit) = radial_menu_hit_test(node, layout, point) {
        return Some(hit);
    }

    Some(node.id)
}

fn node_layer_order(node: &UiNode) -> u32 {
    match &node.kind {
        UiNodeKind::Popup(popup) => popup.layer_order,
        UiNodeKind::RadialMenu(radial) => radial.layer_order,
        _ => 0,
    }
}

fn clip_rect_for_node(node: &UiNode, layout: &ComputedLayout) -> Option<ui_math::UiRect> {
    match &node.kind {
        UiNodeKind::Panel(_)
        | UiNodeKind::Popup(_)
        | UiNodeKind::RadialMenu(_)
        | UiNodeKind::OverlayAdornment(_)
        | UiNodeKind::Scroll(_)
        | UiNodeKind::TextInput(_)
        | UiNodeKind::NumericInput(_)
        | UiNodeKind::Tabs(_)
        | UiNodeKind::Select(_) => Some(layout.content_bounds),
        UiNodeKind::GraphCanvas(graph_canvas) if graph_canvas.clip => Some(layout.bounds),
        UiNodeKind::GraphCanvas(_) => None,
        UiNodeKind::Button(_) => Some(layout.bounds),
        UiNodeKind::Table(_) | UiNodeKind::Tree(_) => Some(layout.bounds),
        UiNodeKind::Label(_)
        | UiNodeKind::Toggle(_)
        | UiNodeKind::Spacer(_)
        | UiNodeKind::Divider(_)
        | UiNodeKind::Image(_)
        | UiNodeKind::ProductSurface(_)
        | UiNodeKind::Stack(_)
        | UiNodeKind::ViewportSurfaceEmbed(_)
        | UiNodeKind::Split(_) => None,
    }
}

fn radial_menu_hit_test(
    node: &UiNode,
    layout: &ComputedLayout,
    point: UiPoint,
) -> Option<WidgetId> {
    let UiNodeKind::RadialMenu(radial) = &node.kind else {
        return None;
    };
    let children = node
        .children
        .iter()
        .filter(|child| !matches!(child.kind, UiNodeKind::Popup(_) | UiNodeKind::RadialMenu(_)))
        .collect::<Vec<_>>();
    if children.is_empty() {
        return None;
    }
    let center_x = layout.bounds.x + layout.bounds.width * 0.5;
    let center_y = layout.bounds.y + layout.bounds.height * 0.5;
    let dx = point.x - center_x;
    let dy = point.y - center_y;
    let distance = (dx * dx + dy * dy).sqrt();
    if distance < radial.inner_radius || distance > radial.outer_radius {
        return None;
    }
    let full_turn = std::f32::consts::TAU;
    let angle = (dy.atan2(dx) - radial.start_angle_radians).rem_euclid(full_turn);
    let wedge = full_turn / children.len() as f32;
    let index = (angle / wedge).floor() as usize;
    children
        .get(index.min(children.len() - 1))
        .map(|child| child.id)
}

fn combine_clip(
    inherited: Option<ui_math::UiRect>,
    local: Option<ui_math::UiRect>,
) -> Option<Option<ui_math::UiRect>> {
    match (inherited, local) {
        (Some(a), Some(b)) => a.intersect(b).map(Some),
        (Some(a), None) => Some(Some(a)),
        (None, Some(b)) => Some(Some(b)),
        (None, None) => Some(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ButtonNode, OverlayAdornmentNode, PopupNode, PopupSide, RadialMenuNode, StackNode,
        UiRuntimeState, compute_tree_layout,
    };
    use ui_layout::SizePolicy;
    use ui_math::{UiPoint, UiRect, UiSize};
    use ui_text::FontId;
    use ui_theme::ThemeTokens;
    use ui_tree::{UiNodeKind, UiTree};

    #[test]
    fn button_hit_test_includes_padding_bounds() {
        let theme = ThemeTokens::default();
        let button_id = WidgetId(7);
        let tree = UiTree::new(UiNode::new(
            button_id,
            UiNodeKind::Button(ButtonNode::new(
                "File",
                theme.body_small_text_style(FontId(1)),
                theme,
            )),
        ));
        let layouts = compute_tree_layout(
            &tree,
            UiRect::new(0.0, 0.0, 120.0, 32.0),
            &UiRuntimeState::default(),
        );
        let layout = layouts.get(&button_id).expect("button layout should exist");
        let point = UiPoint::new(
            layout.bounds.x + 1.0,
            layout.content_bounds.y + layout.content_bounds.height * 0.5,
        );

        assert!(layout.bounds.contains(point));
        assert!(!layout.content_bounds.contains(point));
        assert_eq!(hit_test_widget(&tree, &layouts, point), Some(button_id));
    }

    #[test]
    fn radial_menu_hit_test_selects_wedge_child() {
        let theme = ThemeTokens::default();
        let anchor_id = WidgetId(2);
        let radial_id = WidgetId(3);
        let north_id = WidgetId(4);
        let east_id = WidgetId(5);
        let south_id = WidgetId(6);
        let west_id = WidgetId(7);
        let tree = UiTree::new(UiNode::with_children(
            WidgetId(1),
            UiNodeKind::Panel(crate::PanelNode::new(theme.clone())),
            vec![
                UiNode::new(
                    anchor_id,
                    UiNodeKind::Button(ButtonNode::new(
                        "Tools",
                        theme.body_small_text_style(FontId(1)),
                        theme.clone(),
                    )),
                ),
                UiNode::with_children(
                    radial_id,
                    UiNodeKind::RadialMenu({
                        let mut radial = RadialMenuNode::anchored_to(anchor_id, theme.clone());
                        radial.inner_radius = 8.0;
                        radial.outer_radius = 64.0;
                        radial.item_size = UiSize::new(24.0, 20.0);
                        radial
                    }),
                    vec![
                        UiNode::new(
                            north_id,
                            UiNodeKind::Button(ButtonNode::new(
                                "N",
                                theme.body_small_text_style(FontId(1)),
                                theme.clone(),
                            )),
                        ),
                        UiNode::new(
                            east_id,
                            UiNodeKind::Button(ButtonNode::new(
                                "E",
                                theme.body_small_text_style(FontId(1)),
                                theme.clone(),
                            )),
                        ),
                        UiNode::new(
                            south_id,
                            UiNodeKind::Button(ButtonNode::new(
                                "S",
                                theme.body_small_text_style(FontId(1)),
                                theme.clone(),
                            )),
                        ),
                        UiNode::new(
                            west_id,
                            UiNodeKind::Button(ButtonNode::new(
                                "W",
                                theme.body_small_text_style(FontId(1)),
                                theme,
                            )),
                        ),
                    ],
                ),
            ],
        ));
        let layouts = compute_tree_layout(
            &tree,
            UiRect::new(0.0, 0.0, 240.0, 180.0),
            &UiRuntimeState::default(),
        );
        let radial_bounds = layouts
            .get(&radial_id)
            .expect("radial menu should have layout")
            .bounds;
        let north_point = UiPoint::new(
            radial_bounds.x + radial_bounds.width * 0.5,
            radial_bounds.y + 12.0,
        );
        let east_point = UiPoint::new(
            radial_bounds.x + radial_bounds.width - 12.0,
            radial_bounds.y + radial_bounds.height * 0.5,
        );

        assert_eq!(
            hit_test_widget(&tree, &layouts, north_point),
            Some(north_id)
        );
        assert_eq!(hit_test_widget(&tree, &layouts, east_point), Some(east_id));
    }

    #[test]
    fn higher_layer_popup_wins_hit_testing_over_later_sibling() {
        let theme = ThemeTokens::default();
        let anchor_id = WidgetId(2);
        let high_popup_id = WidgetId(3);
        let high_item_id = WidgetId(4);
        let low_popup_id = WidgetId(5);
        let low_item_id = WidgetId(6);
        let text_style = theme.body_small_text_style(FontId(1));
        let mut high_popup = PopupNode::anchored_bottom_start(anchor_id, theme.clone());
        high_popup.layer_order = 4;
        let mut low_popup = PopupNode::anchored_bottom_start(anchor_id, theme.clone());
        low_popup.layer_order = 2;
        let tree = UiTree::new(UiNode::with_children(
            WidgetId(1),
            UiNodeKind::Panel(crate::PanelNode::new(theme.clone())),
            vec![
                UiNode::new(
                    anchor_id,
                    UiNodeKind::Button(ButtonNode::new("Open", text_style.clone(), theme.clone())),
                ),
                UiNode::with_children(
                    high_popup_id,
                    UiNodeKind::Popup(high_popup),
                    vec![UiNode::new(
                        high_item_id,
                        UiNodeKind::Button(ButtonNode::new(
                            "High",
                            text_style.clone(),
                            theme.clone(),
                        )),
                    )],
                ),
                UiNode::with_children(
                    low_popup_id,
                    UiNodeKind::Popup(low_popup),
                    vec![UiNode::new(
                        low_item_id,
                        UiNodeKind::Button(ButtonNode::new("Low", text_style, theme)),
                    )],
                ),
            ],
        ));
        let layouts = compute_tree_layout(
            &tree,
            UiRect::new(0.0, 0.0, 320.0, 220.0),
            &UiRuntimeState::default(),
        );
        let low_item = layouts
            .get(&low_item_id)
            .expect("low popup item should have layout")
            .bounds;
        let point = UiPoint::new(
            low_item.x + low_item.width * 0.5,
            low_item.y + low_item.height * 0.5,
        );

        assert_eq!(hit_test_widget(&tree, &layouts, point), Some(high_item_id));
    }

    #[test]
    fn structural_chrome_slots_hit_their_child_slot_before_the_host() {
        let theme = ThemeTokens::default();
        let chrome_id = WidgetId(2);
        let close_id = WidgetId(3);
        let label_id = WidgetId(4);
        let active_indicator_id = WidgetId(5);
        let text_style = theme.body_small_text_style(FontId(1));
        let mut chrome = StackNode::horizontal(0.0);
        chrome.child_main_policies = vec![
            SizePolicy::Fixed(18.0),
            SizePolicy::Fixed(80.0),
            SizePolicy::Fixed(18.0),
        ];
        let mut close = ButtonNode::new("x", text_style.clone(), theme.clone());
        close.padding = ui_math::UiInsets::ZERO;
        close.min_size = UiSize::new(18.0, 18.0);
        let mut label = ButtonNode::new("Viewport", text_style.clone(), theme.clone());
        label.fill_width = true;
        let mut indicator = ButtonNode::new("", text_style, theme.clone());
        indicator.padding = ui_math::UiInsets::ZERO;
        indicator.min_size = UiSize::new(18.0, 18.0);
        let tree = UiTree::new(UiNode::with_children(
            WidgetId(1),
            UiNodeKind::Panel(crate::PanelNode::new(theme)),
            vec![UiNode::with_children(
                chrome_id,
                UiNodeKind::Stack(chrome),
                vec![
                    UiNode::new(close_id, UiNodeKind::Button(close)),
                    UiNode::new(label_id, UiNodeKind::Button(label)),
                    UiNode::new(active_indicator_id, UiNodeKind::Button(indicator)),
                ],
            )],
        ));
        let layouts = compute_tree_layout(
            &tree,
            UiRect::new(0.0, 0.0, 160.0, 48.0),
            &UiRuntimeState::default(),
        );
        let close = layouts.get(&close_id).expect("close slot layout").bounds;
        let label = layouts.get(&label_id).expect("label slot layout").bounds;
        let indicator = layouts
            .get(&active_indicator_id)
            .expect("indicator slot layout")
            .bounds;

        assert_eq!(
            hit_test_widget(&tree, &layouts, UiPoint::new(close.x + 1.0, close.y + 1.0)),
            Some(close_id)
        );
        assert_eq!(
            hit_test_widget(&tree, &layouts, UiPoint::new(label.x + 1.0, label.y + 1.0)),
            Some(label_id)
        );
        assert_eq!(
            hit_test_widget(
                &tree,
                &layouts,
                UiPoint::new(indicator.x + 1.0, indicator.y + 1.0)
            ),
            Some(active_indicator_id)
        );
        assert!(close.x + close.width <= label.x);
        assert!(label.x + label.width <= indicator.x);
    }

    #[test]
    fn overlay_adornment_preview_child_wins_hit_testing_over_anchor() {
        let theme = ThemeTokens::default();
        let anchor_id = WidgetId(2);
        let overlay_id = WidgetId(3);
        let preview_id = WidgetId(4);
        let text_style = theme.body_small_text_style(FontId(1));
        let mut anchor = ButtonNode::new("Area", text_style.clone(), theme.clone());
        anchor.min_size = UiSize::new(120.0, 80.0);
        let mut preview = ButtonNode::new("Drop", text_style, theme.clone());
        preview.fill_width = true;
        let tree = UiTree::new(UiNode::with_children(
            WidgetId(1),
            UiNodeKind::Panel(crate::PanelNode::new(theme)),
            vec![
                UiNode::new(anchor_id, UiNodeKind::Button(anchor)),
                UiNode::with_children(
                    overlay_id,
                    UiNodeKind::OverlayAdornment(OverlayAdornmentNode::anchored_inside_edge(
                        anchor_id,
                        PopupSide::Left,
                        24.0,
                    )),
                    vec![UiNode::new(preview_id, UiNodeKind::Button(preview))],
                ),
            ],
        ));
        let layouts = compute_tree_layout(
            &tree,
            UiRect::new(0.0, 0.0, 180.0, 120.0),
            &UiRuntimeState::default(),
        );
        let preview_bounds = layouts
            .get(&preview_id)
            .expect("preview child should have layout")
            .bounds;

        assert_eq!(
            hit_test_widget(
                &tree,
                &layouts,
                UiPoint::new(
                    preview_bounds.x + preview_bounds.width * 0.5,
                    preview_bounds.y + preview_bounds.height * 0.5,
                )
            ),
            Some(preview_id)
        );
    }
}
