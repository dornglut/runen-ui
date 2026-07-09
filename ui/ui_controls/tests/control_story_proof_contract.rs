use ui_controls::{
    ControlMountEligibility, ControlPackageValidationReason, ControlStoryId,
    ControlStoryMatrixDescriptor, ControlStoryProofCategory, ControlStoryProofDiagnostic,
    ControlStoryProofEnvelope, ControlStoryProofExpectedOutcome, ControlStoryProofProfile,
    ControlStoryProofRequirement, ControlStoryProofSummary, ControlStoryProofVerdict,
    runenwerk_control_package,
};

#[test]
fn control_story_proof_valid_matrix_validates() {
    let package = runenwerk_control_package();
    let kind = &package.control_kinds[0];
    let matrix = minimum_matrix(kind.control_kind_id.clone(), kind.story_ids[0].clone());

    let report = matrix.validate_against_package(&package);

    assert!(report.is_valid(), "{:?}", report.diagnostics);
}

#[test]
fn control_story_proof_rejects_duplicate_requirement() {
    let package = runenwerk_control_package();
    let kind = &package.control_kinds[0];
    let story_id = kind.story_ids[0].clone();
    let matrix = ControlStoryMatrixDescriptor::new(
        kind.control_kind_id.clone(),
        ControlStoryProofProfile::DescriptorOnly,
    )
    .with_requirement(ControlStoryProofRequirement::new(
        story_id.clone(),
        ControlStoryProofCategory::Normal,
    ))
    .with_requirement(ControlStoryProofRequirement::new(
        story_id,
        ControlStoryProofCategory::Normal,
    ));

    let report = matrix.validate_against_package(&package);

    assert!(!report.is_valid());
    assert!(report.has_reason(ControlPackageValidationReason::DuplicateStoryId));
}

#[test]
fn control_story_proof_rejects_unresolved_story_requirement() {
    let package = runenwerk_control_package();
    let kind = &package.control_kinds[0];
    let matrix = ControlStoryMatrixDescriptor::new(
        kind.control_kind_id.clone(),
        ControlStoryProofProfile::DescriptorOnly,
    )
    .with_requirement(ControlStoryProofRequirement::new(
        ControlStoryId::new("runenwerk.ui.controls.story.missing"),
        ControlStoryProofCategory::Normal,
    ));

    let report = matrix.validate_against_package(&package);

    assert!(!report.is_valid());
    assert!(report.has_reason(ControlPackageValidationReason::MissingStory));
}

#[test]
fn control_story_proof_rejects_missing_minimum_category() {
    let package = runenwerk_control_package();
    let kind = &package.control_kinds[0];
    let story_id = kind.story_ids[0].clone();
    let matrix = ControlStoryMatrixDescriptor::new(
        kind.control_kind_id.clone(),
        ControlStoryProofProfile::MinimumMaturity,
    )
    .with_requirement(ControlStoryProofRequirement::new(
        story_id.clone(),
        ControlStoryProofCategory::Normal,
    ))
    .with_requirement(ControlStoryProofRequirement::expected_failure(
        story_id.clone(),
    ))
    .with_requirement(ControlStoryProofRequirement::new(
        story_id.clone(),
        ControlStoryProofCategory::Accessibility,
    ))
    .with_requirement(ControlStoryProofRequirement::new(
        story_id,
        ControlStoryProofCategory::Render,
    ));

    let report = matrix.validate_against_package(&package);

    assert!(!report.is_valid());
    assert!(report.has_reason(ControlPackageValidationReason::UnresolvedReference));
}

#[test]
fn control_story_proof_expected_failure_is_first_class_not_normal_pass() {
    let package = runenwerk_control_package();
    let kind = &package.control_kinds[0];
    let story_id = kind.story_ids[0].clone();
    let requirement = ControlStoryProofRequirement::expected_failure(story_id);

    assert_eq!(requirement.category, ControlStoryProofCategory::Failure);
    assert_eq!(
        requirement.expected_outcome,
        ControlStoryProofExpectedOutcome::ExpectedFailure
    );
    assert!(requirement.expected_outcome.is_expected_failure());
}

