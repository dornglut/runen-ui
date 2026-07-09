//! Build UI frame visual states tests.

use super::*;

#[test]
fn build_ui_frame_applies_hover_and_focus_visual_states_to_button() {
    let atlas_source = TestAtlasSource {
        atlas: atlas_with_ascii(FontId(1)),
    };
    let theme = ThemeTokens::default();
    let button_id = WidgetId(21);
    let tree = UiTree::new(UiNode::new(
        button_id,
        UiNodeKind::Button(crate::ButtonNode::new(
            "Apply",
            TextStyle::default(),
            theme.clone(),
        )),
    ));
    let bounds = UiRect::new(0.0, 0.0, 140.0, 36.0);
    let layouts = compute_tree_layout(&tree, bounds, &UiRuntimeState::default());

    let base_frame = build_ui_frame(
        &tree,
        &layouts,
        bounds.size(),
        InteractionVisualState::default(),
        &atlas_source,
    );
    let hover_frame = build_ui_frame(
        &tree,
        &layouts,
        bounds.size(),
        InteractionVisualState {
            hovered_widget: Some(button_id),
            ..Default::default()
        },
        &atlas_source,
    );
    let focus_frame = build_ui_frame(
        &tree,
        &layouts,
        bounds.size(),
        InteractionVisualState {
            focused_widget: Some(button_id),
            ..Default::default()
        },
        &atlas_source,
    );

    let base_background = first_rect_paint(&base_frame).expect("base button rect should exist");
    let hover_background = first_rect_paint(&hover_frame).expect("hover button rect should exist");
    assert_ne!(
        base_background, hover_background,
        "hovered button should render a different background paint"
    );

    let base_border = first_border_paint(&base_frame).expect("base button border should exist");
    let focus_border =
        first_border_paint(&focus_frame).expect("focused button border should exist");
    assert_ne!(
        base_border, focus_border,
        "focused button should render a different border paint"
    );
}

#[test]
fn button_emission_supports_round_close_shape_and_centered_label() {
    let atlas_source = TestAtlasSource {
        atlas: atlas_with_ascii(FontId(1)),
    };
    let theme = ThemeTokens::default();
    let button_id = WidgetId(221);
    let style = TextStyle {
        font_id: FontId(1),
        font_size: 12.0,
        ..TextStyle::default()
    };
    let mut button = crate::ButtonNode::new("x", style, theme);
    button.padding = ui_math::UiInsets::ZERO;
    button.min_size = UiSize::new(18.0, 18.0);
    button.corner_radius = Some(f32::MAX);
    let tree = UiTree::new(UiNode::new(button_id, UiNodeKind::Button(button)));
    let bounds = UiRect::new(0.0, 0.0, 18.0, 18.0);
    let layouts = compute_tree_layout(&tree, bounds, &UiRuntimeState::default());
    let frame = build_ui_frame(
        &tree,
        &layouts,
        bounds.size(),
        InteractionVisualState::default(),
        &atlas_source,
    );

    let rect = frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .find_map(|primitive| match primitive {
            UiPrimitive::Rect(rect) => Some(rect),
            _ => None,
        })
        .expect("button should emit a background rect");
    assert!(
        (rect.radius - 9.0).abs() <= 0.001,
        "full corner radius should clamp to a circular 50% radius"
    );

    let glyph = frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .find_map(|primitive| match primitive {
            UiPrimitive::GlyphRun(run) => first_text_glyph(run),
            _ => None,
        })
        .expect("button label should emit a glyph");
    assert!(
        glyph.origin.x > 0.0 && glyph.origin.y > 0.0,
        "button label should be centered away from the top-left edge"
    );
}

