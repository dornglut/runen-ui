use ui_input::{NormalizedInputFact, NormalizedInputSample, PointerEventKind, PointerInputFact};
use ui_math::UiPoint;
use ui_runtime::{
    OverlayLayeringScript, OverlayLayeringStep, base_controls_overlay_layering_fixture,
    base_controls_overlay_layering_negative_scripts,
    base_controls_overlay_layering_positive_script, replay_overlay_layering,
};

#[test]
fn overlay_layering_report_proves_required_rows_and_boundaries() {
    let fixture = base_controls_overlay_layering_fixture();
    let report =
        replay_overlay_layering(&fixture, &base_controls_overlay_layering_positive_script());

    assert!(report.declarations.len() >= 8);
    assert!(
        report
            .open_intents
            .iter()
            .any(|row| row.anchor_id == "anchor.button.popup" && row.overlay_kind == "popup")
    );
    assert!(
        report
            .open_intents
            .iter()
            .any(|row| row.anchor_id == "anchor.action-prompt.menu" && row.overlay_kind == "menu")
    );
    assert!(report.open_intents.iter().any(|row| row.anchor_id == "anchor.dropdown.fixture" && row.overlay_kind == "dropdown"));
    assert!(
        report
            .open_intents
            .iter()
            .any(|row| row.anchor_id == "anchor.tooltip.hover" && row.overlay_kind == "tooltip")
    );
    assert!(
        report
            .open_intents
            .iter()
            .any(|row| row.anchor_id == "anchor.tooltip.focus" && row.overlay_kind == "tooltip")
    );
    assert!(
        report
            .open_intents
            .iter()
            .any(|row| row.anchor_id == "anchor.color-picker.picker-popup"
                && row.overlay_kind == "picker-popup")
    );
    assert!(
        report
            .open_intents
            .iter()
            .any(|row| row.anchor_id == "anchor.focus-containing.fixture"
                && row.overlay_kind == "focus-containing-overlay")
    );

    let submenu = report
        .stack_entries
        .iter()
        .find(|row| row.anchor_id == "anchor.action-prompt.submenu")
        .expect("submenu stack entry");
    assert_eq!(
        submenu.parent_request_id.as_deref(),
        Some("overlay.request.2")
    );
    assert_eq!(submenu.layer_class, "submenu");

    assert!(
        report
            .dismissal_evidence
            .iter()
            .any(|row| row.reason == "escape.topmost_dismissed")
    );
    assert!(
        report
            .dismissal_evidence
            .iter()
            .any(|row| row.reason == "outside-pointer.topmost_dismissed")
    );
    assert!(
        report
            .keyboard_navigation_evidence
            .iter()
            .any(|row| row.product_commands_executed == 0)
    );
    assert!(report.boundary_assertions.placement_recomputed_after_scroll > 0);
    assert!(
        report
            .boundary_assertions
            .placement_recomputed_after_viewport_resize
            > 0
    );
    assert!(report.boundary_assertions.anchor_invalidation_suppressed > 0);
    assert!(report.boundary_assertions.no_bypass_evidence());
}

#[test]
fn overlay_layering_inside_active_overlay_does_not_dismiss() {
    let fixture = base_controls_overlay_layering_fixture();
    let script = OverlayLayeringScript::new("overlay.inside-active-proof")
        .with_step(pointer_step("step.open-menu.action", 40.0, 94.0))
        .with_step(pointer_step("step.open-submenu.menu", 40.0, 146.0))
        .with_step(pointer_step(
            "step.pointer-inside-submenu-overlay",
            40.0,
            180.0,
        ));
    let report = replay_overlay_layering(&fixture, &script);

    assert!(
        report
            .pointer_capture_evidence
            .iter()
            .any(|row| row.reason == "inside-active-overlay" && row.outside_dismissal_blocked)
    );
    assert!(report.dismissal_evidence.is_empty());
    assert_eq!(report.boundary_assertions.overlay_stack_entries_closed, 0);
}

#[test]
fn overlay_layering_rows_have_input_and_declaration_evidence() {
    let fixture = base_controls_overlay_layering_fixture();
    let report =
        replay_overlay_layering(&fixture, &base_controls_overlay_layering_positive_script());

    for open in &report.open_intents {
        assert!(report.input_steps.iter().any(|step| step == &open.step_id));
        assert!(
            report
                .declarations
                .iter()
                .any(|decl| decl.anchor_id == open.anchor_id
                    && decl.overlay_kind == open.overlay_kind
                    && decl.trigger == open.trigger)
        );
    }
    for dismissal in &report.dismissal_evidence {
        assert!(dismissal.input_sample_id.starts_with("sample."));
        assert!(dismissal.closed);
    }
    for suppression in &report.suppression_evidence {
        assert!(suppression.input_sample_id.starts_with("sample."));
        assert!(!suppression.opened);
    }
}

#[test]
fn overlay_layering_negative_report_proves_disabled_suppression() {
    let fixture = base_controls_overlay_layering_fixture();
    let scripts = base_controls_overlay_layering_negative_scripts();
    let report = replay_overlay_layering(&fixture, &scripts[0]);

    assert!(report.open_intents.is_empty());
    assert!(
        report
            .suppression_evidence
            .iter()
            .any(|row| row.reason == "anchor.disabled")
    );
    assert_eq!(report.boundary_assertions.host_commands_executed, 0);
    assert_eq!(report.boundary_assertions.product_mutations, 0);
    assert_eq!(report.boundary_assertions.text_edit_transactions, 0);
}

fn pointer_step(id: &str, x: f32, y: f32) -> OverlayLayeringStep {
    OverlayLayeringStep::new(
        id,
        NormalizedInputSample::new(format!("sample.{id}")).with_fact(NormalizedInputFact::Pointer(
            PointerInputFact::new(PointerEventKind::Down, UiPoint::new(x, y)),
        )),
    )
}
