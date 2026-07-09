use ui_controls::{
    BUTTON_CONTROL_KIND_ID, BaseControlsPlugin, ControlInteractionDescriptor,
    ControlInteractionOutcome, ControlInteractionRequirement, ControlInteractionState,
    ControlInteractionTrigger, ControlKindId, ControlMountEligibility,
    INSPECTOR_FIELD_CONTROL_KIND_ID, runenwerk_control_package,
};

#[test]
fn control_interaction_descriptor_records_states_triggers_and_outcomes() {
    let descriptor = ControlInteractionDescriptor::new(ControlKindId::new(BUTTON_CONTROL_KIND_ID))
        .with_requirement(ControlInteractionRequirement::new(
            ControlInteractionTrigger::PointerPress,
        ))
        .with_requirement(
            ControlInteractionRequirement::new(ControlInteractionTrigger::PointerActivate)
                .with_outcome(ControlInteractionOutcome::ActivationRequested),
        )
        .with_requirement(
            ControlInteractionRequirement::new(ControlInteractionTrigger::KeyboardActivate)
                .requiring_focus()
                .with_outcome(ControlInteractionOutcome::ActivationRequested),
        );

    let summary = descriptor.summary();

    assert!(descriptor.states.contains(ControlInteractionState::Enabled));
    assert!(summary.triggers.contains(&"pointer-press".to_owned()));
    assert!(summary.triggers.contains(&"pointer-activate".to_owned()));
    assert!(summary.triggers.contains(&"keyboard-activate".to_owned()));
    assert!(
        summary
            .outcomes
            .contains(&"activation-requested".to_owned())
    );
    assert!(summary.requires_focus);
    assert!(summary.runtime_interaction_supported);
    assert!(!summary.control_owned_runtime_behavior);
    assert!(!summary.executes_host_commands);
    assert!(!summary.mutates_product_state);
}

#[test]
fn base_control_interaction_lowering_marks_text_intent_as_probe_only() {
    let compiled = BaseControlsPlugin::new().compile();
    let inspector = compiled
        .controls
        .iter()
        .find(|control| {
            control.module.kind.control_kind_id.as_str() == INSPECTOR_FIELD_CONTROL_KIND_ID
        })
        .expect("inspector field should be compiled");
    let summary = inspector.interaction.summary();

    assert!(summary.text_intent_probe);
    assert!(summary.outcomes.contains(&"text-intent-seen".to_owned()));
    assert!(summary.runtime_interaction_supported);
    assert!(!summary.control_owned_runtime_behavior);
    assert!(!summary.executes_host_commands);
    assert!(!summary.mutates_product_state);
}

#[test]
fn interaction_declarations_do_not_upgrade_runtime_mount_eligibility() {
    let package = runenwerk_control_package();
    assert_eq!(package.interaction_descriptors.len(), 9);

    for kind in &package.control_kinds {
        assert!(
            package
                .interaction_descriptor(&kind.control_kind_id)
                .is_some()
        );
        assert!(!kind.compatibility.supports_runtime_mount);
        assert!(matches!(
            &kind.mount_eligibility,
            ControlMountEligibility::NotEligible { .. }
        ));
    }
}
