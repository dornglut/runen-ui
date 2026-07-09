use std::collections::BTreeSet;

use ui_controls::{
    BASE_CONTROL_TARGET_KIND_IDS, BaseControlsPlugin, ControlFieldGroupRole,
    ControlInspectionSection, ControlMountEligibility, ControlPreset, LIST_VIEW_CONTROL_KIND_ID,
    TABLE_VIEW_CONTROL_KIND_ID, TREE_VIEW_CONTROL_KIND_ID, runenwerk_control_package,
};

#[test]
fn base_control_plugin_contributes_target_controls_to_ui_controls_extension() {
    let controls = BaseControlsPlugin::new().extension();
    let actual = controls
        .contributions()
        .iter()
        .map(|contribution| contribution.control_kind_id().as_str().to_owned())
        .collect::<BTreeSet<_>>();
    let expected = BASE_CONTROL_TARGET_KIND_IDS
        .iter()
        .map(|kind_id| (*kind_id).to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(controls.len(), BASE_CONTROL_TARGET_KIND_IDS.len());
    assert_eq!(actual, expected);
    assert!(
        controls
            .contributions()
            .iter()
            .all(|contribution| !contribution.def().field_groups().is_empty())
    );
    assert!(
        controls
            .contributions()
            .iter()
            .all(|contribution| !contribution.def().theme_groups().is_empty())
    );
}

#[test]
fn base_control_definitions_declare_schema_fields_and_presets() {
    let controls = BaseControlsPlugin::new().extension();
    let presets = controls
        .contributions()
        .iter()
        .map(|contribution| contribution.preset())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        presets,
        [
            ControlPreset::Label,
            ControlPreset::Button,
            ControlPreset::InspectorField,
            ControlPreset::ColorPicker,
            ControlPreset::ActionPrompt,
            ControlPreset::ListView,
            ControlPreset::TreeView,
            ControlPreset::TableView,
            ControlPreset::Surface2D,
        ]
        .into_iter()
        .collect()
    );

    for contribution in controls.contributions() {
        let roles = contribution
            .def()
            .field_groups()
            .iter()
            .map(|group| group.role)
            .collect::<BTreeSet<_>>();
        assert!(roles.contains(&ControlFieldGroupRole::Properties));
        assert!(roles.contains(&ControlFieldGroupRole::State));
        assert!(roles.contains(&ControlFieldGroupRole::EventPayload));
    }
}

#[test]
fn base_control_compiler_lowers_to_valid_package_catalog_and_inspection() {
    let compiled = BaseControlsPlugin::new().compile();
    let report = compiled.package.validate_contract();

    assert!(report.is_valid(), "{:?}", report.diagnostics);
    assert_eq!(compiled.controls.len(), BASE_CONTROL_TARGET_KIND_IDS.len());
    assert_eq!(
        compiled.catalog.entries.len(),
        BASE_CONTROL_TARGET_KIND_IDS.len()
    );
    assert_eq!(
        compiled.inspection.len(),
        BASE_CONTROL_TARGET_KIND_IDS.len()
    );

    for target_kind_id in BASE_CONTROL_TARGET_KIND_IDS {
        let catalog_entry = compiled
            .catalog
            .entry(target_kind_id)
            .expect("base control should be catalog-visible");
        let inspection = compiled
            .inspection
            .descriptor(target_kind_id)
            .expect("base control should be inspection-visible");

        assert_eq!(catalog_entry.category, "base-control");
        assert!(!catalog_entry.mount_eligible);
        assert!(
            catalog_entry
                .tags
                .iter()
                .any(|tag| tag == "catalog-visible")
        );
        assert!(
            catalog_entry
                .tags
                .iter()
                .any(|tag| tag == "inspection-ready")
        );
        assert!(catalog_entry.tags.iter().any(|tag| tag == "non-mountable"));
        assert_eq!(
            inspection.fact(ControlInspectionSection::MountEligibility, "eligible"),
            Some("false")
        );
        assert_eq!(
            inspection.fact(ControlInspectionSection::Input, "has_runtime_behavior"),
            Some("false")
        );
        assert_eq!(
            inspection.fact(ControlInspectionSection::State, "mutates_host_state"),
            Some("false")
        );
        assert_eq!(
            inspection.fact(
                ControlInspectionSection::Metadata,
                "render.has_backend_render_behavior"
            ),
            Some("false")
        );
    }
}

#[test]
fn base_control_collection_presets_lower_owner_crate_layout_vocabulary() {
    let compiled = BaseControlsPlugin::new().compile();

    for target_kind_id in [
        LIST_VIEW_CONTROL_KIND_ID,
        TREE_VIEW_CONTROL_KIND_ID,
        TABLE_VIEW_CONTROL_KIND_ID,
    ] {
        let control = compiled
            .controls
            .iter()
            .find(|control| control.module.kind.control_kind_id.as_str() == target_kind_id)
            .expect("collection control should be compiled");
        let layout = control.layout.summary();

        assert!(layout.layout_roles.iter().any(|role| role == "scroll"));
        assert!(!layout.scroll_requirements.is_empty());
        assert!(!layout.item_identities.is_empty());
        assert!(!layout.selection_identities.is_empty());
        assert!(!layout.virtualization_requirements.is_empty());
        assert!(!layout.large_content_budgets.is_empty());
        assert!(!layout.has_runtime_layout_behavior);
    }
}

#[test]
fn base_control_package_keeps_runtime_mount_eligibility_blocked() {
    let package = runenwerk_control_package();

    for kind in &package.control_kinds {
        assert!(!kind.compatibility.supports_runtime_mount);
        assert!(matches!(
            &kind.mount_eligibility,
            ControlMountEligibility::NotEligible { .. }
        ));
    }
}
