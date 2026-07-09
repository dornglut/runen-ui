use ui_controls::{
    ControlInspectionDescriptor, ControlInspectionSection, ControlKindId, ControlRenderDescriptor,
    ControlRenderInspectionExt, LABEL_CONTROL_KIND_ID, runenwerk_control_package,
};
use ui_render_data::{UiExpectedPrimitiveCount, UiPrimitiveFamily};

#[test]
fn control_render_summary_attaches_to_catalog_inspection_read_only() {
    let package = runenwerk_control_package();
    let label_id = ControlKindId::new(LABEL_CONTROL_KIND_ID);
    let kind = package
        .control_kind(&label_id)
        .expect("label control kind should exist");
    let summary = ControlRenderDescriptor::new(label_id)
        .with_required_primitive_family(UiPrimitiveFamily::Rect)
        .with_required_primitive_family(UiPrimitiveFamily::GlyphRun)
        .with_expected_primitive_count(UiExpectedPrimitiveCount::exactly(
            UiPrimitiveFamily::GlyphRun,
            1,
        ))
        .summary();
    let inspection = ControlInspectionDescriptor::from_control_kind(&package, kind)
        .with_control_render_summary(&summary);

    assert_eq!(
        inspection.fact(
            ControlInspectionSection::Metadata,
            "render.required_primitive_families",
        ),
        Some("glyph-run,rect")
    );
    assert_eq!(
        inspection.fact(
            ControlInspectionSection::Metadata,
            "render.expected_primitive_counts",
        ),
        Some("glyph-run:1")
    );
    assert_eq!(
        inspection.fact(
            ControlInspectionSection::Metadata,
            "render.has_backend_render_behavior",
        ),
        Some("false")
    );
}
