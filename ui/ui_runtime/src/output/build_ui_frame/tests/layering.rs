//! Build UI frame layering tests.

use super::*;

#[test]
fn popup_button_emits_text_on_popup_layer() {
    let atlas_source = TestAtlasSource {
        atlas: atlas_with_ascii(FontId(1)),
    };
    let theme = ThemeTokens::default();
    let anchor_id = WidgetId(2);
    let popup_id = WidgetId(3);
    let item_id = WidgetId(4);
    let text_style = theme.body_small_text_style(FontId(1));
    let mut popup_button = ButtonNode::new("Save", text_style.clone(), theme.clone());
    popup_button.fill_width = true;
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![
            UiNode::new(
                anchor_id,
                UiNodeKind::Button(ButtonNode::new("File", text_style.clone(), theme.clone())),
            ),
            UiNode::with_children(
                popup_id,
                UiNodeKind::Popup(PopupNode::anchored_bottom_start(anchor_id, theme.clone())),
                vec![UiNode::new(item_id, UiNodeKind::Button(popup_button))],
            ),
        ],
    ));
    let layouts = compute_tree_layout(
        &tree,
        UiRect::new(0.0, 0.0, 240.0, 160.0),
        &UiRuntimeState::default(),
    );

    let frame = build_ui_frame(
        &tree,
        &layouts,
        UiSize::new(240.0, 160.0),
        InteractionVisualState::default(),
        &atlas_source,
    );

    let popup_text = frame.surfaces[0].layers[0]
        .primitives
        .iter()
        .find_map(|primitive| {
            let UiPrimitive::GlyphRun(run) = primitive else {
                return None;
            };
            let text = crate::text_emission::text_from_primitive(run);
            (text == "Save").then_some(run)
        })
        .unwrap_or_else(|| {
            panic!(
                "popup button text should emit a glyph run; frame:\n{}",
                frame_signature(&frame)
            )
        });

    assert_eq!(popup_text.sort_key.layer_order, POPUP_LAYER_ORDER + 1);

    let popup_background = frame.surfaces[0].layers[0]
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            UiPrimitive::Rect(rect)
                if rect.sort_key.layer_order == POPUP_LAYER_ORDER + 1
                    && rect.sort_key.primitive_order < popup_text.sort_key.primitive_order =>
            {
                Some(rect)
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "popup background should render on the popup layer before text; frame:\n{}",
                frame_signature(&frame)
            )
        });

    assert_eq!(popup_background.sort_key.layer_order, POPUP_LAYER_ORDER + 1);
    assert!(
        popup_background.sort_key.primitive_order < popup_text.sort_key.primitive_order,
        "popup background must not render after popup text; frame:\n{}",
        frame_signature(&frame)
    );
    assert!(
        frame.surfaces[0].layers[0]
            .primitives
            .iter()
            .all(|primitive| primitive_sort_key(primitive).layer_order <= POPUP_LAYER_ORDER + 1),
        "test popup frame should not emit a higher overlay layer than the popup text; frame:\n{}",
        frame_signature(&frame)
    );
}