#[test]
fn icon_button_uses_line_box_vertical_centering() {
    let mut atlas = atlas_with_ascii(FontId(1));
    let metrics = atlas.glyphs.get_mut(&'x').expect("x glyph should exist");
    metrics.plane_top = 3.0;
    metrics.plane_bottom = 0.0;
    let atlas_source = TestAtlasSource { atlas };
    let theme = ThemeTokens::default();
    let button_id = WidgetId(231);
    let style = TextStyle {
        font_id: FontId(1),
        font_size: 12.0,
        ..TextStyle::default()
    };
    let mut button = crate::ButtonNode::new("x", style, theme);
    button.padding = ui_math::UiInsets::ZERO;
    button.min_size = UiSize::new(18.0, 18.0);
    let tree = UiTree::new(UiNode::new(button_id, UiNodeKind::Button(button)));
    let bounds = UiRect::new(0.0, 0.0, 18.0, 18.0);
    let layouts = compute_tree_layout(&tree, bounds, &UiRuntimeState::default());
    let frame = build_ui_frame(
        &tree,
        &layouts,
        bounds.size(),
        InteractionVisualState::default(),
        &atlas_source,
    );
    let glyph = first_glyph(&frame).expect("button label should emit a glyph");
    assert!(
        (glyph.origin.y - 12.0).abs() <= 0.001,
        "button label line box should be centered in the button"
    );
}

#[test]
fn normal_label_keeps_line_box_vertical_centering() {
    let mut atlas = atlas_with_ascii(FontId(1));
    let metrics = atlas.glyphs.get_mut(&'x').expect("x glyph should exist");
    metrics.plane_top = 3.0;
    metrics.plane_bottom = 0.0;
    let atlas_source = TestAtlasSource { atlas };
    let style = TextStyle {
        font_id: FontId(1),
        font_size: 12.0,
        line_height: TextLineHeightPolicy::Absolute(12.0),
        ..TextStyle::default()
    };
    let mut label = LabelNode::new("x", style);
    label.constraints = ui_layout::LayoutConstraints::tight(UiSize::new(18.0, 18.0));
    let tree = UiTree::new(UiNode::new(WidgetId(241), UiNodeKind::Label(label)));
    let bounds = UiRect::new(0.0, 0.0, 18.0, 18.0);
    let layouts = compute_tree_layout(&tree, bounds, &UiRuntimeState::default());
    let frame = build_ui_frame(
        &tree,
        &layouts,
        bounds.size(),
        InteractionVisualState::default(),
        &atlas_source,
    );
    let glyph = first_glyph(&frame).expect("label should emit a glyph");

    assert!(
        (glyph.origin.y - 12.0).abs() <= 0.001,
        "normal labels should keep typographic line-box centering"
    );
}

#[test]
fn label_node_text_layout_policy_drives_runtime_overflow() {
    let atlas_source = TestAtlasSource {
        atlas: atlas_with_ascii(FontId(1)),
    };
    let style = TextStyle {
        font_id: FontId(1),
        font_size: 12.0,
        line_height: TextLineHeightPolicy::Absolute(12.0),
        ..TextStyle::default()
    };
    let mut label = LabelNode::new("overflowing label", style);
    label.constraints = ui_layout::LayoutConstraints::tight(UiSize::new(28.0, 18.0));
    label.text_layout.overflow = TextOverflowPolicy::Ellipsis(TextEllipsisPlacement::End);

    let tree = UiTree::new(UiNode::new(WidgetId(242), UiNodeKind::Label(label)));
    let bounds = UiRect::new(0.0, 0.0, 28.0, 18.0);
    let layouts = compute_tree_layout(&tree, bounds, &UiRuntimeState::default());
    let frame = build_ui_frame(
        &tree,
        &layouts,
        bounds.size(),
        InteractionVisualState::default(),
        &atlas_source,
    );
    let run = frame.surfaces[0].layers[0]
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            UiPrimitive::GlyphRun(run) => Some(run),
            _ => None,
        })
        .expect("label should emit a glyph run");

    assert!(run.overflow_evidence.ellipsized);
    assert_eq!(
        run.overflow_evidence.ellipsis_placement,
        Some(TextEllipsisPlacement::End)
    );
}
