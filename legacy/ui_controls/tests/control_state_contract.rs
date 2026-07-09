use ui_controls::{
    ControlEditLifecycle, ControlHostIntentProposal, ControlKindId, ControlRouteCapabilityDecision,
    ControlStateBindingKind, ControlStateBindingRequirement, ControlStateBucket,
    ControlStateBucketRequirement, ControlStateDescriptor, ControlValidationState,
    LABEL_CONTROL_KIND_ID,
};

#[test]
fn control_state_descriptor_records_state_buckets() {
    let summary = label_state_descriptor().summary();

    assert!(summary.required_buckets.contains(&"transient".to_owned()));
    assert!(summary.required_buckets.contains(&"preview".to_owned()));
    assert!(summary.required_buckets.contains(&"committed".to_owned()));
    assert!(summary.required_buckets.contains(&"host-fed".to_owned()));
    assert!(
        summary
            .required_buckets
            .contains(&"package-owned".to_owned())
    );
}

#[test]
fn control_state_bindings_distinguish_declared_shapes() {
    let summary = label_state_descriptor()
        .with_binding(ControlStateBindingRequirement::new(
            "label.items",
            ControlStateBindingKind::Collection,
        ))
        .with_binding(ControlStateBindingRequirement::new(
            "label.option",
            ControlStateBindingKind::Option,
        ))
        .with_binding(ControlStateBindingRequirement::new(
            "label.selection",
            ControlStateBindingKind::Selection,
        ))
        .summary();

    assert!(summary.binding_kinds.contains(&"read".to_owned()));
    assert!(summary.binding_kinds.contains(&"write".to_owned()));
    assert!(summary.binding_kinds.contains(&"collection".to_owned()));
    assert!(summary.binding_kinds.contains(&"option".to_owned()));
    assert!(summary.binding_kinds.contains(&"selection".to_owned()));
}

#[test]
fn control_state_lifecycle_validation_and_intents_are_declarative() {
    let summary = label_state_descriptor().summary();

    assert!(summary.edit_lifecycle.contains(&"live-edit".to_owned()));
    assert!(summary.edit_lifecycle.contains(&"commit-edit".to_owned()));
    assert!(summary.edit_lifecycle.contains(&"cancel-edit".to_owned()));
    assert!(summary.edit_lifecycle.contains(&"rollback-edit".to_owned()));
    assert!(summary.validation_states.contains(&"clean".to_owned()));
    assert!(summary.validation_states.contains(&"dirty".to_owned()));
    assert!(summary.validation_states.contains(&"read-only".to_owned()));
    assert!(summary.validation_states.contains(&"invalid".to_owned()));
    assert!(summary.validation_states.contains(&"warning".to_owned()));
    assert!(
        summary
            .validation_states
            .contains(&"pending-validation".to_owned())
    );
    assert!(
        summary
            .host_intents
            .contains(&"label.commit.preview".to_owned())
    );
    assert!(
        summary
            .route_ids
            .contains(&"runenwerk.ui.controls.label.intent".to_owned())
    );
    assert!(
        summary
            .required_capabilities
            .contains(&"runenwerk.ui.controls.read".to_owned())
    );
    assert!(summary.host_decisions.contains(&"blocked".to_owned()));
    assert!(!summary.mutates_host_state);
}

#[test]
fn control_state_summary_exposes_read_only_inspection_facts() {
    let facts = label_state_descriptor().summary().inspection_facts();

    assert!(
        facts
            .iter()
            .any(|fact| fact.key == "required_buckets" && fact.value.contains("host-fed"))
    );
    assert!(
        facts
            .iter()
            .any(|fact| fact.key == "binding_kinds" && fact.value.contains("write"))
    );
    assert!(
        facts
            .iter()
            .any(|fact| fact.key == "host_intents" && fact.value.contains("label.commit.preview"))
    );
    assert!(
        facts
            .iter()
            .any(|fact| fact.key == "mutates_host_state" && fact.value == "false")
    );
}

fn label_state_descriptor() -> ControlStateDescriptor {
    ControlStateDescriptor::new(ControlKindId::new(LABEL_CONTROL_KIND_ID))
        .with_bucket(ControlStateBucketRequirement::new(
            ControlStateBucket::Transient,
        ))
        .with_bucket(ControlStateBucketRequirement::new(
            ControlStateBucket::Preview,
        ))
        .with_bucket(ControlStateBucketRequirement::new(
            ControlStateBucket::Committed,
        ))
        .with_bucket(ControlStateBucketRequirement::new(
            ControlStateBucket::HostFed,
        ))
        .with_bucket(ControlStateBucketRequirement::new(
            ControlStateBucket::PackageOwned,
        ))
        .with_binding(ControlStateBindingRequirement::new(
            "label.value.read",
            ControlStateBindingKind::Read,
        ))
        .with_binding(ControlStateBindingRequirement::new(
            "label.value.write",
            ControlStateBindingKind::Write,
        ))
        .with_edit_lifecycle(ControlEditLifecycle::LiveEdit)
        .with_edit_lifecycle(ControlEditLifecycle::CommitEdit)
        .with_edit_lifecycle(ControlEditLifecycle::CancelEdit)
        .with_edit_lifecycle(ControlEditLifecycle::RollbackEdit)
        .with_validation_state(ControlValidationState::Clean)
        .with_validation_state(ControlValidationState::Dirty)
        .with_validation_state(ControlValidationState::ReadOnly)
        .with_validation_state(ControlValidationState::Invalid)
        .with_validation_state(ControlValidationState::Warning)
        .with_validation_state(ControlValidationState::PendingValidation)
        .with_host_intent(
            ControlHostIntentProposal::new(
                "label.commit.preview",
                "runenwerk.ui.controls.label.intent",
                1,
            )
            .with_capability("runenwerk.ui.controls.read"),
        )
        .with_route_decision(ControlRouteCapabilityDecision::blocked(
            "runenwerk.ui.controls.label.intent",
            "route not available in this host",
        ))
}
