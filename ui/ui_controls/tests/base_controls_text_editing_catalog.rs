use ui_controls::{BaseControlsPlugin, INSPECTOR_FIELD_CONTROL_KIND_ID, LABEL_CONTROL_KIND_ID};

#[test]
fn base_controls_catalog_projects_editable_text_support() {
    let catalog = BaseControlsPlugin::new().catalog();
    let inspector = catalog
        .entry(INSPECTOR_FIELD_CONTROL_KIND_ID)
        .expect("inspector field catalog entry");

    assert!(inspector.editable_text_supported);
    assert!(inspector.editable_text_caret_supported);
    assert!(inspector.editable_text_range_selection_supported);
    assert!(inspector.editable_text_composition_supported);
    assert!(inspector.editable_text_host_owned_mutation);
    assert!(
        inspector
            .editable_text_modes
            .iter()
            .any(|mode| mode == "inspector-field")
    );
    assert!(
        inspector
            .editable_text_intents
            .iter()
            .any(|intent| intent == "insert-text")
    );
    assert!(!inspector.executes_host_commands);
    assert!(!inspector.mutates_product_state);
}

#[test]
fn base_controls_catalog_does_not_mark_non_editable_controls_as_editable() {
    let catalog = BaseControlsPlugin::new().catalog();
    let label = catalog
        .entry(LABEL_CONTROL_KIND_ID)
        .expect("label catalog entry");

    assert!(!label.editable_text_supported);
    assert!(label.editable_text_modes.is_empty());
    assert!(label.editable_text_intents.is_empty());
}
