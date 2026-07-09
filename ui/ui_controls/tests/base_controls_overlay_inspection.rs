use ui_controls::{BaseControlsPlugin, ControlInspectionSection, LABEL_CONTROL_KIND_ID};

#[test]
fn base_controls_inspection_projects_overlay_facts() {
    let inspection = BaseControlsPlugin::new().inspection();
    let label = inspection
        .descriptor(LABEL_CONTROL_KIND_ID)
        .expect("label inspection descriptor");
    assert_eq!(
        label.fact(ControlInspectionSection::Layering, "overlay.supported"),
        Some("true")
    );
    assert!(
        label
            .fact(ControlInspectionSection::Layering, "overlay.kinds")
            .unwrap_or_default()
            .contains("tooltip")
    );
}
