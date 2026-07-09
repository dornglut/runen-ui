use ui_controls::BaseControlsPlugin;
use ui_render_data::{UiPrimitive, UiSortKey};
use ui_runtime::{
    BASE_CONTROLS_GENERIC_INTERACTION_PROOF_ID, InteractionVisibleState, WidgetId,
    base_controls_generic_interaction_proof_frame, interaction_visual_proof_to_frame,
};
use ui_static_mount::UiStaticMountReport;

#[test]
fn base_controls_generic_interaction_visual_proof_static_mounts() {
    let compiled = BaseControlsPlugin::new().compile();
    let proof_frame = base_controls_generic_interaction_proof_frame(&compiled);
    let rendered = interaction_visual_proof_to_frame(&proof_frame.proof);

    let mount_report = UiStaticMountReport::from_frame(rendered.frame.clone());

    assert!(mount_report.passed());
    assert_eq!(
        rendered.proof_id,
        BASE_CONTROLS_GENERIC_INTERACTION_PROOF_ID
    );
    assert!(rendered.summary.has_main_inspector_and_report);
    assert!(rendered.summary.main_control_count >= 8);
    assert!(rendered.summary.marker_count >= 8);
    assert!(rendered.summary.inspector_requirement_count > 0);
    assert!(rendered.summary.report_row_count > 0);

    let mounted_frame = mount_report
        .mounted_frame()
        .expect("static mount should expose mounted frame evidence");
    assert!(mounted_frame.summary.has_rect_primitive);
    assert!(mounted_frame.summary.has_border_primitive);
    assert!(mounted_frame.summary.glyph_run_count > 0);
    assert!(mounted_frame.summary.draw_order_stable);
    assert!(primitive_sort_keys_are_stable(&rendered.frame));

    assert_marker(&proof_frame, WidgetId(1), InteractionVisibleState::Hovered);
    assert_marker(&proof_frame, WidgetId(1), InteractionVisibleState::Pressed);
    assert_marker(&proof_frame, WidgetId(1), InteractionVisibleState::Focused);
    assert_marker(
        &proof_frame,
        WidgetId(1),
        InteractionVisibleState::FocusVisible,
    );
    assert_marker(
        &proof_frame,
        WidgetId(1),
        InteractionVisibleState::ActivationRequested,
    );
    assert_marker(&proof_frame, WidgetId(7), InteractionVisibleState::Disabled);
    assert_marker(
        &proof_frame,
        WidgetId(7),
        InteractionVisibleState::Suppressed,
    );
    assert_marker(
        &proof_frame,
        WidgetId(4),
        InteractionVisibleState::ListActiveItemIntent,
    );
    assert_marker(
        &proof_frame,
        WidgetId(5),
        InteractionVisibleState::TreeNodeIntent,
    );
    assert_marker(
        &proof_frame,
        WidgetId(6),
        InteractionVisibleState::TableCellOrRowIntent,
    );
    assert_marker(
        &proof_frame,
        WidgetId(3),
        InteractionVisibleState::TextIntentProbe,
    );
    assert_marker(
        &proof_frame,
        WidgetId(8),
        InteractionVisibleState::ReadOnlyTextIntentProbe,
    );

    let button = proof_frame
        .proof
        .main_view
        .control(WidgetId(1))
        .expect("button proof control should exist");
    assert!(
        !button.has_current_state(InteractionVisibleState::Pressed),
        "pressed is observed evidence but not final current state"
    );
}

fn assert_marker(
    proof_frame: &ui_runtime::InteractionProofFrame,
    widget_id: WidgetId,
    state: InteractionVisibleState,
) {
    let control = proof_frame
        .proof
        .main_view
        .control(widget_id)
        .expect("visual proof should contain control");
    assert!(control.has_marker(state));
}

fn primitive_sort_keys_are_stable(frame: &ui_render_data::UiFrame) -> bool {
    let sort_keys = frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .map(primitive_sort_key)
        .collect::<Vec<_>>();
    let mut sorted = sort_keys.clone();
    sorted.sort();
    sort_keys == sorted
}

fn primitive_sort_key(primitive: &UiPrimitive) -> UiSortKey {
    match primitive {
        UiPrimitive::Rect(value) => value.sort_key,
        UiPrimitive::Border(value) => value.sort_key,
        UiPrimitive::GlyphRun(value) => value.sort_key,
        UiPrimitive::Image(value) => value.sort_key,
        UiPrimitive::Stroke(value) => value.sort_key,
        UiPrimitive::ViewportSurfaceEmbed(value) => value.sort_key,
        UiPrimitive::ProductSurface(value) => value.sort_key,
        UiPrimitive::Clip(ui_render_data::ClipPrimitive::Push { sort_key, .. })
        | UiPrimitive::Clip(ui_render_data::ClipPrimitive::Pop { sort_key }) => *sort_key,
    }
}
