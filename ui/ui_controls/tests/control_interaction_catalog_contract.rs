use ui_controls::{
    BASE_CONTROL_TARGET_KIND_IDS, BUTTON_CONTROL_KIND_ID, BaseControlsPlugin, ControlCatalogIndex,
    ControlInspectionSection,
};

#[test]
fn control_interaction_package_catalog_exposes_interaction_facts_read_only() {
    let compiled = BaseControlsPlugin::new().compile();
    let catalog = ControlCatalogIndex::from_packages([&compiled.package]);

    assert_eq!(
        compiled.package.interaction_descriptors.len(),
        BASE_CONTROL_TARGET_KIND_IDS.len()
    );

    for control_kind_id in BASE_CONTROL_TARGET_KIND_IDS {
        let entry = catalog
            .entry(control_kind_id)
            .expect("base control should be catalog-visible");

        assert!(!entry.interaction_states.is_empty());
        assert!(!entry.interaction_triggers.is_empty());
        assert!(entry.runtime_interaction_supported);
        assert!(!entry.control_owned_runtime_behavior);
        assert!(!entry.executes_host_commands);
        assert!(!entry.mutates_product_state);
    }

    let entry = catalog
        .entry(BUTTON_CONTROL_KIND_ID)
        .expect("button should be catalog-visible");
    assert!(
        entry
            .interaction_triggers
            .contains(&"pointer-press".to_owned())
    );
    assert!(
        entry
            .interaction_triggers
            .contains(&"pointer-release".to_owned())
    );
    assert!(
        entry
            .interaction_triggers
            .contains(&"pointer-activate".to_owned())
    );
    assert!(
        entry
            .interaction_outcomes
            .contains(&"activation-requested".to_owned())
    );
}

#[test]
fn control_interaction_compiled_base_control_catalog_matches_package_catalog_interaction_facts() {
    let compiled = BaseControlsPlugin::new().compile();
    let package_catalog = ControlCatalogIndex::from_packages([&compiled.package]);

    for control_kind_id in BASE_CONTROL_TARGET_KIND_IDS {
        let compiled_entry = compiled
            .catalog
            .entry(control_kind_id)
            .expect("compiled base control should be catalog-visible");
        let package_entry = package_catalog
            .entry(control_kind_id)
            .expect("package base control should be catalog-visible");

        assert_eq!(
            compiled_entry.interaction_states,
            package_entry.interaction_states
        );
        assert_eq!(
            compiled_entry.interaction_triggers,
            package_entry.interaction_triggers
        );
        assert_eq!(
            compiled_entry.interaction_outcomes,
            package_entry.interaction_outcomes
        );
        assert_eq!(
            compiled_entry.runtime_interaction_supported,
            package_entry.runtime_interaction_supported
        );
    }
}

#[test]
fn control_interaction_compiled_base_control_inspection_exposes_interaction_facts_read_only() {
    let compiled = BaseControlsPlugin::new().compile();
    let inspection = compiled
        .inspection
        .descriptor(BUTTON_CONTROL_KIND_ID)
        .expect("button inspection should exist");

    assert_eq!(
        inspection.fact(
            ControlInspectionSection::Interaction,
            "runtime_interaction_supported"
        ),
        Some("true")
    );
    assert_eq!(
        inspection.fact(
            ControlInspectionSection::Interaction,
            "control_owned_runtime_behavior"
        ),
        Some("false")
    );
    assert_eq!(
        inspection.fact(
            ControlInspectionSection::Interaction,
            "executes_host_commands"
        ),
        Some("false")
    );
    assert_eq!(
        inspection.fact(
            ControlInspectionSection::Interaction,
            "mutates_product_state"
        ),
        Some("false")
    );
    assert!(
        inspection
            .fact(ControlInspectionSection::Interaction, "outcomes")
            .is_some_and(|value| value.contains("activation-requested"))
    );
}
