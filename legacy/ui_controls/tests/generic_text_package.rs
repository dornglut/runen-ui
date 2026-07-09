use ui_controls::{
    BaseControlsPlugin, ControlCatalogIndex, ControlGenericTextDescriptor,
    ControlGenericTextRoleDescriptor, ControlGenericTextSemanticRole, ControlInspectionSection,
    ControlKindId, ControlPackageValidationReason, LABEL_CONTROL_KIND_ID,
    runenwerk_control_package,
};

#[test]
fn generic_text_descriptors_validate_with_dedicated_reasons() {
    let mut package = runenwerk_control_package();
    package
        .generic_text_descriptors
        .push(package.generic_text_descriptors[0].clone());
    assert_has_reason(
        package,
        ControlPackageValidationReason::DuplicateGenericTextDescriptor,
    );

    let mut package = runenwerk_control_package();
    package.generic_text_descriptors = vec![
        ControlGenericTextDescriptor::new(ControlKindId::new("missing.kind")).with_role(
            ControlGenericTextRoleDescriptor::new(
                "label.primary",
                ControlGenericTextSemanticRole::Label,
            ),
        ),
    ];
    assert_has_reason(
        package,
        ControlPackageValidationReason::UnresolvedGenericTextDescriptor,
    );

    let mut package = runenwerk_control_package();
    let kind_id = package.generic_text_descriptors[0].control_kind_id.clone();
    package.generic_text_descriptors = vec![ControlGenericTextDescriptor::new(kind_id).with_role(
        ControlGenericTextRoleDescriptor::new("bad", ControlGenericTextSemanticRole::Label),
    )];
    assert_has_reason(
        package,
        ControlPackageValidationReason::InvalidGenericTextRole,
    );

    let mut package = runenwerk_control_package();
    let kind_id = package.generic_text_descriptors[0].control_kind_id.clone();
    package.generic_text_descriptors = vec![
        ControlGenericTextDescriptor::new(kind_id).with_role(
            ControlGenericTextRoleDescriptor::new(
                "label.primary",
                ControlGenericTextSemanticRole::Label,
            )
            .with_max_lines(0),
        ),
    ];
    assert_has_reason(
        package,
        ControlPackageValidationReason::UnsupportedGenericTextLayoutPolicy,
    );

    let mut package = runenwerk_control_package();
    let mut descriptor = package.generic_text_descriptors[0].clone();
    descriptor.layout_support.renderer_backend_required = true;
    package.generic_text_descriptors = vec![descriptor];
    assert_has_reason(
        package,
        ControlPackageValidationReason::InvalidGenericTextDescriptor,
    );
}

#[test]
fn catalog_projects_generic_text_support() {
    let package = runenwerk_control_package();
    let catalog = ControlCatalogIndex::from_packages([&package]);
    let label = catalog
        .entry(LABEL_CONTROL_KIND_ID)
        .expect("label catalog entry");
    assert!(label.generic_text_supported);
    assert!(label.text_roles.contains(&"label.primary".to_owned()));
    assert!(label.inline_spans_supported);
    assert!(label.line_metrics_supported);
    assert!(label.glyph_evidence_supported);
    assert!(label.fallback_evidence_supported);
    assert!(!label.renderer_backend_required);
    assert!(!label.executes_host_commands);
    assert!(!label.mutates_product_state);
}

#[test]
fn inspection_projects_text_display_separately_from_text_editing() {
    let inspection = BaseControlsPlugin::new().inspection();
    let label = inspection
        .controls
        .iter()
        .find(|control| control.control_kind_id == LABEL_CONTROL_KIND_ID)
        .expect("label inspection");
    assert_eq!(
        label.fact(
            ControlInspectionSection::TextDisplay,
            "text_display.supported"
        ),
        Some("true")
    );
    assert_eq!(
        label.fact(
            ControlInspectionSection::TextDisplay,
            "text_display.inline_spans"
        ),
        Some("true")
    );
    assert_eq!(
        label.fact(
            ControlInspectionSection::TextEditing,
            "text_display.supported"
        ),
        None
    );
}

fn assert_has_reason(
    package: ui_controls::ControlPackageDescriptor,
    reason: ControlPackageValidationReason,
) {
    let report = package.validate_contract();
    assert!(!report.is_valid(), "package unexpectedly valid");
    assert!(
        report.has_reason(reason),
        "expected reason {:?}, got {:?}",
        reason,
        report.diagnostics
    );
}
