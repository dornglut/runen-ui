use ui_math::{UiRect, UiSize};
use ui_render_data::UiPrimitiveFamily;
use ui_runtime::{
    ComputedLayout, ComputedLayoutMap, InteractionVisualState, PanelNode, UiNode, UiNodeKind,
    UiRuntimeOutputEvidenceSource, UiTree, WidgetId, build_ui_frame_with_render_output_evidence,
    expected_panel_output,
};
use ui_text::{FontAtlasSource, FontId, MsdfFontAtlas};
use ui_theme::ThemeTokens;

#[test]
fn runtime_output_evidence_is_derived_from_built_ui_frame() {
    let root_id = WidgetId(1);
    let tree = UiTree::new(UiNode::new(
        root_id,
        UiNodeKind::Panel(PanelNode::new(ThemeTokens::default())),
    ));
    let mut layouts = ComputedLayoutMap::default();
    layouts.insert(
        root_id,
        ComputedLayout::new(
            UiRect::new(0.0, 0.0, 128.0, 64.0),
            UiRect::new(4.0, 4.0, 120.0, 56.0),
            UiSize::new(128.0, 64.0),
        ),
    );

    let output = build_ui_frame_with_render_output_evidence(
        &tree,
        &layouts,
        UiSize::new(128.0, 64.0),
        InteractionVisualState::default(),
        &EmptyAtlasSource,
        "runenwerk.ui.runtime.evidence.panel",
        UiRuntimeOutputEvidenceSource::new("ui_runtime.build_ui_frame", "panel.runtime"),
        expected_panel_output(),
    );

    assert!(
        output.evidence.is_valid(),
        "{:?}",
        output.evidence.diagnostics
    );
    assert_eq!(output.frame.surfaces.len(), 1);
    assert_eq!(
        output
            .evidence
            .frame_summary
            .count_for_family(UiPrimitiveFamily::Rect),
        1
    );
    assert_eq!(
        output
            .evidence
            .frame_summary
            .count_for_family(UiPrimitiveFamily::Border),
        1
    );
    assert_eq!(
        output
            .evidence
            .frame_summary
            .count_for_family(UiPrimitiveFamily::Clip),
        2
    );
}

struct EmptyAtlasSource;

impl FontAtlasSource for EmptyAtlasSource {
    fn atlas(&self, _font_id: FontId) -> Option<&MsdfFontAtlas> {
        None
    }
}