#[test]
fn inside_top_end_adornment_stays_in_scroll_layer_under_scrollbar() {
    let atlas_source = TestAtlasSource {
        atlas: atlas_with_ascii(FontId(1)),
    };
    let theme = ThemeTokens::default();
    let scroll_id = WidgetId(10);
    let row_id = WidgetId(11);
    let anchor_id = WidgetId(12);
    let filler_id = WidgetId(13);
    let popup_id = WidgetId(14);
    let close_id = WidgetId(15);
    let text_style = theme.body_small_text_style(FontId(1));
    let mut filler = ButtonNode::new("Very wide tab title", text_style.clone(), theme.clone());
    filler.min_size.width = 180.0;
    let mut close = ButtonNode::new("x", text_style.clone(), theme.clone());
    close.reveal_on_hover_anchor = Some(anchor_id);
    close.min_size = UiSize::new(18.0, 18.0);
    close.padding = ui_math::UiInsets::ZERO;
    let mut popup = PopupNode::anchored_inside_top_end(anchor_id, theme.clone());
    popup.offset = theme.spacing.xs;

    let tree = UiTree::new(UiNode::with_children(
        scroll_id,
        UiNodeKind::Scroll(crate::ScrollNode::horizontal(theme.clone())),
        vec![UiNode::with_children(
            row_id,
            UiNodeKind::Stack(crate::StackNode::horizontal(theme.spacing.xs)),
            vec![
                UiNode::new(
                    anchor_id,
                    UiNodeKind::Button(ButtonNode::new("Tab", text_style, theme.clone())),
                ),
                UiNode::new(filler_id, UiNodeKind::Button(filler)),
                UiNode::with_children(
                    popup_id,
                    UiNodeKind::Popup(popup),
                    vec![UiNode::new(close_id, UiNodeKind::Button(close))],
                ),
            ],
        )],
    ));
    let bounds = UiRect::new(0.0, 0.0, 96.0, 36.0);
    let layouts = compute_tree_layout(&tree, bounds, &UiRuntimeState::default());
    let frame = build_ui_frame(
        &tree,
        &layouts,
        bounds.size(),
        InteractionVisualState {
            hovered_widget: Some(anchor_id),
            active_scrollbar: Some(ScrollbarAxisTarget::new(scroll_id, Axis::Horizontal)),
            ..Default::default()
        },
        &atlas_source,
    );
    let close_text = frame.surfaces[0].layers[0]
        .primitives
        .iter()
        .find_map(|primitive| {
            let UiPrimitive::GlyphRun(run) = primitive else {
                return None;
            };
            let text = crate::text_emission::text_from_primitive(run);
            (text == "x").then_some(run)
        })
        .unwrap_or_else(|| {
            panic!(
                "inside adornment close text should emit; frame:\n{}",
                frame_signature(&frame)
            )
        });
    let scroll_layout = layouts.get(&scroll_id).expect("scroll layout should exist");
    let scrollbar_track = scrollbar_geometry(
        &tree,
        scroll_id,
        &layouts,
        scroll_layout.bounds,
        scroll_layout.content_bounds,
    )
    .expect("overflowing horizontal scroll should have a scrollbar")
    .track_rect;
    let track_rect = frame.surfaces[0].layers[0]
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            UiPrimitive::Rect(rect) if rect_approx_eq(rect.rect, scrollbar_track) => Some(rect),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "active scrollbar track should emit; frame:\n{}",
                frame_signature(&frame)
            )
        });

    assert_eq!(close_text.sort_key.layer_order, BASE_LAYER_ORDER);
    assert_eq!(track_rect.sort_key.layer_order, BASE_LAYER_ORDER);
    assert!(
        track_rect.sort_key.primitive_order > close_text.sort_key.primitive_order,
        "scrollbar must paint over in-scroll adornments; frame:\n{}",
        frame_signature(&frame)
    );
}

#[test]
fn scroll_children_and_overlays_remain_inside_scroll_clip_stack() {
    let atlas_source = TestAtlasSource {
        atlas: atlas_with_ascii(FontId(1)),
    };
    let theme = ThemeTokens::default();
    let scroll_id = WidgetId(301);
    let anchor_id = WidgetId(302);
    let popup_id = WidgetId(303);
    let popup_button_id = WidgetId(304);
    let mut anchor = crate::ButtonNode::new("Wide tab", TextStyle::default(), theme.clone());
    anchor.min_size = UiSize::new(200.0, 24.0);
    let mut close = crate::ButtonNode::new("x", TextStyle::default(), theme.clone());
    close.min_size = UiSize::new(18.0, 18.0);
    let mut popup = PopupNode::anchored_inside_top_end(anchor_id, theme.clone());
    popup.offset = 0.0;
    let row = UiNode::with_children(
        WidgetId(305),
        UiNodeKind::Stack(crate::StackNode::horizontal(0.0)),
        vec![
            UiNode::new(anchor_id, UiNodeKind::Button(anchor)),
            UiNode::with_children(
                popup_id,
                UiNodeKind::Popup(popup),
                vec![UiNode::new(popup_button_id, UiNodeKind::Button(close))],
            ),
        ],
    );
    let tree = UiTree::new(UiNode::with_children(
        scroll_id,
        UiNodeKind::Scroll(crate::ScrollNode::horizontal(theme)),
        vec![row],
    ));
    let bounds = UiRect::new(0.0, 0.0, 80.0, 24.0);
    let layouts = compute_tree_layout(&tree, bounds, &UiRuntimeState::default());
    let frame = build_ui_frame(
        &tree,
        &layouts,
        bounds.size(),
        InteractionVisualState::default(),
        &atlas_source,
    );

    let mut clip_depth = 0_i32;
    let mut popup_rect_depth = None;
    for primitive in &frame.surfaces[0].layers[0].primitives {
        match primitive {
            UiPrimitive::Clip(ClipPrimitive::Push { .. }) => clip_depth += 1,
            UiPrimitive::Clip(ClipPrimitive::Pop { .. }) => clip_depth -= 1,
            UiPrimitive::Rect(rect) if rect.rect.x >= 80.0 => {
                popup_rect_depth = Some(clip_depth);
                break;
            }
            _ => {}
        }
    }

    assert!(
        popup_rect_depth.is_some_and(|depth| depth > 0),
        "overflowing overlay primitives should still be emitted under the scroll clip"
    );
}
