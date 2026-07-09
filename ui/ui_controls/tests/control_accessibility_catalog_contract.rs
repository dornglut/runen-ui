use ui_controls::{
    ControlAccessibilityDescriptor, ControlAccessibilityLabelRequirement, ControlAccessibilityRole,
    ControlFocusRequirement, ControlInspectionDescriptor, ControlInspectionSection, ControlKindId,
    LABEL_CONTROL_KIND_ID, runenwerk_control_package,
};

#[test]
fn control_accessibility_summary_attaches_to_catalog_inspection_read_only() {
    let package = runenwerk_control_package();
    let label_id = ControlKindId::new(LABEL_CONTROL_KIND_ID);
    let kind = package
        .control_kind(&label_id)
        .expect("label control kind should exist");
    let summary = ControlAccessibilityDescriptor::new(label_id)
        .with_role(ControlAccessibilityRole::Label)
        .with_label(ControlAccessibilityLabelRequirement::new("label.primary"))
        .with_focus(ControlFocusRequirement::focusable().with_focus_order(1))
        .summary();
    let inspection = ControlInspectionDescriptor::from_control_kind(&package, kind)
        .with_accessibility_summary(&summary);

    assert_eq!(
        inspection.fact(ControlInspectionSection::Accessibility, "roles"),
        Some("label")
    );
    assert_eq!(
        inspection.fact(
            ControlInspectionSection::Accessibility,
            "label_requirements"
        ),
        Some("label.primary")
    );
    assert_eq!(
        inspection.fact(
            ControlInspectionSection::Accessibility,
            "has_runtime_focus_behavior",
        ),
        Some("false")
    );
}
