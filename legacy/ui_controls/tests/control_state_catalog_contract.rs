use ui_controls::{
    ControlHostIntentProposal, ControlInspectionDescriptor, ControlInspectionSection,
    ControlKindId, ControlRouteCapabilityDecision, ControlStateBindingKind,
    ControlStateBindingRequirement, ControlStateBucket, ControlStateBucketRequirement,
    ControlStateDescriptor, LABEL_CONTROL_KIND_ID, runenwerk_control_package,
};

#[test]
fn control_state_summary_attaches_to_catalog_inspection_read_only() {
    let package = runenwerk_control_package();
    let label_id = ControlKindId::new(LABEL_CONTROL_KIND_ID);
    let kind = package
        .control_kind(&label_id)
        .expect("label control kind should exist");
    let summary = ControlStateDescriptor::new(label_id)
        .with_bucket(ControlStateBucketRequirement::new(
            ControlStateBucket::HostFed,
        ))
        .with_bucket(ControlStateBucketRequirement::new(
            ControlStateBucket::PackageOwned,
        ))
        .with_binding(ControlStateBindingRequirement::new(
            "label.value.write",
            ControlStateBindingKind::Write,
        ))
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
        .summary();
    let inspection =
        ControlInspectionDescriptor::from_control_kind(&package, kind).with_state_summary(&summary);

    assert!(
        inspection
            .fact(ControlInspectionSection::State, "required_buckets")
            .is_some_and(|value| value.contains("host-fed"))
    );
    assert_eq!(
        inspection.fact(ControlInspectionSection::State, "mutates_host_state"),
        Some("false")
    );
    assert!(
        inspection
            .fact(ControlInspectionSection::State, "route_ids")
            .is_some_and(|value| value.contains("runenwerk.ui.controls.label.intent"))
    );
}
