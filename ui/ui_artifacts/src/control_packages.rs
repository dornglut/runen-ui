use serde::{Deserialize, Serialize};
use ui_controls::{
    ControlDiagnosticDescriptor, ControlFixtureDescriptor, ControlKernelDescriptor,
    ControlKindDescriptor, ControlMigrationHook, ControlPackageDescriptor,
    ControlPackageRegistrySnapshot, ControlPackageValidationReport, ControlRouteRequirement,
    ControlSchemaDescriptor, ControlStoryDescriptor, ControlTargetProfileRef,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiControlPackageArtifact {
    pub manifest: UiControlPackageArtifactManifest,
    pub tables: UiControlPackageArtifactTables,
    pub validation: ControlPackageValidationReport,
}

impl UiControlPackageArtifact {
    pub fn from_registry_snapshot(snapshot: &ControlPackageRegistrySnapshot) -> Self {
        Self {
            manifest: UiControlPackageArtifactManifest::from_registry_snapshot(snapshot),
            tables: UiControlPackageArtifactTables::from_registry_snapshot(snapshot),
            validation: snapshot.validate_contract(),
        }
    }

    pub fn validate_contract(&self) -> &ControlPackageValidationReport {
        &self.validation
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiControlPackageArtifactManifest {
    pub package_ids: Vec<String>,
    pub control_kind_ids: Vec<String>,
    pub schema_count: usize,
    pub kernel_count: usize,
    pub diagnostic_count: usize,
    pub migration_count: usize,
    pub story_count: usize,
}

impl UiControlPackageArtifactManifest {
    pub fn from_registry_snapshot(snapshot: &ControlPackageRegistrySnapshot) -> Self {
        Self {
            package_ids: snapshot
                .packages
                .iter()
                .map(|package| package.package_id.as_str().to_owned())
                .collect(),
            control_kind_ids: snapshot
                .control_kinds
                .iter()
                .map(|kind| kind.control_kind_id.as_str().to_owned())
                .collect(),
            schema_count: snapshot.schemas.len(),
            kernel_count: snapshot.kernels.len(),
            diagnostic_count: snapshot.diagnostics.len(),
            migration_count: snapshot.migrations.len(),
            story_count: snapshot.stories.len(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiControlPackageArtifactTables {
    pub packages: Vec<ControlPackageDescriptor>,
    pub control_kinds: Vec<ControlKindDescriptor>,
    pub schemas: Vec<ControlSchemaDescriptor>,
    pub kernels: Vec<ControlKernelDescriptor>,
    pub fixtures: Vec<ControlFixtureDescriptor>,
    pub diagnostics: Vec<ControlDiagnosticDescriptor>,
    pub migrations: Vec<ControlMigrationHook>,
    pub stories: Vec<ControlStoryDescriptor>,
    pub route_requirements: Vec<ControlRouteRequirement>,
    pub target_profiles: Vec<ControlTargetProfileRef>,
}

impl UiControlPackageArtifactTables {
    pub fn from_registry_snapshot(snapshot: &ControlPackageRegistrySnapshot) -> Self {
        Self {
            packages: snapshot.packages.clone(),
            control_kinds: snapshot.control_kinds.clone(),
            schemas: snapshot.schemas.clone(),
            kernels: snapshot.kernels.clone(),
            fixtures: snapshot.fixtures.clone(),
            diagnostics: snapshot.diagnostics.clone(),
            migrations: snapshot.migrations.clone(),
            stories: snapshot.stories.clone(),
            route_requirements: snapshot.route_requirements.clone(),
            target_profiles: snapshot.target_profiles.clone(),
        }
    }
}
