//! Build UI frame scrollbars tests.

use super::*;

#[test]
fn build_ui_frame_emits_scrollbar_only_when_content_overflows() {
    let atlas_source = TestAtlasSource {
        atlas: atlas_with_ascii(FontId(1)),
    };
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let scroll_id = WidgetId(1);

    let overflow_tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Scroll(crate::ScrollNode::vertical(theme.clone())),
        vec![UiNode::with_children(
            WidgetId(2),
            UiNodeKind::Stack(crate::StackNode::vertical(theme.spacing.xs)),
            vec![
                UiNode::new(
                    WidgetId(3),
                    UiNodeKind::Button(crate::ButtonNode::new(
                        "First",
                        text_style.clone(),
                        theme.clone(),
                    )),
                ),
                UiNode::new(
                    WidgetId(4),
                    UiNodeKind::Button(crate::ButtonNode::new(
                        "Second",
                        text_style.clone(),
                        theme.clone(),
                    )),
                ),
                UiNode::new(
                    WidgetId(5),
                    UiNodeKind::Button(crate::ButtonNode::new(
                        "Third",
                        text_style.clone(),
                        theme.clone(),
                    )),
                ),
            ],
        )],
    ));
    let overflow_bounds = UiRect::new(0.0, 0.0, 120.0, 64.0);
    let overflow_layouts =
        compute_tree_layout(&overflow_tree, overflow_bounds, &UiRuntimeState::default());
    let inactive_overflow_frame = build_ui_frame(
        &overflow_tree,
        &overflow_layouts,
        overflow_bounds.size(),
        InteractionVisualState::default(),
        &atlas_source,
    );
    let active_overflow_frame = build_ui_frame(
        &overflow_tree,
        &overflow_layouts,
        overflow_bounds.size(),
        InteractionVisualState {
            active_scrollbar: Some(ScrollbarAxisTarget::new(scroll_id, Axis::Vertical)),
            ..Default::default()
        },
        &atlas_source,
    );

    let scroll_layout = overflow_layouts
        .get(&scroll_id)
        .expect("scroll layout should exist");
    let track_rect = scrollbar_geometry(
        &overflow_tree,
        scroll_id,
        &overflow_layouts,
        scroll_layout.bounds,
        scroll_layout.content_bounds,
    )
    .expect("overflowing scroll should have scrollbar geometry")
    .track_rect;
    assert!(
        !has_rect_primitive(&inactive_overflow_frame, track_rect),
        "overflowing scrollbars should stay hidden until active",
    );
    assert!(
        has_rect_primitive(&active_overflow_frame, track_rect),
        "active overflowing scroll should emit an overlay scrollbar track primitive",
    );

    let no_overflow_tree = UiTree::new(UiNode::with_children(
        WidgetId(11),
        UiNodeKind::Scroll(crate::ScrollNode::vertical(theme.clone())),
        vec![UiNode::new(
            WidgetId(12),
            UiNodeKind::Button(crate::ButtonNode::new("One", text_style, theme)),
        )],
    ));
    let no_overflow_bounds = UiRect::new(0.0, 0.0, 240.0, 128.0);
    let no_overflow_layouts = compute_tree_layout(
        &no_overflow_tree,
        no_overflow_bounds,
        &UiRuntimeState::default(),
    );
    let _no_overflow_frame = build_ui_frame(
        &no_overflow_tree,
        &no_overflow_layouts,
        no_overflow_bounds.size(),
        InteractionVisualState::default(),
        &atlas_source,
    );
    let no_overflow_scroll = no_overflow_layouts
        .get(&WidgetId(11))
        .expect("scroll layout should exist");
    assert!(
        scrollbar_geometry(
            &no_overflow_tree,
            WidgetId(11),
            &no_overflow_layouts,
            no_overflow_scroll.bounds,
            no_overflow_scroll.content_bounds,
        )
        .is_none(),
        "non-overflowing scroll should not emit a scrollbar track primitive",
    );
}

