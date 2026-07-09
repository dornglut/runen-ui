use ui_controls::{
    ControlEditableTextDescriptor, ControlEditableTextIntent, ControlEditableTextSelectionPolicy,
    ControlKindId, ControlPackageValidationReason, INSPECTOR_FIELD_CONTROL_KIND_ID,
    runenwerk_control_package,
};

#[test]
fn base_controls_package_exposes_package_backed_editable_text_for_inspector_field() {
    let package = runenwerk_control_package();
    assert_eq!(package.editable_text_descriptors.len(), 1);
    assert!(package.validate_contract().is_valid());

    let descriptor = package
        .editable_text_descriptor(&ControlKindId::new(INSPECTOR_FIELD_CONTROL_KIND_ID))
        .expect("inspector field editable text descriptor");
    assert_eq!(descriptor.mode.as_str(), "inspector-field");
    assert_eq!(
        descriptor.selection_policy,
        ControlEditableTextSelectionPolicy::RangeSelection
    );
    assert!(
        descriptor
            .supported_intents
            .contains(&ControlEditableTextIntent::InsertText)
    );
    assert!(
        descriptor
            .supported_intents
            .contains(&ControlEditableTextIntent::ReplaceSelection)
    );
    assert!(descriptor.host_owned_mutation);
    assert!(descriptor.proof_required);
}

#[test]
fn base_controls_package_rejects_duplicate_editable_text_descriptor() {
    let mut package = runenwerk_control_package();
    package
        .editable_text_descriptors
        .push(package.editable_text_descriptors[0].clone());
    assert_has_reason(
        package,
        ControlPackageValidationReason::DuplicateEditableTextDescriptor,
    );
}

#[test]
fn base_controls_package_rejects_unresolved_editable_text_descriptor() {
    let mut package = runenwerk_control_package();
    package.editable_text_descriptors[0].control_kind_id =
        ControlKindId::new("runenwerk.ui.missing");
    assert_has_reason(
        package,
        ControlPackageValidationReason::UnresolvedEditableTextDescriptor,
    );
}

#[test]
fn base_controls_package_rejects_invalid_editable_text_descriptor() {
    let mut package = runenwerk_control_package();
    package.editable_text_descriptors[0]
        .supported_intents
        .clear();
    assert_has_reason(
        package,
        ControlPackageValidationReason::InvalidEditableTextDescriptor,
    );
}

#[test]
fn package_rejects_editable_text_without_matching_interaction_descriptor() {
    let mut package = runenwerk_control_package();
    package.interaction_descriptors.clear();
    assert_has_reason(
        package,
        ControlPackageValidationReason::InvalidEditableTextDescriptor,
    );
}

#[test]
fn package_rejects_range_intent_without_range_selection() {
    let mut package = runenwerk_control_package();
    package.editable_text_descriptors[0] = ControlEditableTextDescriptor::single_line(
        ControlKindId::new(INSPECTOR_FIELD_CONTROL_KIND_ID),
    )
    .with_intent(ControlEditableTextIntent::ReplaceSelection);
    assert_has_reason(
        package,
        ControlPackageValidationReason::InvalidEditableTextDescriptor,
    );
}

#[test]
fn package_rejects_read_only_mutation_intent() {
    let mut package = runenwerk_control_package();
    package.editable_text_descriptors[0] = ControlEditableTextDescriptor::read_only_selectable(
        ControlKindId::new(INSPECTOR_FIELD_CONTROL_KIND_ID),
    )
    .with_intent(ControlEditableTextIntent::InsertText);
    assert_has_reason(
        package,
        ControlPackageValidationReason::InvalidEditableTextDescriptor,
    );
}

#[test]
fn package_rejects_non_host_owned_editable_text_mutation() {
    let mut package = runenwerk_control_package();
    package.editable_text_descriptors[0].host_owned_mutation = false;
    assert_has_reason(
        package,
        ControlPackageValidationReason::InvalidEditableTextDescriptor,
    );
}

fn assert_has_reason(
    package: ui_controls::ControlPackageDescriptor,
    reason: ControlPackageValidationReason,
) {
    let report = package.validate_contract();
    assert!(!report.is_valid(), "package unexpectedly valid");
    assert!(
        report.has_reason(reason),
        "expected reason {:?}, got {:?}",
        reason,
        report.diagnostics
    );
}
