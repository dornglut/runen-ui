//! Build UI frame snapshot tests.

use super::*;

#[test]
fn build_ui_frame_panel_label_snapshot_signature() {
    let atlas_source = TestAtlasSource {
        atlas: atlas_with_ascii(FontId(1)),
    };
    let theme = ThemeTokens::default();
    let text_style = TextStyle {
        font_id: FontId(1),
        font_size: 14.0,
        color: [0.9, 0.95, 1.0, 1.0],
        line_height: TextLineHeightPolicy::Absolute(18.0),
        ..TextStyle::default()
    };
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme)),
        vec![UiNode::new(
            WidgetId(2),
            UiNodeKind::Label(LabelNode::new("Overlay", text_style)),
        )],
    ));
    let bounds = UiRect::new(12.0, 16.0, 240.0, 96.0);
    let layouts = compute_tree_layout(&tree, bounds, &UiRuntimeState::default());

    let frame = build_ui_frame(
        &tree,
        &layouts,
        UiSize::new(320.0, 180.0),
        InteractionVisualState::default(),
        &atlas_source,
    );
    let signature = frame_signature(&frame);
    let expected = [
        "Rect(x=12.0 y=16.0 w=240.0 h=96.0)",
        "Border(x=12.0 y=16.0 w=240.0 h=96.0)",
        "ClipPush(x=14.0 y=18.0 w=236.0 h=92.0)",
        "GlyphRun(text=\"Overlay\" clip=true)",
        "ClipPop",
    ]
    .join("\n");
    assert_eq!(signature, expected);
}

#[test]
fn build_ui_frame_emits_viewport_embed_with_full_product_uv() {
    let atlas_source = TestAtlasSource {
        atlas: atlas_with_ascii(FontId(1)),
    };
    let tree = UiTree::new(UiNode::new(
        WidgetId(7),
        UiNodeKind::ViewportSurfaceEmbed(ViewportSurfaceEmbedNode::new(
            9,
            ViewportSurfaceEmbedSlotId::new(1),
        )),
    ));
    let layouts = compute_tree_layout(
        &tree,
        UiRect::new(10.0, 20.0, 100.0, 50.0),
        &UiRuntimeState::default(),
    );

    let frame = build_ui_frame(
        &tree,
        &layouts,
        UiSize::new(200.0, 100.0),
        InteractionVisualState::default(),
        &atlas_source,
    );
    let layer = &frame.surfaces[0].layers[0];
    let embed = layer
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            UiPrimitive::ViewportSurfaceEmbed(value) => Some(value),
            _ => None,
        })
        .expect("viewport embed primitive should exist");

    assert_eq!(embed.uv_rect, UiRect::new(0.0, 0.0, 1.0, 1.0));
}

#[test]
fn build_ui_frame_emits_divider_as_rect_and_spacer_as_no_primitive() {
    let atlas_source = TestAtlasSource {
        atlas: atlas_with_ascii(FontId(1)),
    };
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Stack(crate::StackNode::vertical(0.0)),
        vec![
            UiNode::new(
                WidgetId(2),
                UiNodeKind::Spacer(crate::SpacerNode::new(UiSize::new(8.0, 4.0))),
            ),
            UiNode::new(
                WidgetId(3),
                UiNodeKind::Divider(crate::DividerNode::new(
                    ui_math::Axis::Horizontal,
                    2.0,
                    ui_layout::SizePolicy::Fixed(40.0),
                    ui_theme::UiColor::new(0.3, 0.4, 0.5, 1.0),
                )),
            ),
        ],
    ));
    let layouts = compute_tree_layout(
        &tree,
        UiRect::new(0.0, 0.0, 100.0, 40.0),
        &UiRuntimeState::default(),
    );

    let frame = build_ui_frame(
        &tree,
        &layouts,
        UiSize::new(100.0, 40.0),
        InteractionVisualState::default(),
        &atlas_source,
    );
    let layer = &frame.surfaces[0].layers[0];
    let rects = layer
        .primitives
        .iter()
        .filter(|primitive| matches!(primitive, UiPrimitive::Rect(_)))
        .count();

    assert_eq!(rects, 1);
    assert_eq!(layer.primitives.len(), 1);
}

#[test]
fn build_ui_frame_emits_image_primitive() {
    let atlas_source = TestAtlasSource {
        atlas: atlas_with_ascii(FontId(1)),
    };
    let draw_key = UiDrawKey::new(5, Some(12));
    let uv_rect = UiRect::new(0.25, 0.25, 0.5, 0.5);
    let tint = UiPaint::rgba(0.8, 0.9, 1.0, 0.75);
    let tree = UiTree::new(UiNode::new(
        WidgetId(4),
        UiNodeKind::Image(crate::ImageNode::new(
            draw_key,
            uv_rect,
            tint,
            UiSize::new(32.0, 24.0),
        )),
    ));
    let layouts = compute_tree_layout(
        &tree,
        UiRect::new(10.0, 20.0, 64.0, 48.0),
        &UiRuntimeState::default(),
    );

    let frame = build_ui_frame(
        &tree,
        &layouts,
        UiSize::new(100.0, 80.0),
        InteractionVisualState::default(),
        &atlas_source,
    );
    let image = frame.surfaces[0].layers[0]
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            UiPrimitive::Image(value) => Some(value),
            _ => None,
        })
        .expect("image primitive should exist");

    assert_eq!(image.rect, UiRect::new(10.0, 20.0, 64.0, 48.0));
    assert_eq!(image.uv_rect, uv_rect);
    assert_eq!(image.tint, tint);
    assert_eq!(image.draw_key, draw_key);
}