#[test]
fn build_ui_frame_reveals_two_axis_scrollbars_per_axis() {
    let atlas_source = TestAtlasSource {
        atlas: atlas_with_ascii(FontId(1)),
    };
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let scroll_id = WidgetId(31);
    let child_id = WidgetId(32);
    let rows = (0..8)
        .map(|row| {
            let columns = (0..5)
                .map(|column| {
                    let mut button = ButtonNode::new(
                        format!("Cell {row}-{column}"),
                        text_style.clone(),
                        theme.clone(),
                    );
                    button.min_size = UiSize::new(96.0, 28.0);
                    UiNode::new(
                        WidgetId(1_000 + row * 10 + column),
                        UiNodeKind::Button(button),
                    )
                })
                .collect::<Vec<_>>();
            UiNode::with_children(
                WidgetId(2_000 + row),
                UiNodeKind::Stack(crate::StackNode::horizontal(theme.spacing.xs)),
                columns,
            )
        })
        .collect::<Vec<_>>();
    let tree = UiTree::new(UiNode::with_children(
        scroll_id,
        UiNodeKind::Scroll(crate::ScrollNode::both(theme.clone())),
        vec![UiNode::with_children(
            child_id,
            UiNodeKind::Stack(crate::StackNode::vertical(theme.spacing.xs)),
            rows,
        )],
    ));
    let bounds = UiRect::new(0.0, 0.0, 180.0, 96.0);
    let layouts = compute_tree_layout(&tree, bounds, &UiRuntimeState::default());
    let scroll_layout = layouts.get(&scroll_id).expect("scroll layout should exist");
    let vertical_track = scrollbar_geometry_for_axis(
        &tree,
        scroll_id,
        &layouts,
        scroll_layout.bounds,
        scroll_layout.content_bounds,
        Axis::Vertical,
    )
    .expect("vertical scrollbar should exist")
    .track_rect;
    let horizontal_track = scrollbar_geometry_for_axis(
        &tree,
        scroll_id,
        &layouts,
        scroll_layout.bounds,
        scroll_layout.content_bounds,
        Axis::Horizontal,
    )
    .expect("horizontal scrollbar should exist")
    .track_rect;

    let inactive_frame = build_ui_frame(
        &tree,
        &layouts,
        bounds.size(),
        InteractionVisualState::default(),
        &atlas_source,
    );
    assert!(!has_rect_primitive(&inactive_frame, vertical_track));
    assert!(!has_rect_primitive(&inactive_frame, horizontal_track));

    let vertical_frame = build_ui_frame(
        &tree,
        &layouts,
        bounds.size(),
        InteractionVisualState {
            active_scrollbar: Some(ScrollbarAxisTarget::new(scroll_id, Axis::Vertical)),
            ..Default::default()
        },
        &atlas_source,
    );
    assert!(has_rect_primitive(&vertical_frame, vertical_track));
    assert!(!has_rect_primitive(&vertical_frame, horizontal_track));

    let horizontal_frame = build_ui_frame(
        &tree,
        &layouts,
        bounds.size(),
        InteractionVisualState {
            active_scrollbar: Some(ScrollbarAxisTarget::new(scroll_id, Axis::Horizontal)),
            ..Default::default()
        },
        &atlas_source,
    );
    assert!(!has_rect_primitive(&horizontal_frame, vertical_track));
    assert!(has_rect_primitive(&horizontal_frame, horizontal_track));

    let hovered_horizontal_frame = build_ui_frame(
        &tree,
        &layouts,
        bounds.size(),
        InteractionVisualState {
            hovered_scrollbar: Some(ScrollbarAxisTarget::new(scroll_id, Axis::Horizontal)),
            ..Default::default()
        },
        &atlas_source,
    );
    assert!(!has_rect_primitive(
        &hovered_horizontal_frame,
        vertical_track
    ));
    assert!(has_rect_primitive(
        &hovered_horizontal_frame,
        horizontal_track
    ));
}
