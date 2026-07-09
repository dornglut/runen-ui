use std::{fs, path::PathBuf};

use ui_artifacts::{UiRuntimeArtifact, UiRuntimeArtifactDiagnostic};
use ui_compiler::UiCompiler;
use ui_controls::{ControlPackageRegistry, runenwerk_control_package};
use ui_definition::UiNodeDefinition;
use ui_headless_render::{
    DIAGNOSTIC_CONTROL_MISSING_PROPERTY_VALUE, DIAGNOSTIC_CONTROL_MISSING_VISUAL_OPERATOR,
    DIAGNOSTIC_RUNTIME_VIEW_FAILED, HeadlessPrimitiveRole, HeadlessRenderReport,
    HeadlessRenderViewport, HeadlessUiPrimitive,
};
use ui_program::UiSchemaValue;
use ui_program_lowering::form_ui_program_report_from_node_with_registry_snapshot;
use ui_runtime_view::UiRuntimeView;

#[test]
fn headless_render_refuses_failed_runtime_view_report() {
    let mut artifact = compiled_selected_button_artifact();
    artifact
        .manifest
        .push_diagnostic(UiRuntimeArtifactDiagnostic::error(
            "fixture.artifact.error",
            "fixture artifact error",
        ));
    let runtime_report = UiRuntimeView::from_artifact_report(&artifact);

    let render_report = HeadlessRenderReport::from_runtime_view_report(
        &runtime_report,
        HeadlessRenderViewport::new(320.0, 200.0),
    );

    assert!(!runtime_report.passed());
    assert!(!render_report.passed());
    assert!(render_report.frame().is_none());
    assert_has_render_diagnostic(&render_report, DIAGNOSTIC_RUNTIME_VIEW_FAILED);
}

#[test]
fn headless_render_emits_selected_button_frame_from_runtime_view() {
    let artifact = compiled_selected_button_artifact();
    let runtime_report = UiRuntimeView::from_artifact_report(&artifact);
    assert!(
        runtime_report.passed(),
        "{:?}",
        runtime_report.view.diagnostics
    );

    let render_report = HeadlessRenderReport::from_runtime_view_report(
        &runtime_report,
        HeadlessRenderViewport::new(320.0, 200.0),
    );

    assert!(render_report.passed(), "{:?}", render_report.diagnostics());
    let frame = render_report.frame().expect("headless frame should exist");
    assert_eq!(frame.frame_id, "headless.frame.root");
    assert_eq!(frame.viewport, HeadlessRenderViewport::new(320.0, 200.0));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            HeadlessUiPrimitive::Text {
                run,
                role: HeadlessPrimitiveRole::LabelText,
                ..
            } if run.text == "Selected"
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            HeadlessUiPrimitive::Rectangle {
                role: HeadlessPrimitiveRole::ButtonBackground,
                ..
            }
        )
    }));
    assert!(
        frame
            .primitives
            .iter()
            .all(|primitive| primitive.source_map_index().is_some())
    );
}

#[test]
fn headless_render_uses_property_value_for_label() {
    let mut artifact = compiled_selected_button_artifact();
    let property = &mut artifact.tables.properties.rows[0].snapshot.value;
    let UiSchemaValue::Object(values) = property else {
        panic!("button property snapshot should be an object");
    };
    values.insert("label".to_owned(), UiSchemaValue::string("Runtime Mutated"));

    let runtime_report = UiRuntimeView::from_artifact_report(&artifact);
    let render_report = HeadlessRenderReport::from_runtime_view_report(
        &runtime_report,
        HeadlessRenderViewport::new(320.0, 200.0),
    );

    assert!(render_report.passed(), "{:?}", render_report.diagnostics());
    let frame = render_report.frame().expect("headless frame should exist");
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            HeadlessUiPrimitive::Text { run, .. } if run.text == "Runtime Mutated"
        )
    }));
    assert!(!frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            HeadlessUiPrimitive::Text { run, .. } if run.text == "Selected"
        )
    }));
}

