use ui_controls::{
    ControlInspectionDescriptor, ControlInspectionSection, ControlKindId, ControlLayoutDescriptor,
    ControlLayoutInspectionExt, LABEL_CONTROL_KIND_ID, runenwerk_control_package,
};
use ui_layout::{UiContainerKind, UiLayoutRole, UiScrollRequirement};

#[test]
fn control_layout_summary_attaches_to_catalog_inspection_read_only() {
    let package = runenwerk_control_package();
    let label_id = ControlKindId::new(LABEL_CONTROL_KIND_ID);
    let kind = package
        .control_kind(&label_id)
        .expect("label control kind should exist");
    let summary = ControlLayoutDescriptor::new(label_id)
        .with_layout_role(UiLayoutRole::Panel)
        .with_container_kind(UiContainerKind::ScrollRegion)
        .with_scroll_requirement(UiScrollRequirement::ScrollOwner)
        .summary();
    let inspection = ControlInspectionDescriptor::from_control_kind(&package, kind)
        .with_control_layout_summary(&summary);

    assert_eq!(
        inspection.fact(ControlInspectionSection::Metadata, "layout.layout_roles"),
        Some("panel")
    );
    assert_eq!(
        inspection.fact(ControlInspectionSection::Metadata, "layout.container_kinds"),
        Some("scroll-region")
    );
    assert_eq!(
        inspection.fact(
            ControlInspectionSection::Metadata,
            "layout.has_runtime_layout_behavior",
        ),
        Some("false")
    );
}
