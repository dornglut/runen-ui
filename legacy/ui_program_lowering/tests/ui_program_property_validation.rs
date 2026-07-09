use std::{collections::BTreeMap, fs, path::PathBuf};

use ui_controls::{ControlPackageRegistry, runenwerk_control_package};
use ui_definition::{
    AuthoredControlAccessibilityDefinition, AuthoredControlKindId, AuthoredControlValue,
    AuthoredRouteId, UiNodeDefinition,
};
use ui_program::UiSchemaValue;
use ui_program_lowering::{
    UiProgramFormationReport, form_ui_program_report_from_node_with_registry_snapshot,
};

#[test]
fn valid_basic_button_properties_are_validated_and_carried() {
    let node = load_node("assets/ui_gallery/button/basic.ron");
    let report = form_report(
        "ui_gallery.button.basic",
        "assets.ui_gallery.button.basic",
        &node,
    );

    assert!(report.passed(), "{:?}", report.diagnostics);

    let program = &report.program;
    assert_eq!(program.graphs.properties.rows.len(), 1);

    let snapshot = &program.graphs.properties.rows[0];
    assert_eq!(snapshot.snapshot_id.as_str(), "properties.button_basic");
    assert_eq!(snapshot.owner_control.as_str(), "control.button_basic");
    assert_eq!(
        snapshot.schema.id.as_str(),
        "runenwerk.ui.controls.button.properties"
    );
    assert_eq!(snapshot.schema.version.value(), 1);
    assert_eq!(string_field(snapshot.value.get("label")), Some("Press me"));
    assert_eq!(string_field(snapshot.value.get("variant")), Some("primary"));
    assert_eq!(string_field(snapshot.value.get("tone")), Some("accent"));
    assert_eq!(string_field(snapshot.value.get("density")), Some("normal"));
    assert_eq!(string_field(snapshot.value.get("size")), Some("md"));
    assert!(snapshot.source_map.is_some());
}

#[test]
fn valid_selected_button_properties_are_validated_and_carried() {
    let node = load_node("assets/ui_gallery/button/selected.ron");
    let report = form_report(
        "ui_gallery.button.selected",
        "assets.ui_gallery.button.selected",
        &node,
    );

    assert!(report.passed(), "{:?}", report.diagnostics);

    let program = &report.program;
    assert_eq!(program.graphs.properties.rows.len(), 1);
    assert_eq!(program.graphs.state.requirements.len(), 1);
    assert_eq!(program.graphs.binding.bindings.len(), 1);

    let snapshot = &program.graphs.properties.rows[0];
    assert_eq!(string_field(snapshot.value.get("label")), Some("Selected"));
    assert_eq!(
        string_field(snapshot.value.get("variant")),
        Some("secondary")
    );
    assert!(snapshot.value.get("selected").is_none());
}

#[test]
fn missing_button_label_fails_formation() {
    let mut node = load_node("assets/ui_gallery/button/basic.ron");
    remove_property(&mut node, "label");

    let report = form_report(
        "ui_gallery.button.missing_label",
        "assets.ui_gallery.button.basic",
        &node,
    );

    assert_property_failure(
        &report,
        "ui.program.control.properties.required_field_missing",
        "label",
        "button_basic",
    );
}

#[test]
fn unknown_button_property_fails_formation() {
    let mut node = load_node("assets/ui_gallery/button/basic.ron");
    insert_property(
        &mut node,
        "background_color",
        AuthoredControlValue::String("#ff00ff".to_owned()),
    );

    let report = form_report(
        "ui_gallery.button.unknown_property",
        "assets.ui_gallery.button.basic",
        &node,
    );

    assert_property_failure(
        &report,
        "ui.program.control.properties.unknown_field",
        "background_color",
        "button_basic",
    );
}

#[test]
fn invalid_button_variant_fails_formation() {
    assert_invalid_enum_property("variant", "giant-primary");
}

#[test]
fn invalid_button_tone_fails_formation() {
    assert_invalid_enum_property("tone", "loud");
}

#[test]
fn invalid_button_density_fails_formation() {
    assert_invalid_enum_property("density", "tiny");
}

#[test]
fn invalid_button_size_fails_formation() {
    assert_invalid_enum_property("size", "xxl");
}

#[test]
fn invalid_parent_properties_do_not_lower_children() {
    let mut parent_properties = BTreeMap::new();
    parent_properties.insert(
        "variant".to_owned(),
        AuthoredControlValue::String("tiny".to_owned()),
    );

    let mut child_properties = BTreeMap::new();
    child_properties.insert(
        "label".to_owned(),
        AuthoredControlValue::String("Child".to_owned()),
    );

    let node = UiNodeDefinition::Control {
        id: "invalid_parent".into(),
        kind: AuthoredControlKindId::new("runenwerk.ui.controls.button"),
        properties: parent_properties,
        bindings: BTreeMap::new(),
        route: None,
        accessibility: Some(AuthoredControlAccessibilityDefinition {
            role: "button".to_owned(),
            label: Some("Invalid parent".to_owned()),
        }),
        children: vec![UiNodeDefinition::Control {
            id: "valid_child".into(),
            kind: AuthoredControlKindId::new("runenwerk.ui.controls.button"),
            properties: child_properties,
            bindings: BTreeMap::new(),
            route: Some(AuthoredRouteId::new("ui_gallery.button.child.activate")),
            accessibility: Some(AuthoredControlAccessibilityDefinition {
                role: "button".to_owned(),
                label: Some("Valid child".to_owned()),
            }),
            children: Vec::new(),
        }],
    };

    let report = form_report(
        "ui_gallery.button.invalid_parent",
        "assets.ui_gallery.button.invalid_parent",
        &node,
    );

    assert_property_failure(
        &report,
        "ui.program.control.properties.required_field_missing",
        "label",
        "invalid_parent",
    );
    assert_no_graph_rows(&report);
}

