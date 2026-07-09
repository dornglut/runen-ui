use ui_controls::{
    ControlGestureKind, ControlGestureRequirement, ControlInputDescriptor, ControlInputMode,
    ControlInspectionDescriptor, ControlInspectionSection, ControlKindId,
    ControlPointerRequirement, ControlTextInputRequirement, LABEL_CONTROL_KIND_ID,
    runenwerk_control_package,
};

#[test]
fn control_input_summary_attaches_to_catalog_inspection_read_only() {
    let package = runenwerk_control_package();
    let label_id = ControlKindId::new(LABEL_CONTROL_KIND_ID);
    let kind = package
        .control_kind(&label_id)
        .expect("label control kind should exist");
    let summary = ControlInputDescriptor::new(label_id)
        .with_modes([
            ControlInputMode::Pointer,
            ControlInputMode::Keyboard,
            ControlInputMode::SemanticAction,
            ControlInputMode::TextInput,
        ])
        .with_gesture(ControlGestureRequirement::new(ControlGestureKind::Drag))
        .with_pointer(ControlPointerRequirement {
            requires_capture: true,
            requires_lost_capture: true,
        })
        .with_text_input(ControlTextInputRequirement {
            requires_text_entry: true,
            requires_composition: false,
        })
        .summary();
    let inspection =
        ControlInspectionDescriptor::from_control_kind(&package, kind).with_input_summary(&summary);

    assert_eq!(
        inspection.fact(ControlInspectionSection::Input, "modes"),
        Some("pointer,keyboard,semantic-action,text-input")
    );
    assert_eq!(
        inspection.fact(ControlInspectionSection::Input, "requires_pointer_capture"),
        Some("true")
    );
    assert_eq!(
        inspection.fact(ControlInspectionSection::Input, "has_runtime_behavior"),
        Some("false")
    );
}
