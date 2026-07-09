use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use ui_schema::{UiSchema, UiSchemaRef, UiSchemaShape};

use super::ids::{ControlKindId, ControlPackageId, ControlPackageVersion, ControlTargetProfileRef};
use super::metadata::ControlMountEligibility;
use crate::diagnostics::ControlDiagnosticId;
use crate::kernel::{ControlKernelDescriptor, ControlKernelKind};
use crate::package::{ControlKindDescriptor, ControlModuleDescriptor, ControlPackageDescriptor};
use crate::schema::ControlSchemaDescriptor;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPackageValidationReport {
    pub diagnostics: Vec<ControlPackageValidationDiagnostic>,
}
impl ControlPackageValidationReport {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == ControlPackageValidationSeverity::Error)
    }
    pub fn push(&mut self, diagnostic: ControlPackageValidationDiagnostic) {
        self.diagnostics.push(diagnostic);
    }
    pub fn extend(&mut self, report: ControlPackageValidationReport) {
        self.diagnostics.extend(report.diagnostics);
    }
    pub fn has_reason(&self, reason: ControlPackageValidationReason) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.reason == reason)
    }
}
impl Default for ControlPackageValidationReport {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPackageValidationDiagnostic {
    pub diagnostic_id: ControlDiagnosticId,
    pub severity: ControlPackageValidationSeverity,
    pub scope: ControlPackageValidationScope,
    pub path: ControlPackageValidationPath,
    pub reason: ControlPackageValidationReason,
    pub message: String,
}
impl ControlPackageValidationDiagnostic {
    pub fn package(
        package_id: ControlPackageId,
        reason: ControlPackageValidationReason,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            ControlPackageValidationScope::Package { package_id },
            ControlPackageValidationPath::new("package"),
            reason,
            message,
        )
    }
    pub fn kind(
        control_kind_id: ControlKindId,
        reason: ControlPackageValidationReason,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            ControlPackageValidationScope::Kind { control_kind_id },
            ControlPackageValidationPath::new("control_kind"),
            reason,
            message,
        )
    }
    pub fn new(
        scope: ControlPackageValidationScope,
        path: ControlPackageValidationPath,
        reason: ControlPackageValidationReason,
        message: impl Into<String>,
    ) -> Self {
        Self {
            diagnostic_id: ControlDiagnosticId::new(format!(
                "runenwerk.ui.controls.validation.{}",
                reason.as_snake_case()
            )),
            severity: ControlPackageValidationSeverity::Error,
            scope,
            path,
            reason,
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlPackageValidationSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlPackageValidationScope {
    Package { package_id: ControlPackageId },
    Kind { control_kind_id: ControlKindId },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPackageValidationPath {
    pub segments: Vec<String>,
}
impl ControlPackageValidationPath {
    pub fn new(segment: impl Into<String>) -> Self {
        Self {
            segments: vec![segment.into()],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlPackageValidationReason {
    MissingPackageMetadata,
    MissingKindMetadata,
    MissingSchema,
    MissingKernel,
    MissingFixture,
    MissingDiagnostic,
    MissingMigration,
    MissingStory,
    MissingRoute,
    MissingCapability,
    MissingTargetProfile,
    MissingMountEvidence,
    DuplicatePackageId,
    DuplicateControlKindId,
    DuplicateSchemaRef,
    DuplicateKernelId,
    DuplicateFixtureId,
    DuplicateDiagnosticId,
    DuplicateMigrationId,
    DuplicateStoryId,
    DuplicateInteractionDescriptor,
    DuplicateOverlayDescriptor,
    DuplicateEditableTextDescriptor,
    DuplicateGenericTextDescriptor,
    DuplicateSurface2DDescriptor,
    DuplicateRouteRequirement,
    InvalidDeprecation,
    InvalidMigrationVersion,
    InvalidMountEligibility,
    UnresolvedInteractionDescriptor,
    UnresolvedOverlayDescriptor,
    InvalidOverlayDescriptor,
    UnresolvedEditableTextDescriptor,
    InvalidEditableTextDescriptor,
    UnresolvedGenericTextDescriptor,
    InvalidGenericTextDescriptor,
    InvalidGenericTextRole,
    UnsupportedGenericTextLayoutPolicy,
    UnresolvedSurface2DDescriptor,
    InvalidSurface2DDescriptor,
    UnsupportedSurface2DDescriptor,
    UnsupportedTargetProfile,
    UnresolvedReference,
    RenderEvidenceMissing,
    BudgetEvidenceMissing,
}
impl ControlPackageValidationReason {
    pub const fn as_snake_case(self) -> &'static str {
        match self {
            Self::MissingPackageMetadata => "missing_package_metadata",
            Self::MissingKindMetadata => "missing_kind_metadata",
            Self::MissingSchema => "missing_schema",
            Self::MissingKernel => "missing_kernel",
            Self::MissingFixture => "missing_fixture",
            Self::MissingDiagnostic => "missing_diagnostic",
            Self::MissingMigration => "missing_migration",
            Self::MissingStory => "missing_story",
            Self::MissingRoute => "missing_route",
            Self::MissingCapability => "missing_capability",
            Self::MissingTargetProfile => "missing_target_profile",
            Self::MissingMountEvidence => "missing_mount_evidence",
            Self::DuplicatePackageId => "duplicate_package_id",
            Self::DuplicateControlKindId => "duplicate_control_kind_id",
            Self::DuplicateSchemaRef => "duplicate_schema_ref",
            Self::DuplicateKernelId => "duplicate_kernel_id",
            Self::DuplicateFixtureId => "duplicate_fixture_id",
            Self::DuplicateDiagnosticId => "duplicate_diagnostic_id",
            Self::DuplicateMigrationId => "duplicate_migration_id",
            Self::DuplicateStoryId => "duplicate_story_id",
            Self::DuplicateInteractionDescriptor => "duplicate_interaction_descriptor",
            Self::DuplicateOverlayDescriptor => "duplicate_overlay_descriptor",
            Self::DuplicateEditableTextDescriptor => "duplicate_editable_text_descriptor",
            Self::DuplicateGenericTextDescriptor => "duplicate_generic_text_descriptor",
            Self::DuplicateSurface2DDescriptor => "duplicate_surface2d_descriptor",
            Self::DuplicateRouteRequirement => "duplicate_route_requirement",
            Self::InvalidDeprecation => "invalid_deprecation",
            Self::InvalidMigrationVersion => "invalid_migration_version",
            Self::InvalidMountEligibility => "invalid_mount_eligibility",
            Self::UnresolvedInteractionDescriptor => "unresolved_interaction_descriptor",
            Self::UnresolvedOverlayDescriptor => "unresolved_overlay_descriptor",
            Self::InvalidOverlayDescriptor => "invalid_overlay_descriptor",
            Self::UnresolvedEditableTextDescriptor => "unresolved_editable_text_descriptor",
            Self::InvalidEditableTextDescriptor => "invalid_editable_text_descriptor",
            Self::UnresolvedGenericTextDescriptor => "unresolved_generic_text_descriptor",
            Self::InvalidGenericTextDescriptor => "invalid_generic_text_descriptor",
            Self::InvalidGenericTextRole => "invalid_generic_text_role",
            Self::UnsupportedGenericTextLayoutPolicy => "unsupported_generic_text_layout_policy",
            Self::UnresolvedSurface2DDescriptor => "unresolved_surface2d_descriptor",
            Self::InvalidSurface2DDescriptor => "invalid_surface2d_descriptor",
            Self::UnsupportedSurface2DDescriptor => "unsupported_surface2d_descriptor",
            Self::UnsupportedTargetProfile => "unsupported_target_profile",
            Self::UnresolvedReference => "unresolved_reference",
            Self::RenderEvidenceMissing => "render_evidence_missing",
            Self::BudgetEvidenceMissing => "budget_evidence_missing",
        }
    }
}

impl ControlKindDescriptor {
    pub fn validate_contract(&self) -> ControlPackageValidationReport {
        let mut report = ControlPackageValidationReport::new();
        if self.display_name.trim().is_empty() {
            report.push(ControlPackageValidationDiagnostic::kind(
                self.control_kind_id.clone(),
                ControlPackageValidationReason::MissingKindMetadata,
                "control kind display name must not be empty",
            ));
        }
        if self.description.trim().is_empty() {
            report.push(ControlPackageValidationDiagnostic::kind(
                self.control_kind_id.clone(),
                ControlPackageValidationReason::MissingKindMetadata,
                "control kind description must not be empty",
            ));
        }
        if self.target_profiles.is_empty() {
            report.push(ControlPackageValidationDiagnostic::kind(
                self.control_kind_id.clone(),
                ControlPackageValidationReason::MissingTargetProfile,
                "control kind must name at least one target profile",
            ));
        }
        if self.diagnostic_ids.is_empty() {
            report.push(ControlPackageValidationDiagnostic::kind(
                self.control_kind_id.clone(),
                ControlPackageValidationReason::MissingDiagnostic,
                "control kind must name at least one diagnostic",
            ));
        }
        if self.fixture_ids.is_empty() {
            report.push(ControlPackageValidationDiagnostic::kind(
                self.control_kind_id.clone(),
                ControlPackageValidationReason::MissingFixture,
                "control kind must name at least one fixture",
            ));
        }
        if self.route_requirements.is_empty() {
            report.push(ControlPackageValidationDiagnostic::kind(
                self.control_kind_id.clone(),
                ControlPackageValidationReason::MissingRoute,
                "control kind must name at least one route requirement",
            ));
        }
        push_duplicate_kind_values(
            &mut report,
            &self.control_kind_id,
            "target_profile",
            self.target_profiles
                .iter()
                .map(|profile| profile.as_str().to_owned()),
            ControlPackageValidationReason::UnsupportedTargetProfile,
        );
        push_duplicate_kind_values(
            &mut report,
            &self.control_kind_id,
            "route",
            self.route_requirements
                .iter()
                .map(|route| route.route_id.as_str().to_owned()),
            ControlPackageValidationReason::DuplicateRouteRequirement,
        );
        push_duplicate_kind_values(
            &mut report,
            &self.control_kind_id,
            "story",
            self.story_ids.iter().map(|story| story.as_str().to_owned()),
            ControlPackageValidationReason::DuplicateStoryId,
        );
        if let ControlMountEligibility::RequiresEvidence {
            story_ids,
            render_evidence_ids,
            budget_evidence_ids,
        } = &self.mount_eligibility
        {
            if story_ids.is_empty() {
                report.push(ControlPackageValidationDiagnostic::kind(
                    self.control_kind_id.clone(),
                    ControlPackageValidationReason::MissingMountEvidence,
                    "mount eligibility requires at least one story",
                ));
            }
            if render_evidence_ids.is_empty() {
                report.push(ControlPackageValidationDiagnostic::kind(
                    self.control_kind_id.clone(),
                    ControlPackageValidationReason::RenderEvidenceMissing,
                    "mount eligibility requires render evidence",
                ));
            }
            if budget_evidence_ids.is_empty() {
                report.push(ControlPackageValidationDiagnostic::kind(
                    self.control_kind_id.clone(),
                    ControlPackageValidationReason::BudgetEvidenceMissing,
                    "mount eligibility requires budget evidence",
                ));
            }
        }
        report
    }
}

impl ControlModuleDescriptor {
    pub fn validate_contract(&self) -> ControlPackageValidationReport {
        ControlPackageDescriptor::from_modules(
            ControlPackageId::new("runenwerk.validation.module"),
            ControlPackageVersion::new(1),
            [self.clone()],
        )
        .with_display_name("Module validation")
        .with_description("Synthetic package for validating one control module descriptor.")
        .with_category("validation")
        .with_target_profile(ControlTargetProfileRef::new("runenwerk.validation.profile"))
        .validate_contract()
    }
}

impl ControlPackageDescriptor {
    pub fn validated(self) -> Result<Self, ControlPackageValidationReport> {
        let report = self.validate_contract();
        if report.is_valid() {
            Ok(self)
        } else {
            Err(report)
        }
    }
    pub fn validate_contract(&self) -> ControlPackageValidationReport {
        let mut report = ControlPackageValidationReport::new();
        if self.display_name.trim().is_empty() {
            report.push(ControlPackageValidationDiagnostic::package(
                self.package_id.clone(),
                ControlPackageValidationReason::MissingPackageMetadata,
                "control package display name must not be empty",
            ));
        }
        if self.description.trim().is_empty() {
            report.push(ControlPackageValidationDiagnostic::package(
                self.package_id.clone(),
                ControlPackageValidationReason::MissingPackageMetadata,
                "control package description must not be empty",
            ));
        }
        if self.target_profiles.is_empty() {
            report.push(ControlPackageValidationDiagnostic::package(
                self.package_id.clone(),
                ControlPackageValidationReason::MissingTargetProfile,
                "control package must name at least one target profile",
            ));
        }
        if self.control_kinds.is_empty() {
            report.push(ControlPackageValidationDiagnostic::package(
                self.package_id.clone(),
                ControlPackageValidationReason::MissingKindMetadata,
                "control package must contain at least one control kind",
            ));
        }
        validate_duplicates(self, &mut report);
        if self.deprecation.replacement_package_id() == Some(&self.package_id) {
            report.push(ControlPackageValidationDiagnostic::package(
                self.package_id.clone(),
                ControlPackageValidationReason::InvalidDeprecation,
                "package replacement must not point at itself",
            ));
        }
        let property_schemas = schema_index(&self.property_schemas);
        let state_schemas = schema_index(&self.state_schemas);
        let event_schemas = schema_index(&self.event_payload_schemas);
        let kernels = kernel_index(&self.kernels);
        let fixtures = self
            .fixtures
            .iter()
            .map(|fixture| fixture.fixture_id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let diagnostics = self
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.diagnostic_id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let migrations = self
            .migrations
            .iter()
            .map(|migration| migration.migration_id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let stories = self
            .stories
            .iter()
            .map(|story| story.story_id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let render_evidence = self
            .render_evidence_requirements
            .iter()
            .map(|evidence| evidence.evidence_id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let budget_evidence = self
            .budget_evidence_requirements
            .iter()
            .map(|evidence| evidence.evidence_id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let control_kind_ids = self
            .control_kinds
            .iter()
            .map(|kind| kind.control_kind_id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        for kind in &self.control_kinds {
            report.extend(kind.validate_contract());
            validate_kind_refs(
                kind,
                &property_schemas,
                &state_schemas,
                &event_schemas,
                &kernels,
                &fixtures,
                &diagnostics,
                &migrations,
                &stories,
                &render_evidence,
                &budget_evidence,
                &mut report,
            );
        }
        for migration in &self.migrations {
            if let Err(message) = migration.validate_contract(&self.package_id) {
                report.push(ControlPackageValidationDiagnostic::package(
                    self.package_id.clone(),
                    ControlPackageValidationReason::InvalidMigrationVersion,
                    message,
                ));
            }
        }
        for diagnostic in &self.diagnostics {
            if diagnostic.message_template.trim().is_empty() {
                report.push(ControlPackageValidationDiagnostic::package(
                    self.package_id.clone(),
                    ControlPackageValidationReason::MissingDiagnostic,
                    "diagnostic message template must not be empty",
                ));
            }
        }
        for descriptor in &self.interaction_descriptors {
            if !control_kind_ids.contains(descriptor.control_kind_id.as_str()) {
                report.push(ControlPackageValidationDiagnostic::kind(
                    descriptor.control_kind_id.clone(),
                    ControlPackageValidationReason::UnresolvedInteractionDescriptor,
                    format!(
                        "interaction descriptor {} does not reference a control kind in package {}",
                        descriptor.control_kind_id.as_str(),
                        self.package_id.as_str()
                    ),
                ));
            }
        }
        super::overlay_validation::validate_overlay_descriptors(
            self,
            &control_kind_ids,
            &mut report,
        );
        super::editable_text_validation::validate_editable_text_descriptors(
            self,
            &control_kind_ids,
            &mut report,
        );
        super::generic_text_validation::validate_generic_text_descriptors(
            self,
            &control_kind_ids,
            &mut report,
        );
        super::surface2d_validation::validate_surface2d_descriptors(
            self,
            &control_kind_ids,
            &mut report,
        );
        report
    }
}

fn validate_duplicates(
    package: &ControlPackageDescriptor,
    report: &mut ControlPackageValidationReport,
) {
    push_duplicate_package_values(
        report,
        &package.package_id,
        "control_kind",
        package
            .control_kinds
            .iter()
            .map(|kind| kind.control_kind_id.as_str().to_owned()),
        ControlPackageValidationReason::DuplicateControlKindId,
    );
    push_duplicate_package_values(
        report,
        &package.package_id,
        "property_schema",
        package
            .property_schemas
            .iter()
            .map(|schema| schema_ref_key(schema.schema_ref())),
        ControlPackageValidationReason::DuplicateSchemaRef,
    );
    push_duplicate_package_values(
        report,
        &package.package_id,
        "state_schema",
        package
            .state_schemas
            .iter()
            .map(|schema| schema_ref_key(schema.schema_ref())),
        ControlPackageValidationReason::DuplicateSchemaRef,
    );
    push_duplicate_package_values(
        report,
        &package.package_id,
        "event_payload_schema",
        package
            .event_payload_schemas
            .iter()
            .map(|schema| schema_ref_key(schema.schema_ref())),
        ControlPackageValidationReason::DuplicateSchemaRef,
    );
    push_duplicate_package_values(
        report,
        &package.package_id,
        "kernel",
        package
            .kernels
            .iter()
            .map(|kernel| kernel.kernel_id.as_str().to_owned()),
        ControlPackageValidationReason::DuplicateKernelId,
    );
    push_duplicate_package_values(
        report,
        &package.package_id,
        "fixture",
        package
            .fixtures
            .iter()
            .map(|fixture| fixture.fixture_id.as_str().to_owned()),
        ControlPackageValidationReason::DuplicateFixtureId,
    );
    push_duplicate_package_values(
        report,
        &package.package_id,
        "diagnostic",
        package
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.diagnostic_id.as_str().to_owned()),
        ControlPackageValidationReason::DuplicateDiagnosticId,
    );
    push_duplicate_package_values(
        report,
        &package.package_id,
        "migration",
        package
            .migrations
            .iter()
            .map(|migration| migration.migration_id.as_str().to_owned()),
        ControlPackageValidationReason::DuplicateMigrationId,
    );
    push_duplicate_package_values(
        report,
        &package.package_id,
        "story",
        package
            .stories
            .iter()
            .map(|story| story.story_id.as_str().to_owned()),
        ControlPackageValidationReason::DuplicateStoryId,
    );
    push_duplicate_package_values(
        report,
        &package.package_id,
        "interaction_descriptor",
        package
            .interaction_descriptors
            .iter()
            .map(|descriptor| descriptor.control_kind_id.as_str().to_owned()),
        ControlPackageValidationReason::DuplicateInteractionDescriptor,
    );
    push_duplicate_package_values(
        report,
        &package.package_id,
        "editable_text_descriptor",
        package
            .editable_text_descriptors
            .iter()
            .map(|descriptor| descriptor.control_kind_id.as_str().to_owned()),
        ControlPackageValidationReason::DuplicateEditableTextDescriptor,
    );
    push_duplicate_package_values(
        report,
        &package.package_id,
        "generic_text_descriptor",
        package
            .generic_text_descriptors
            .iter()
            .map(|descriptor| descriptor.control_kind_id.as_str().to_owned()),
        ControlPackageValidationReason::DuplicateGenericTextDescriptor,
    );
    push_duplicate_package_values(
        report,
        &package.package_id,
        "surface2d_descriptor",
        package
            .surface2d_descriptors
            .iter()
            .map(|descriptor| descriptor.control_kind_id.as_str().to_owned()),
        ControlPackageValidationReason::DuplicateSurface2DDescriptor,
    );
}

#[allow(clippy::too_many_arguments)]
fn validate_kind_refs(
    kind: &ControlKindDescriptor,
    property_schemas: &BTreeMap<String, &ControlSchemaDescriptor>,
    state_schemas: &BTreeMap<String, &ControlSchemaDescriptor>,
    event_schemas: &BTreeMap<String, &ControlSchemaDescriptor>,
    kernels: &BTreeMap<String, &ControlKernelDescriptor>,
    fixtures: &BTreeSet<String>,
    diagnostics: &BTreeSet<String>,
    migrations: &BTreeSet<String>,
    stories: &BTreeSet<String>,
    render_evidence: &BTreeSet<String>,
    budget_evidence: &BTreeSet<String>,
    report: &mut ControlPackageValidationReport,
) {
    if !property_schemas.contains_key(&schema_ref_key(&kind.property_schema)) {
        report.push(ControlPackageValidationDiagnostic::kind(
            kind.control_kind_id.clone(),
            ControlPackageValidationReason::MissingSchema,
            "property schema ref is unresolved",
        ));
    }
    if !state_schemas.contains_key(&schema_ref_key(&kind.state_schema)) {
        report.push(ControlPackageValidationDiagnostic::kind(
            kind.control_kind_id.clone(),
            ControlPackageValidationReason::MissingSchema,
            "state schema ref is unresolved",
        ));
    }
    let event_schema = event_schemas.get(&schema_ref_key(&kind.event_payload_schema));
    if event_schema.is_none() {
        report.push(ControlPackageValidationDiagnostic::kind(
            kind.control_kind_id.clone(),
            ControlPackageValidationReason::MissingSchema,
            "event payload schema ref is unresolved",
        ));
    }
    for (kernel_id, expected_kind) in [
        (&kind.kernels.layout, ControlKernelKind::Layout),
        (&kind.kernels.interaction, ControlKernelKind::Interaction),
        (&kind.kernels.visual, ControlKernelKind::Visual),
        (
            &kind.kernels.accessibility,
            ControlKernelKind::Accessibility,
        ),
        (&kind.kernels.inspection, ControlKernelKind::Inspection),
    ] {
        match kernels.get(kernel_id.as_str()) {
            Some(kernel) if kernel.kind == expected_kind => {}
            Some(_) => report.push(ControlPackageValidationDiagnostic::kind(
                kind.control_kind_id.clone(),
                ControlPackageValidationReason::MissingKernel,
                format!("kernel {} has wrong role", kernel_id.as_str()),
            )),
            None => report.push(ControlPackageValidationDiagnostic::kind(
                kind.control_kind_id.clone(),
                ControlPackageValidationReason::MissingKernel,
                format!("kernel {} is unresolved", kernel_id.as_str()),
            )),
        }
    }
    for fixture_id in &kind.fixture_ids {
        if !fixtures.contains(fixture_id.as_str()) {
            report.push(ControlPackageValidationDiagnostic::kind(
                kind.control_kind_id.clone(),
                ControlPackageValidationReason::MissingFixture,
                format!("fixture {} is unresolved", fixture_id.as_str()),
            ));
        }
    }
    for diagnostic_id in &kind.diagnostic_ids {
        if !diagnostics.contains(diagnostic_id.as_str()) {
            report.push(ControlPackageValidationDiagnostic::kind(
                kind.control_kind_id.clone(),
                ControlPackageValidationReason::MissingDiagnostic,
                format!("diagnostic {} is unresolved", diagnostic_id.as_str()),
            ));
        }
    }
    for migration_id in &kind.migration_ids {
        if !migrations.contains(migration_id.as_str()) {
            report.push(ControlPackageValidationDiagnostic::kind(
                kind.control_kind_id.clone(),
                ControlPackageValidationReason::MissingMigration,
                format!("migration {} is unresolved", migration_id.as_str()),
            ));
        }
    }
    for story_id in &kind.story_ids {
        if !stories.contains(story_id.as_str()) {
            report.push(ControlPackageValidationDiagnostic::kind(
                kind.control_kind_id.clone(),
                ControlPackageValidationReason::MissingStory,
                format!("story {} is unresolved", story_id.as_str()),
            ));
        }
    }
    if let ControlMountEligibility::RequiresEvidence {
        story_ids,
        render_evidence_ids,
        budget_evidence_ids,
    } = &kind.mount_eligibility
    {
        for story_id in story_ids {
            if !stories.contains(story_id.as_str()) {
                report.push(ControlPackageValidationDiagnostic::kind(
                    kind.control_kind_id.clone(),
                    ControlPackageValidationReason::MissingStory,
                    format!("mount story {} is unresolved", story_id.as_str()),
                ));
            }
        }
        for evidence_id in render_evidence_ids {
            if !render_evidence.contains(evidence_id.as_str()) {
                report.push(ControlPackageValidationDiagnostic::kind(
                    kind.control_kind_id.clone(),
                    ControlPackageValidationReason::RenderEvidenceMissing,
                    format!("render evidence {} is unresolved", evidence_id.as_str()),
                ));
            }
        }
        for evidence_id in budget_evidence_ids {
            if !budget_evidence.contains(evidence_id.as_str()) {
                report.push(ControlPackageValidationDiagnostic::kind(
                    kind.control_kind_id.clone(),
                    ControlPackageValidationReason::BudgetEvidenceMissing,
                    format!("budget evidence {} is unresolved", evidence_id.as_str()),
                ));
            }
        }
    }
    if kind
        .deprecation
        .replacement_control_kind_id()
        .is_some_and(|replacement| replacement == &kind.control_kind_id)
    {
        report.push(ControlPackageValidationDiagnostic::kind(
            kind.control_kind_id.clone(),
            ControlPackageValidationReason::InvalidDeprecation,
            "kind replacement must not point at itself",
        ));
    }
    if event_schema.is_some_and(|schema| schema_contains_route_ref(&schema.schema))
        && kind.route_requirements.is_empty()
    {
        report.push(ControlPackageValidationDiagnostic::kind(
            kind.control_kind_id.clone(),
            ControlPackageValidationReason::MissingRoute,
            "route ref event schema requires route metadata",
        ));
    }
}

fn schema_ref_key(schema_ref: &UiSchemaRef) -> String {
    format!("{}@{}", schema_ref.id.as_str(), schema_ref.version.value())
}
fn schema_index(schemas: &[ControlSchemaDescriptor]) -> BTreeMap<String, &ControlSchemaDescriptor> {
    schemas
        .iter()
        .map(|schema| (schema_ref_key(schema.schema_ref()), schema))
        .collect()
}
fn kernel_index(kernels: &[ControlKernelDescriptor]) -> BTreeMap<String, &ControlKernelDescriptor> {
    kernels
        .iter()
        .map(|kernel| (kernel.kernel_id.as_str().to_owned(), kernel))
        .collect()
}
fn schema_contains_route_ref(schema: &UiSchema) -> bool {
    schema
        .fields
        .values()
        .any(|field| shape_contains_route_ref(&field.shape))
}
fn shape_contains_route_ref(shape: &UiSchemaShape) -> bool {
    match shape {
        UiSchemaShape::RouteRef => true,
        UiSchemaShape::List(inner) | UiSchemaShape::Nullable(inner) => {
            shape_contains_route_ref(inner)
        }
        _ => false,
    }
}
fn push_duplicate_package_values(
    report: &mut ControlPackageValidationReport,
    package_id: &ControlPackageId,
    label: &'static str,
    values: impl IntoIterator<Item = String>,
    reason: ControlPackageValidationReason,
) {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value.clone()) {
            report.push(ControlPackageValidationDiagnostic::package(
                package_id.clone(),
                reason,
                format!("duplicate {label} id {value}"),
            ));
        }
    }
}
fn push_duplicate_kind_values(
    report: &mut ControlPackageValidationReport,
    control_kind_id: &ControlKindId,
    label: &'static str,
    values: impl IntoIterator<Item = String>,
    reason: ControlPackageValidationReason,
) {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value.clone()) {
            report.push(ControlPackageValidationDiagnostic::kind(
                control_kind_id.clone(),
                reason,
                format!("duplicate {label} value {value}"),
            ));
        }
    }
}
