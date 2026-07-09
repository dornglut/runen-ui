use ui_controls::{BUTTON_CONTROL_KIND_ID, BaseControlsPlugin};

#[test]
fn base_controls_catalog_projects_overlay_support() {
    let catalog = BaseControlsPlugin::new().catalog();
    let button = catalog
        .entry(BUTTON_CONTROL_KIND_ID)
        .expect("button catalog entry");
    assert!(button.overlay_supported);
    assert!(button.overlay_kinds.iter().any(|kind| kind == "popup"));
    assert!(
        button
            .overlay_triggers
            .iter()
            .any(|trigger| trigger == "pointer-press")
    );
    assert!(!button.executes_host_commands);
    assert!(!button.mutates_product_state);
}
