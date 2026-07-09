use ui_controls::{
    ControlAccessibilityDescriptionRequirement, ControlAccessibilityDescriptor,
    ControlAccessibilityDiagnostic, ControlAccessibilityDiagnosticKind,
    ControlAccessibilityLabelRequirement, ControlAccessibilityRole, ControlFocusRequirement,
    ControlKeyboardActivation, ControlKindId, ControlSemanticHint, ControlSemanticState,
    ControlValueRangeMetadata, LABEL_CONTROL_KIND_ID,
};

#[test]
fn control_accessibility_descriptor_records_roles_and_text_requirements() {
    let summary = label_accessibility_descriptor().summary();

    assert!(summary.roles.contains(&"button".to_owned()));
    assert!(summary.roles.contains(&"label".to_owned()));
    assert!(summary.roles.contains(&"slider".to_owned()));
    assert!(summary.roles.contains(&"custom".to_owned()));
    assert!(
        summary
            .label_requirements
            .contains(&"label.primary".to_owned())
    );
    assert!(
        summary
            .description_requirements
            .contains(&"label.description".to_owned())
    );
    assert!(summary.semantic_hints.contains(&"label.hint".to_owned()));
}

#[test]
fn control_accessibility_focus_and_keyboard_are_declarative() {
    let summary = label_accessibility_descriptor().summary();

    assert!(summary.focus_facts.contains(&"focusable".to_owned()));
    assert!(summary.focus_facts.contains(&"focus-order".to_owned()));
    assert!(summary.focus_facts.contains(&"focus-trap".to_owned()));
    assert!(summary.focus_facts.contains(&"focus-return".to_owned()));
    assert!(
        summary
            .keyboard_activations
            .contains(&"activate".to_owned())
    );
    assert!(summary.keyboard_activations.contains(&"cancel".to_owned()));
    assert!(summary.keyboard_activations.contains(&"commit".to_owned()));
    assert!(summary.keyboard_activations.contains(&"expand".to_owned()));
    assert!(
        summary
            .keyboard_activations
            .contains(&"collapse".to_owned())
    );
    assert!(
        summary
            .keyboard_activations
            .contains(&"increment".to_owned())
    );
    assert!(
        summary
            .keyboard_activations
            .contains(&"decrement".to_owned())
    );
    assert!(
        summary
            .keyboard_activations
            .contains(&"navigate-next".to_owned())
    );
    assert!(
        summary
            .keyboard_activations
            .contains(&"navigate-previous".to_owned())
    );
    assert!(!summary.has_runtime_focus_behavior);
}

#[test]
fn control_accessibility_semantic_states_and_range_metadata_are_declarative() {
    let summary = label_accessibility_descriptor().summary();

    assert!(summary.semantic_states.contains(&"enabled".to_owned()));
    assert!(summary.semantic_states.contains(&"disabled".to_owned()));
    assert!(summary.semantic_states.contains(&"selected".to_owned()));
    assert!(summary.semantic_states.contains(&"pressed".to_owned()));
    assert!(summary.semantic_states.contains(&"expanded".to_owned()));
    assert!(summary.semantic_states.contains(&"collapsed".to_owned()));
    assert!(summary.semantic_states.contains(&"checked".to_owned()));
    assert!(summary.semantic_states.contains(&"unchecked".to_owned()));
    assert!(summary.semantic_states.contains(&"mixed".to_owned()));
    assert!(summary.semantic_states.contains(&"busy".to_owned()));
    assert!(summary.semantic_states.contains(&"invalid".to_owned()));
    assert!(summary.semantic_states.contains(&"readonly".to_owned()));
    assert!(summary.semantic_states.contains(&"required".to_owned()));
    assert!(summary.value_ranges.contains(&"label.range".to_owned()));
}

