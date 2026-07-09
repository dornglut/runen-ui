use ui_controls::runenwerk_control_package;
use ui_input::{NormalizedInputFact, NormalizedInputSample, PointerEventKind, PointerInputFact};
use ui_math::UiPoint;
use ui_runtime::{
    OverlayLayeringScript, OverlayLayeringStep,
    base_controls_overlay_layering_fixture_from_package, replay_overlay_layering,
};

#[test]
fn runtime_overlay_replay_consumes_package_backed_descriptors() {
    let package = runenwerk_control_package();
    let fixture = base_controls_overlay_layering_fixture_from_package(&package);
    let report = replay_overlay_layering(
        &fixture,
        &OverlayLayeringScript::new("package-backed.empty"),
    );

    assert_eq!(fixture.controls.len(), package.overlay_descriptors.len());
    assert_eq!(
        report.declarations.len(),
        package
            .overlay_descriptors
            .iter()
            .map(|d| d.requirements.len())
            .sum::<usize>()
    );
    for declaration in &report.declarations {
        assert!(
            package
                .overlay_descriptors
                .iter()
                .flat_map(|descriptor| descriptor.requirements.iter())
                .any(
                    |requirement| requirement.anchor_role == declaration.anchor_id
                        && requirement.kind.as_str() == declaration.overlay_kind
                        && requirement.trigger.as_str() == declaration.trigger
                )
        );
    }
}

#[test]
fn runtime_overlay_open_intent_links_to_package_declaration() {
    let package = runenwerk_control_package();
    let fixture = base_controls_overlay_layering_fixture_from_package(&package);
    let script = OverlayLayeringScript::new("package-backed.hover-open").with_step(pointer_step(
        "step.hover-first-package-anchor",
        PointerEventKind::Move,
        40.0,
        42.0,
    ));
    let report = replay_overlay_layering(&fixture, &script);

    let intent = report
        .open_intents
        .first()
        .expect("package-backed open intent");
    assert!(report.declarations.iter().any(|declaration| {
        declaration.anchor_id == intent.anchor_id
            && declaration.overlay_kind == intent.overlay_kind
            && declaration.trigger == intent.trigger
    }));
    assert!(report.boundary_assertions.no_bypass_evidence());
}

fn pointer_step(id: &str, kind: PointerEventKind, x: f32, y: f32) -> OverlayLayeringStep {
    OverlayLayeringStep::new(
        id,
        NormalizedInputSample::new(format!("sample.{id}")).with_fact(NormalizedInputFact::Pointer(
            PointerInputFact::new(kind, UiPoint::new(x, y)),
        )),
    )
}