#[test]
fn headless_render_uses_defaults_for_optional_button_style_properties() {
    let mut artifact = compiled_selected_button_artifact();
    let property = &mut artifact.tables.properties.rows[0].snapshot.value;
    let UiSchemaValue::Object(values) = property else {
        panic!("button property snapshot should be an object");
    };
    for property_name in ["variant", "tone", "density", "size"] {
        values.remove(property_name);
    }

    let runtime_report = UiRuntimeView::from_artifact_report(&artifact);
    assert!(
        runtime_report.passed(),
        "{:?}",
        runtime_report.view.diagnostics
    );

    let render_report = HeadlessRenderReport::from_runtime_view_report(
        &runtime_report,
        HeadlessRenderViewport::new(320.0, 200.0),
    );

    assert!(render_report.passed(), "{:?}", render_report.diagnostics());
    let frame = render_report.frame().expect("headless frame should exist");
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            HeadlessUiPrimitive::Text { run, .. } if run.text == "Selected"
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            HeadlessUiPrimitive::Rectangle {
                role: HeadlessPrimitiveRole::ButtonBackground,
                ..
            }
        )
    }));
    for property_name in ["variant", "tone", "density", "size"] {
        assert!(
            !render_report.diagnostics().iter().any(|diagnostic| {
                diagnostic.code == DIAGNOSTIC_CONTROL_MISSING_PROPERTY_VALUE
                    && diagnostic.message.contains(property_name)
            }),
            "{:?}",
            render_report.diagnostics()
        );
    }
}

#[test]
fn headless_render_requires_visual_operator() {
    let mut artifact = compiled_selected_button_artifact();
    artifact.tables.visual.rows.clear();
    let runtime_report = UiRuntimeView::from_artifact_report(&artifact);
    assert!(
        runtime_report.passed(),
        "{:?}",
        runtime_report.view.diagnostics
    );

    let render_report = HeadlessRenderReport::from_runtime_view_report(
        &runtime_report,
        HeadlessRenderViewport::new(320.0, 200.0),
    );

    assert!(!render_report.passed());
    assert!(render_report.frame().is_none());
    assert_has_render_diagnostic(&render_report, DIAGNOSTIC_CONTROL_MISSING_VISUAL_OPERATOR);
}

#[test]
fn headless_render_output_is_deterministic() {
    let artifact = compiled_selected_button_artifact();
    let runtime_report = UiRuntimeView::from_artifact_report(&artifact);

    let first = HeadlessRenderReport::from_runtime_view_report(
        &runtime_report,
        HeadlessRenderViewport::new(320.0, 200.0),
    );
    let second = HeadlessRenderReport::from_runtime_view_report(
        &runtime_report,
        HeadlessRenderViewport::new(320.0, 200.0),
    );

    assert!(first.passed(), "{:?}", first.diagnostics());
    assert_eq!(first.frame(), second.frame());
}

fn compiled_selected_button_artifact() -> UiRuntimeArtifact {
    let node = load_node("assets/ui_gallery/button/selected.ron");
    let registry = ControlPackageRegistry::new()
        .with_package(runenwerk_control_package())
        .expect("runenwerk controls package should register");

    let formation_report = form_ui_program_report_from_node_with_registry_snapshot(
        "ui_gallery.button.selected",
        "assets.ui_gallery.button.selected",
        &node,
        &registry.snapshot(),
    );

    assert!(
        formation_report.passed(),
        "{:?}",
        formation_report.diagnostics
    );

    let report = UiCompiler.compile_report(&formation_report.program);
    assert!(
        report.passed(),
        "{:?}",
        report.artifact.manifest.diagnostics
    );

    report.artifact
}

fn assert_has_render_diagnostic(report: &HeadlessRenderReport, code: &str) {
    assert!(
        report
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == code),
        "{:?}",
        report.diagnostics()
    );
}

fn load_node(relative_repo_path: &str) -> UiNodeDefinition {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .expect("ui_headless_render should live under domain/ui/ui_headless_render")
        .to_path_buf();

    let path = repo_root.join(relative_repo_path);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {:?}: {error}", path));

    ron::from_str(&source).expect("fixture should parse as UiNodeDefinition")
}
