use ui_controls::{
    ControlCatalogIndex, ControlCatalogQuery, ControlInspectionDescriptor,
    ControlInspectionSection, ControlKindId, ControlMountEligibility, ControlStoryProofCategory,
    ControlStoryProofRequirement, ControlStoryProofSummary, ControlStoryProofVerdict,
    LABEL_CONTROL_KIND_ID, RUNENWERK_CONTROL_PACKAGE_ID, RUNENWERK_CONTROL_TARGET_EDITOR,
    runenwerk_control_package,
};

#[test]
fn control_catalog_derives_deterministic_entries_from_package() {
    let package = runenwerk_control_package();
    let first = ControlCatalogIndex::from_packages([&package]);
    let second = ControlCatalogIndex::from_packages([&package]);

    assert_eq!(first, second);
    assert_eq!(first.entries.len(), 9);
    assert!(first.entry(LABEL_CONTROL_KIND_ID).is_some());
}

#[test]
fn control_catalog_filters_by_identity_metadata_target_and_capability() {
    let package = runenwerk_control_package();
    let index = ControlCatalogIndex::from_packages([&package]);

    let by_package =
        index.query(&ControlCatalogQuery::new().with_package_id(RUNENWERK_CONTROL_PACKAGE_ID));
    assert_eq!(by_package.entries.len(), 9);

    let by_kind =
        index.query(&ControlCatalogQuery::new().with_control_kind_id(LABEL_CONTROL_KIND_ID));
    assert_eq!(by_kind.entries.len(), 1);
    assert_eq!(by_kind.entries[0].display_name, "Label");

    let by_category = index.query(&ControlCatalogQuery::new().with_category("base-control"));
    assert_eq!(by_category.entries.len(), 9);

    let by_tag = index.query(&ControlCatalogQuery::new().with_tag("label"));
    assert_eq!(by_tag.entries.len(), 1);
    assert_eq!(by_tag.entries[0].control_kind_id, LABEL_CONTROL_KIND_ID);

    let by_target = index
        .query(&ControlCatalogQuery::new().with_target_profile(RUNENWERK_CONTROL_TARGET_EDITOR));
    assert_eq!(by_target.entries.len(), 9);

    let by_capability =
        index.query(&ControlCatalogQuery::new().with_capability("runenwerk.ui.controls.read"));
    assert!(
        by_capability
            .entries
            .iter()
            .any(|entry| entry.control_kind_id == LABEL_CONTROL_KIND_ID)
    );
}

#[test]
fn control_catalog_filters_story_diagnostics_and_mount_state() {
    let package = runenwerk_control_package();
    let index = ControlCatalogIndex::from_packages([&package]);

    assert_eq!(
        index
            .query(&ControlCatalogQuery::new().with_story_required(true))
            .entries
            .len(),
        9
    );
    assert_eq!(
        index
            .query(&ControlCatalogQuery::new().with_has_diagnostics(true))
            .entries
            .len(),
        9
    );
    assert_eq!(
        index
            .query(&ControlCatalogQuery::new().with_mount_eligible(false))
            .entries
            .len(),
        9
    );
    assert!(
        index
            .query(&ControlCatalogQuery::new().with_mount_eligible(true))
            .entries
            .is_empty()
    );
}

#[test]
fn control_catalog_inspection_exposes_package_control_facts() {
    let package = runenwerk_control_package();
    let label_id = ControlKindId::new(LABEL_CONTROL_KIND_ID);
    let kind = package
        .control_kind(&label_id)
        .expect("label control kind should exist");
    let inspection = ControlInspectionDescriptor::from_control_kind(&package, kind);

    assert_eq!(inspection.package_id, RUNENWERK_CONTROL_PACKAGE_ID);
    assert_eq!(inspection.control_kind_id, LABEL_CONTROL_KIND_ID);
    assert_eq!(
        inspection.fact(ControlInspectionSection::Identity, "display_name"),
        Some("Label")
    );
    assert_eq!(
        inspection.fact(ControlInspectionSection::Metadata, "category"),
        Some("base-control")
    );
    assert!(
        inspection
            .fact(ControlInspectionSection::Schemas, "properties")
            .is_some_and(|value| value.contains("label.properties@1"))
    );
    assert!(
        inspection
            .fact(ControlInspectionSection::Kernels, "layout")
            .is_some_and(|value| value.ends_with(".label.layout"))
    );
    assert!(
        inspection
            .fact(ControlInspectionSection::Routes, "routes")
            .is_some_and(|value| value.ends_with(".label.intent"))
    );
    assert!(
        inspection
            .fact(ControlInspectionSection::Fixtures, "fixtures")
            .is_some_and(|value| value.ends_with(".label.fixture.default"))
    );
    assert!(
        inspection
            .fact(ControlInspectionSection::Stories, "stories")
            .is_some_and(|value| value.ends_with(".label.story.contract"))
    );
    assert_eq!(inspection.diagnostic_badges.len(), 1);
}

#[test]
fn control_catalog_story_proof_badge_preserves_summary_status() {
    let package = runenwerk_control_package();
    let label_id = ControlKindId::new(LABEL_CONTROL_KIND_ID);
    let kind = package
        .control_kind(&label_id)
        .expect("label control kind should exist");
    let story_id = kind.story_ids[0].clone();
    let requirement = ControlStoryProofRequirement::new(
        story_id.clone(),
        ControlStoryProofCategory::Accessibility,
    );
    let summary = ControlStoryProofSummary::from_satisfied_story_ids(
        kind.control_kind_id.clone(),
        &[requirement],
        [story_id],
    );
    let inspection = ControlInspectionDescriptor::from_control_kind(&package, kind)
        .with_story_proof_summary(&summary);

    let badge = inspection
        .story_proof_badge
        .expect("story proof badge should be attached");
    assert_eq!(badge.verdict, ControlStoryProofVerdict::Satisfied);
    assert!(badge.first_unsatisfied_story_id.is_none());
}

#[test]
fn control_catalog_inspection_does_not_upgrade_runtime_mount_eligibility() {
    let package = runenwerk_control_package();
    let label_id = ControlKindId::new(LABEL_CONTROL_KIND_ID);
    let kind = package
        .control_kind(&label_id)
        .expect("label control kind should exist");
    let index = ControlCatalogIndex::from_packages([&package]);
    let entry = index
        .entry(LABEL_CONTROL_KIND_ID)
        .expect("label catalog entry should exist");
    let inspection = ControlInspectionDescriptor::from_control_kind(&package, kind);

    assert!(!entry.mount_eligible);
    assert_eq!(
        inspection.fact(ControlInspectionSection::MountEligibility, "eligible"),
        Some("false")
    );
    assert!(matches!(
        &kind.mount_eligibility,
        ControlMountEligibility::NotEligible { .. }
    ));
}
