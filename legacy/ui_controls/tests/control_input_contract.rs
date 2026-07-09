use ui_controls::{
    ControlDeviceKind, ControlDeviceRequirement, ControlGestureKind, ControlGestureRequirement,
    ControlInputDescriptor, ControlInputMode, ControlInputModeSet, ControlKindId,
    ControlMountEligibility, ControlPointerRequirement, ControlSemanticActionRequirement,
    ControlTextInputRequirement, LABEL_CONTROL_KIND_ID, runenwerk_control_package,
};

#[test]
fn control_input_descriptor_records_supported_modes() {
    let descriptor = label_input_descriptor();

    assert!(descriptor.modes.contains(ControlInputMode::Pointer));
    assert!(descriptor.modes.contains(ControlInputMode::Keyboard));
    assert!(descriptor.modes.contains(ControlInputMode::SemanticAction));
    assert!(descriptor.modes.contains(ControlInputMode::TextInput));

    let summary = descriptor.summary();
    assert_eq!(
        summary.modes,
        vec!["pointer", "keyboard", "semantic-action", "text-input"]
    );
}

#[test]
fn control_input_gesture_requirements_distinguish_semantics_without_execution() {
    let descriptor = label_input_descriptor()
        .with_gesture(ControlGestureRequirement::new(ControlGestureKind::MarqueeSelect).optional())
        .with_gesture(ControlGestureRequirement::new(
            ControlGestureKind::MultiClick,
        ))
        .with_gesture(ControlGestureRequirement::new(ControlGestureKind::Cancel))
        .with_gesture(ControlGestureRequirement::new(ControlGestureKind::Commit))
        .with_gesture(ControlGestureRequirement::new(ControlGestureKind::Rollback));
    let summary = descriptor.summary();

    assert!(summary.required_gestures.contains(&"drag".to_owned()));
    assert!(
        summary
            .required_gestures
            .contains(&"multi-click".to_owned())
    );
    assert!(summary.required_gestures.contains(&"cancel".to_owned()));
    assert!(summary.required_gestures.contains(&"commit".to_owned()));
    assert!(summary.required_gestures.contains(&"rollback".to_owned()));
    assert!(
        summary
            .optional_gestures
            .contains(&"marquee-select".to_owned())
    );
    assert!(!summary.has_runtime_behavior);
}

#[test]
fn control_input_device_requirements_distinguish_normalized_facts() {
    let descriptor = label_input_descriptor()
        .with_device(ControlDeviceRequirement::new(ControlDeviceKind::Pressure))
        .with_device(ControlDeviceRequirement::new(ControlDeviceKind::Tilt))
        .with_device(ControlDeviceRequirement::new(ControlDeviceKind::Twist))
        .with_device(ControlDeviceRequirement::new(
            ControlDeviceKind::TangentialPressure,
        ))
        .with_device(ControlDeviceRequirement::new(ControlDeviceKind::Eraser).optional())
        .with_device(ControlDeviceRequirement::new(ControlDeviceKind::BarrelButton).optional())
        .with_device(ControlDeviceRequirement::new(
            ControlDeviceKind::CoalescedSamples,
        ))
        .with_device(ControlDeviceRequirement::new(
            ControlDeviceKind::PredictedSamples,
        ));
    let summary = descriptor.summary();

    assert!(
        summary
            .required_device_facts
            .contains(&"pressure".to_owned())
    );
    assert!(summary.required_device_facts.contains(&"tilt".to_owned()));
    assert!(summary.required_device_facts.contains(&"twist".to_owned()));
    assert!(
        summary
            .required_device_facts
            .contains(&"tangential-pressure".to_owned())
    );
    assert!(
        summary
            .required_device_facts
            .contains(&"coalesced-samples".to_owned())
    );
    assert!(
        summary
            .required_device_facts
            .contains(&"predicted-samples".to_owned())
    );
    assert!(summary.optional_device_facts.contains(&"eraser".to_owned()));
    assert!(
        summary
            .optional_device_facts
            .contains(&"barrel-button".to_owned())
    );
}

#[test]
fn control_input_summary_exposes_read_only_inspection_facts() {
    let summary = label_input_descriptor().summary();
    let facts = summary.inspection_facts();

    assert!(
        facts
            .iter()
            .any(|fact| fact.key == "modes" && fact.value.contains("pointer"))
    );
    assert!(
        facts
            .iter()
            .any(|fact| fact.key == "required_gestures" && fact.value.contains("drag"))
    );
    assert!(
        facts
            .iter()
            .any(|fact| { fact.key == "requires_pointer_capture" && fact.value == "true" })
    );
    assert!(
        facts
            .iter()
            .any(|fact| fact.key == "requires_text_entry" && fact.value == "true")
    );
    assert!(
        facts
            .iter()
            .any(|fact| fact.key == "has_runtime_behavior" && fact.value == "false")
    );
}

#[test]
fn control_input_declaration_does_not_upgrade_runtime_mount_eligibility() {
    let package = runenwerk_control_package();
    let label_id = ControlKindId::new(LABEL_CONTROL_KIND_ID);
    let kind = package
        .control_kind(&label_id)
        .expect("label control kind should exist");
    let summary = label_input_descriptor().summary();

    assert!(!summary.has_runtime_behavior);
    assert!(matches!(
        &kind.mount_eligibility,
        ControlMountEligibility::NotEligible { .. }
    ));
}

fn label_input_descriptor() -> ControlInputDescriptor {
    ControlInputDescriptor::new(ControlKindId::new(LABEL_CONTROL_KIND_ID))
        .with_modes(
            ControlInputModeSet::new([
                ControlInputMode::TextInput,
                ControlInputMode::Keyboard,
                ControlInputMode::SemanticAction,
                ControlInputMode::Pointer,
            ])
            .modes,
        )
        .with_gesture(ControlGestureRequirement::new(ControlGestureKind::Hover))
        .with_gesture(ControlGestureRequirement::new(ControlGestureKind::Press))
        .with_gesture(ControlGestureRequirement::new(ControlGestureKind::Drag))
        .with_gesture(ControlGestureRequirement::new(
            ControlGestureKind::PointerCapture,
        ))
        .with_gesture(ControlGestureRequirement::new(
            ControlGestureKind::LostCapture,
        ))
        .with_pointer(ControlPointerRequirement {
            requires_capture: true,
            requires_lost_capture: true,
        })
        .with_text_input(ControlTextInputRequirement {
            requires_text_entry: true,
            requires_composition: false,
        })
        .with_semantic_action(ControlSemanticActionRequirement::new("activate"))
}
