use ui_controls::{
    BaseControlsPlugin, ControlInspectionSection, INSPECTOR_FIELD_CONTROL_KIND_ID,
    LABEL_CONTROL_KIND_ID,
};

#[test]
fn base_controls_inspection_projects_text_editing_facts() {
    let inspection = BaseControlsPlugin::new().inspection();
    let inspector = inspection
        .descriptor(INSPECTOR_FIELD_CONTROL_KIND_ID)
        .expect("inspector field inspection descriptor");

    assert_eq!(
        inspector.fact(
            ControlInspectionSection::TextEditing,
            "text_editing.supported"
        ),
        Some("true")
    );
    assert_eq!(
        inspector.fact(
            ControlInspectionSection::TextEditing,
            "text_editing.caret_supported"
        ),
        Some("true")
    );
    assert_eq!(
        inspector.fact(
            ControlInspectionSection::TextEditing,
            "text_editing.range_selection_supported"
        ),
        Some("true")
    );
    assert_eq!(
        inspector.fact(
            ControlInspectionSection::TextEditing,
            "text_editing.host_owned_mutation"
        ),
        Some("true")
    );
    assert!(
        inspector
            .fact(
                ControlInspectionSection::TextEditing,
                "text_editing.edit_intents"
            )
            .unwrap_or_default()
            .contains("replace-selection")
    );
}

#[test]
fn base_controls_inspection_marks_non_editable_controls_as_not_supported() {
    let inspection = BaseControlsPlugin::new().inspection();
    let label = inspection
        .descriptor(LABEL_CONTROL_KIND_ID)
        .expect("label inspection descriptor");

    assert_eq!(
        label.fact(
            ControlInspectionSection::TextEditing,
            "text_editing.supported"
        ),
        Some("false")
    );
}