#[test]
fn property_diagnostics_are_source_mapped() {
    let mut node = load_node("assets/ui_gallery/button/basic.ron");
    insert_property(
        &mut node,
        "density",
        AuthoredControlValue::String("tiny".to_owned()),
    );

    let report = form_report(
        "ui_gallery.button.invalid_density",
        "assets.ui_gallery.button.basic",
        &node,
    );

    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "ui.program.control.properties.string_value_not_allowed"
            && diagnostic.source_map.is_some()
    }));
}

#[test]
fn known_valid_button_still_lowers_all_existing_graph_families() {
    let node = load_node("assets/ui_gallery/button/selected.ron");
    let report = form_report(
        "ui_gallery.button.selected",
        "assets.ui_gallery.button.selected",
        &node,
    );

    assert!(report.passed(), "{:?}", report.diagnostics);
    assert_eq!(report.program.graphs.control.nodes.len(), 1);
    assert_eq!(report.program.graphs.properties.rows.len(), 1);
    assert_eq!(report.program.graphs.layout.constraints.len(), 1);
    assert_eq!(report.program.graphs.style.rules.len(), 1);
    assert_eq!(report.program.graphs.state.requirements.len(), 1);
    assert_eq!(report.program.graphs.interaction.handlers.len(), 1);
    assert_eq!(report.program.graphs.binding.bindings.len(), 1);
    assert_eq!(report.program.graphs.visual.operators.len(), 1);
    assert_eq!(report.program.graphs.accessibility.nodes.len(), 1);
    assert_eq!(report.program.graphs.inspection.entries.len(), 1);
}

fn assert_invalid_enum_property(field: &str, value: &str) {
    let mut node = load_node("assets/ui_gallery/button/basic.ron");
    insert_property(
        &mut node,
        field,
        AuthoredControlValue::String(value.to_owned()),
    );

    let report = form_report(
        format!("ui_gallery.button.invalid_{field}"),
        "assets.ui_gallery.button.basic",
        &node,
    );

    assert_property_failure(
        &report,
        "ui.program.control.properties.string_value_not_allowed",
        field,
        "button_basic",
    );
}

fn assert_property_failure(
    report: &UiProgramFormationReport,
    expected_code: &str,
    expected_field_path: &str,
    expected_control_id: &str,
) {
    assert!(!report.passed());
    assert!(report.catalog_report.passed());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == expected_code
            && diagnostic.message.contains(expected_field_path)
            && diagnostic
                .message
                .contains("runenwerk.ui.controls.button.properties@1")
            && diagnostic.message.contains(expected_control_id)
            && diagnostic.message.contains("runenwerk.ui.controls.button")
            && diagnostic.source_map.is_some()
    }));
    assert_eq!(report.diagnostics, report.program.diagnostics);
    assert_no_graph_rows(report);
}

fn assert_no_graph_rows(report: &UiProgramFormationReport) {
    let program = &report.program;
    assert_eq!(program.graphs.control.nodes.len(), 0);
    assert_eq!(program.graphs.properties.rows.len(), 0);
    assert_eq!(program.graphs.layout.constraints.len(), 0);
    assert_eq!(program.graphs.style.rules.len(), 0);
    assert_eq!(program.graphs.state.requirements.len(), 0);
    assert_eq!(program.graphs.interaction.handlers.len(), 0);
    assert_eq!(program.graphs.binding.bindings.len(), 0);
    assert_eq!(program.graphs.visual.operators.len(), 0);
    assert_eq!(program.graphs.accessibility.nodes.len(), 0);
    assert_eq!(program.graphs.inspection.entries.len(), 0);
}

fn remove_property(node: &mut UiNodeDefinition, field: &str) {
    let UiNodeDefinition::Control { properties, .. } = node else {
        panic!("expected generic Control node");
    };
    properties.remove(field);
}

fn insert_property(node: &mut UiNodeDefinition, field: &str, value: AuthoredControlValue) {
    let UiNodeDefinition::Control { properties, .. } = node else {
        panic!("expected generic Control node");
    };
    properties.insert(field.to_owned(), value);
}

fn string_field(value: Option<&UiSchemaValue>) -> Option<&str> {
    value.and_then(UiSchemaValue::as_str)
}

fn form_report(
    program_id: impl Into<String>,
    source_id: &str,
    node: &UiNodeDefinition,
) -> UiProgramFormationReport {
    let registry = ControlPackageRegistry::new()
        .with_package(runenwerk_control_package())
        .expect("runenwerk controls package should register");

    form_ui_program_report_from_node_with_registry_snapshot(
        program_id,
        source_id,
        node,
        &registry.snapshot(),
    )
}

fn load_node(relative_repo_path: &str) -> UiNodeDefinition {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .expect("ui_program_lowering should live under domain/ui/ui_program_lowering")
        .to_path_buf();

    let path = repo_root.join(relative_repo_path);

    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {:?}: {error}", path));

    ron::from_str(&source).expect("fixture should parse as UiNodeDefinition")
}
