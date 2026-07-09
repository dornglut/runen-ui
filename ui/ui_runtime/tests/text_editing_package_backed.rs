use ui_controls::runenwerk_control_package;
use ui_input::{NormalizedInputFact, NormalizedInputSample, TextEditFact};
use ui_runtime::{
    TextEditingReplayScript, TextEditingReplayStep,
    base_controls_text_editing_fixture_from_package, replay_text_editing,
};

#[test]
fn runtime_text_editing_replay_consumes_package_backed_descriptors() {
    let package = runenwerk_control_package();
    let fixture = base_controls_text_editing_fixture_from_package(&package);
    let report = replay_text_editing(
        &fixture,
        &TextEditingReplayScript::new("package-backed.empty"),
    );

    assert_eq!(
        fixture.controls.len(),
        package.editable_text_descriptors.len()
    );
    assert_eq!(
        report.descriptor_evidence.len(),
        package.editable_text_descriptors.len()
    );
    for evidence in &report.descriptor_evidence {
        assert!(package.editable_text_descriptors.iter().any(|descriptor| {
            descriptor.control_kind_id.as_str() == evidence.control_kind_id
                && descriptor.mode.as_str() == evidence.mode
        }));
    }
}

#[test]
fn runtime_text_editing_intent_links_to_package_declaration() {
    let package = runenwerk_control_package();
    let fixture = base_controls_text_editing_fixture_from_package(&package);
    let target_id = fixture.controls[0].target_id.clone();
    let script = TextEditingReplayScript::new("package-backed.insert").with_step(
        TextEditingReplayStep::new(
            "step.insert.package-backed",
            NormalizedInputSample::new("sample.insert.package-backed").with_fact(
                NormalizedInputFact::TextEdit(
                    TextEditFact::insert_text("package").with_target(target_id.clone()),
                ),
            ),
        ),
    );
    let report = replay_text_editing(&fixture, &script);
    let intent = report
        .accepted_edit_intents
        .first()
        .expect("accepted package-backed insert intent");

    assert_eq!(intent.target_id, target_id);
    assert!(report.descriptor_evidence.iter().any(|descriptor| {
        descriptor.target_id == intent.target_id
            && descriptor
                .supported_intents
                .iter()
                .any(|value| value == &intent.intent)
    }));
    assert!(report.boundary_assertions.no_bypass_evidence());
}