#[test]
fn control_story_proof_summary_reports_first_unsatisfied_requirement() {
    let package = runenwerk_control_package();
    let kind = &package.control_kinds[0];
    let story_id = kind.story_ids[0].clone();
    let missing_story_id = ControlStoryId::new("runenwerk.ui.controls.story.unsatisfied");
    let requirements = vec![
        ControlStoryProofRequirement::new(story_id.clone(), ControlStoryProofCategory::Normal),
        ControlStoryProofRequirement::new(
            missing_story_id.clone(),
            ControlStoryProofCategory::Accessibility,
        ),
    ];

    let summary = ControlStoryProofSummary::from_satisfied_story_ids(
        kind.control_kind_id.clone(),
        &requirements,
        [story_id],
    );

    assert_eq!(summary.verdict, ControlStoryProofVerdict::Unsatisfied);
    assert_eq!(summary.satisfied_requirements, 1);
    assert_eq!(summary.total_requirements, 2);
    assert_eq!(
        summary
            .first_unsatisfied_requirement
            .as_ref()
            .map(|requirement| &requirement.story_id),
        Some(&missing_story_id)
    );
}

#[test]
fn control_story_proof_envelope_preserves_first_blocking_diagnostic() {
    let package = runenwerk_control_package();
    let kind = &package.control_kinds[0];
    let story_id = kind.story_ids[0].clone();
    let matrix = minimum_matrix(kind.control_kind_id.clone(), story_id.clone());
    let diagnostic = ControlStoryProofDiagnostic::new(
        kind.control_kind_id.clone(),
        Some(story_id.clone()),
        Some(ControlStoryProofCategory::Render),
        "render evidence did not satisfy the story proof requirement",
    );
    let summary = ControlStoryProofSummary::from_satisfied_story_ids(
        kind.control_kind_id.clone(),
        &matrix.requirements,
        [story_id],
    )
    .with_first_blocking_diagnostic(diagnostic.clone());

    assert_eq!(summary.verdict, ControlStoryProofVerdict::Unsatisfied);
    assert_eq!(summary.first_blocking_diagnostic, Some(diagnostic));
}

#[test]
fn control_story_proof_envelope_stays_descriptor_only_for_mounting() {
    let package = runenwerk_control_package();
    let kind = &package.control_kinds[0];
    let matrix = minimum_matrix(kind.control_kind_id.clone(), kind.story_ids[0].clone());
    let envelope = ControlStoryProofEnvelope::not_evaluated(matrix);

    assert_eq!(
        envelope.summary.verdict,
        ControlStoryProofVerdict::NotEvaluated
    );
    assert!(matches!(
        &kind.mount_eligibility,
        ControlMountEligibility::NotEligible { .. }
    ));
}

fn minimum_matrix(
    control_kind_id: ui_controls::ControlKindId,
    story_id: ControlStoryId,
) -> ControlStoryMatrixDescriptor {
    ControlStoryMatrixDescriptor::new(control_kind_id, ControlStoryProofProfile::MinimumMaturity)
        .with_requirement(ControlStoryProofRequirement::new(
            story_id.clone(),
            ControlStoryProofCategory::Normal,
        ))
        .with_requirement(ControlStoryProofRequirement::expected_failure(
            story_id.clone(),
        ))
        .with_requirement(ControlStoryProofRequirement::new(
            story_id.clone(),
            ControlStoryProofCategory::Accessibility,
        ))
        .with_requirement(ControlStoryProofRequirement::new(
            story_id.clone(),
            ControlStoryProofCategory::Render,
        ))
        .with_requirement(ControlStoryProofRequirement::new(
            story_id,
            ControlStoryProofCategory::Budget,
        ))
}
