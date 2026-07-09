use ui_controls::{
    ControlKindId, ControlRenderDescriptor, ControlRenderEvidenceId, LABEL_CONTROL_KIND_ID,
};
use ui_render_data::{UiExpectedPrimitiveCount, UiPrimitiveFamily};

#[test]
fn control_render_bridge_references_ui_render_data_vocabulary() {
    let summary = label_render_descriptor().summary();

    assert!(
        summary
            .required_primitive_families
            .contains(&"rect".to_owned())
    );
    assert!(
        summary
            .required_primitive_families
            .contains(&"glyph-run".to_owned())
    );
    assert!(
        summary
            .expected_primitive_counts
            .contains(&"rect:1..".to_owned())
    );
    assert!(
        summary
            .expected_primitive_counts
            .contains(&"glyph-run:1".to_owned())
    );
    assert!(
        summary
            .render_evidence_ids
            .contains(&"runenwerk.ui.controls.label.evidence.render.contract".to_owned())
    );
    assert!(!summary.has_backend_render_behavior);
}

fn label_render_descriptor() -> ControlRenderDescriptor {
    ControlRenderDescriptor::new(ControlKindId::new(LABEL_CONTROL_KIND_ID))
        .with_required_primitive_family(UiPrimitiveFamily::Rect)
        .with_required_primitive_family(UiPrimitiveFamily::GlyphRun)
        .with_expected_primitive_count(UiExpectedPrimitiveCount::at_least(
            UiPrimitiveFamily::Rect,
            1,
        ))
        .with_expected_primitive_count(UiExpectedPrimitiveCount::exactly(
            UiPrimitiveFamily::GlyphRun,
            1,
        ))
        .with_render_evidence(ControlRenderEvidenceId::new(
            "runenwerk.ui.controls.label.evidence.render.contract",
        ))
}
