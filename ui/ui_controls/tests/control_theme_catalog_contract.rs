use ui_controls::{
    ControlInspectionDescriptor, ControlInspectionSection, ControlKindId, ControlStyleRequirement,
    ControlStyleRole, ControlThemeDescriptor, ControlThemeTokenKind, ControlThemeTokenRequirement,
    ControlThemeTokenRole, ControlVisualState, ControlVisualStateRequirement,
    LABEL_CONTROL_KIND_ID, runenwerk_control_package,
};

#[test]
fn control_theme_summary_attaches_to_catalog_inspection_read_only() {
    let package = runenwerk_control_package();
    let label_id = ControlKindId::new(LABEL_CONTROL_KIND_ID);
    let kind = package
        .control_kind(&label_id)
        .expect("label control kind should exist");
    let summary = ControlThemeDescriptor::new(label_id)
        .with_token(ControlThemeTokenRequirement::new(
            "runenwerk.theme.label.color",
            ControlThemeTokenKind::Color,
            ControlThemeTokenRole::Text,
        ))
        .with_visual_state(ControlVisualStateRequirement::new(
            ControlVisualState::Focused,
        ))
        .with_style(ControlStyleRequirement::new(
            ControlStyleRole::FocusRing,
            "runenwerk.theme.label.color",
        ))
        .summary();
    let inspection =
        ControlInspectionDescriptor::from_control_kind(&package, kind).with_theme_summary(&summary);

    assert_eq!(
        inspection.fact(ControlInspectionSection::Theme, "required_token_kinds"),
        Some("color")
    );
    assert_eq!(
        inspection.fact(ControlInspectionSection::Theme, "visual_states"),
        Some("focused")
    );
    assert_eq!(
        inspection.fact(
            ControlInspectionSection::Theme,
            "has_runtime_style_behavior"
        ),
        Some("false")
    );
}