#[test]
fn control_accessibility_diagnostics_and_inspection_facts_are_read_only() {
    let summary = label_accessibility_descriptor().summary();
    let facts = summary.inspection_facts();

    assert!(summary.diagnostics.contains(&"missing-role".to_owned()));
    assert!(summary.diagnostics.contains(&"missing-label".to_owned()));
    assert!(
        summary
            .diagnostics
            .contains(&"missing-description".to_owned())
    );
    assert!(
        summary
            .diagnostics
            .contains(&"missing-focus-order".to_owned())
    );
    assert!(summary.diagnostics.contains(&"expected-failure".to_owned()));
    assert!(
        summary
            .expected_failures
            .contains(&"label.accessibility.expected".to_owned())
    );
    assert!(
        facts
            .iter()
            .any(|fact| fact.key == "roles" && fact.value.contains("label"))
    );
    assert!(
        facts
            .iter()
            .any(|fact| { fact.key == "has_runtime_focus_behavior" && fact.value == "false" })
    );
}

fn label_accessibility_descriptor() -> ControlAccessibilityDescriptor {
    ControlAccessibilityDescriptor::new(ControlKindId::new(LABEL_CONTROL_KIND_ID))
        .with_role(ControlAccessibilityRole::Button)
        .with_role(ControlAccessibilityRole::Label)
        .with_role(ControlAccessibilityRole::Slider)
        .with_role(ControlAccessibilityRole::Custom)
        .with_label(ControlAccessibilityLabelRequirement::new("label.primary"))
        .with_description(ControlAccessibilityDescriptionRequirement::new(
            "label.description",
        ))
        .with_hint(ControlSemanticHint::new("label.hint"))
        .with_focus(
            ControlFocusRequirement::focusable()
                .with_focus_order(1)
                .with_focus_trap()
                .with_focus_return(),
        )
        .with_keyboard_activation(ControlKeyboardActivation::Activate)
        .with_keyboard_activation(ControlKeyboardActivation::Cancel)
        .with_keyboard_activation(ControlKeyboardActivation::Commit)
        .with_keyboard_activation(ControlKeyboardActivation::Expand)
        .with_keyboard_activation(ControlKeyboardActivation::Collapse)
        .with_keyboard_activation(ControlKeyboardActivation::Increment)
        .with_keyboard_activation(ControlKeyboardActivation::Decrement)
        .with_keyboard_activation(ControlKeyboardActivation::NavigateNext)
        .with_keyboard_activation(ControlKeyboardActivation::NavigatePrevious)
        .with_semantic_state(ControlSemanticState::Enabled)
        .with_semantic_state(ControlSemanticState::Disabled)
        .with_semantic_state(ControlSemanticState::Selected)
        .with_semantic_state(ControlSemanticState::Pressed)
        .with_semantic_state(ControlSemanticState::Expanded)
        .with_semantic_state(ControlSemanticState::Collapsed)
        .with_semantic_state(ControlSemanticState::Checked)
        .with_semantic_state(ControlSemanticState::Unchecked)
        .with_semantic_state(ControlSemanticState::Mixed)
        .with_semantic_state(ControlSemanticState::Busy)
        .with_semantic_state(ControlSemanticState::Invalid)
        .with_semantic_state(ControlSemanticState::Readonly)
        .with_semantic_state(ControlSemanticState::Required)
        .with_value_range(
            ControlValueRangeMetadata::new("label.range")
                .with_minimum()
                .with_maximum()
                .with_step(),
        )
        .with_diagnostic(ControlAccessibilityDiagnostic::new(
            "label.accessibility.role",
            ControlAccessibilityDiagnosticKind::MissingRole,
            "accessibility role is missing",
        ))
        .with_diagnostic(ControlAccessibilityDiagnostic::new(
            "label.accessibility.label",
            ControlAccessibilityDiagnosticKind::MissingLabel,
            "accessibility label is missing",
        ))
        .with_diagnostic(ControlAccessibilityDiagnostic::new(
            "label.accessibility.description",
            ControlAccessibilityDiagnosticKind::MissingDescription,
            "accessibility description is missing",
        ))
        .with_diagnostic(ControlAccessibilityDiagnostic::new(
            "label.accessibility.focus",
            ControlAccessibilityDiagnosticKind::MissingFocusOrder,
            "focus order is missing",
        ))
        .with_diagnostic(ControlAccessibilityDiagnostic::new(
            "label.accessibility.expected",
            ControlAccessibilityDiagnosticKind::ExpectedFailure,
            "expected accessibility fixture failure",
        ))
}
