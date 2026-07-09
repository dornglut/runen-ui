use serde::{Deserialize, Serialize};
use ui_program::RouteCapability;
use ui_schema::UiSchemaRef;

use super::ids::{
    ControlFixtureId, ControlKindId, ControlPackageId, ControlPackageVersion, ControlStoryId,
    ControlTargetProfileRef,
};
use super::metadata::{
    ControlBudgetEvidenceRequirement, ControlCatalogMetadata, ControlCompatibilityFlags,
    ControlFixtureDescriptor, ControlMountEligibility, ControlPackageCategory,
    ControlRenderEvidenceRequirement, ControlRequirement, ControlRouteRequirement,
    ControlStoryDescriptor, ControlTag,
};
use crate::diagnostics::{ControlDiagnosticDescriptor, ControlDiagnosticId};
use crate::editable_text::ControlEditableTextDescriptor;
use crate::generic_text::ControlGenericTextDescriptor;
use crate::interaction::ControlInteractionDescriptor;
use crate::kernel::{ControlKernelDescriptor, ControlKernelId, ControlKernelSet};
use crate::migration::{ControlDeprecationStatus, ControlMigrationHook, ControlMigrationId};
use crate::overlay::ControlOverlayDescriptor;
use crate::schema::{ControlSchemaDescriptor, ControlSchemaRole};
use crate::surface2d::ControlSurface2DDescriptor;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlKindDescriptor {
    pub control_kind_id: ControlKindId,
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: ControlPackageCategory,
    #[serde(default)]
    pub tags: Vec<ControlTag>,
    #[serde(default)]
    pub target_profiles: Vec<ControlTargetProfileRef>,
    #[serde(default)]
    pub compatibility: ControlCompatibilityFlags,
    pub property_schema: UiSchemaRef,
    pub state_schema: UiSchemaRef,
    pub event_payload_schema: UiSchemaRef,
    pub kernels: ControlKernelSet,
    #[serde(default)]
    pub required_capabilities: Vec<RouteCapability>,
    #[serde(default)]
    pub route_requirements: Vec<ControlRouteRequirement>,
    #[serde(default)]
    pub fixture_ids: Vec<ControlFixtureId>,
    #[serde(default)]
    pub diagnostic_ids: Vec<ControlDiagnosticId>,
    #[serde(default)]
    pub migration_ids: Vec<ControlMigrationId>,
    #[serde(default)]
    pub story_ids: Vec<ControlStoryId>,
    #[serde(default)]
    pub mount_eligibility: ControlMountEligibility,
    #[serde(default)]
    pub binding_requirements: Vec<ControlRequirement>,
    #[serde(default)]
    pub theme_token_requirements: Vec<ControlRequirement>,
    #[serde(default)]
    pub accessibility_requirements: Vec<ControlRequirement>,
    #[serde(default)]
    pub render_evidence_requirements: Vec<ControlRenderEvidenceRequirement>,
    #[serde(default)]
    pub budget_evidence_requirements: Vec<ControlBudgetEvidenceRequirement>,
    #[serde(default)]
    pub deprecation: ControlDeprecationStatus,
}

impl ControlKindDescriptor {
    pub fn new(
        control_kind_id: ControlKindId,
        display_name: impl Into<String>,
        property_schema: UiSchemaRef,
        state_schema: UiSchemaRef,
        event_payload_schema: UiSchemaRef,
        kernels: ControlKernelSet,
    ) -> Self {
        Self {
            control_kind_id,
            display_name: display_name.into(),
            description: String::new(),
            category: ControlPackageCategory::default(),
            tags: Vec::new(),
            target_profiles: Vec::new(),
            compatibility: ControlCompatibilityFlags::descriptor_only(),
            property_schema,
            state_schema,
            event_payload_schema,
            kernels,
            required_capabilities: Vec::new(),
            route_requirements: Vec::new(),
            fixture_ids: Vec::new(),
            diagnostic_ids: Vec::new(),
            migration_ids: Vec::new(),
            story_ids: Vec::new(),
            mount_eligibility: ControlMountEligibility::default(),
            binding_requirements: Vec::new(),
            theme_token_requirements: Vec::new(),
            accessibility_requirements: Vec::new(),
            render_evidence_requirements: Vec::new(),
            budget_evidence_requirements: Vec::new(),
            deprecation: ControlDeprecationStatus::Active,
        }
    }
    pub fn with_description(mut self, value: impl Into<String>) -> Self {
        self.description = value.into();
        self
    }
    pub fn with_category(mut self, value: impl Into<String>) -> Self {
        self.category = ControlPackageCategory::new(value);
        self
    }
    pub fn with_tag(mut self, value: impl Into<String>) -> Self {
        self.tags.push(ControlTag::new(value));
        self
    }
    pub fn with_target_profile(mut self, value: ControlTargetProfileRef) -> Self {
        self.target_profiles.push(value);
        self
    }
    pub fn with_required_capability(mut self, value: RouteCapability) -> Self {
        self.required_capabilities.push(value);
        self
    }
    pub fn with_route_requirement(mut self, value: ControlRouteRequirement) -> Self {
        self.route_requirements.push(value);
        self
    }
    pub fn with_fixture(mut self, value: ControlFixtureId) -> Self {
        self.fixture_ids.push(value);
        self
    }
    pub fn with_diagnostic(mut self, value: ControlDiagnosticId) -> Self {
        self.diagnostic_ids.push(value);
        self
    }
    pub fn with_migration(mut self, value: ControlMigrationId) -> Self {
        self.migration_ids.push(value);
        self
    }
    pub fn with_story(mut self, value: ControlStoryId) -> Self {
        self.story_ids.push(value);
        self
    }
    pub fn with_mount_eligibility(mut self, value: ControlMountEligibility) -> Self {
        self.mount_eligibility = value;
        self
    }
    pub fn with_binding_requirement(mut self, value: ControlRequirement) -> Self {
        self.binding_requirements.push(value);
        self
    }
    pub fn with_theme_token_requirement(mut self, value: ControlRequirement) -> Self {
        self.theme_token_requirements.push(value);
        self
    }
    pub fn with_accessibility_requirement(mut self, value: ControlRequirement) -> Self {
        self.accessibility_requirements.push(value);
        self
    }
    pub fn with_render_evidence_requirement(
        mut self,
        value: ControlRenderEvidenceRequirement,
    ) -> Self {
        self.render_evidence_requirements.push(value);
        self
    }
    pub fn with_budget_evidence_requirement(
        mut self,
        value: ControlBudgetEvidenceRequirement,
    ) -> Self {
        self.budget_evidence_requirements.push(value);
        self
    }
    pub fn with_deprecation(mut self, value: ControlDeprecationStatus) -> Self {
        self.deprecation = value;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlModuleDescriptor {
    pub kind: ControlKindDescriptor,
    #[serde(default)]
    pub schemas: Vec<ControlSchemaDescriptor>,
    #[serde(default)]
    pub kernels: Vec<ControlKernelDescriptor>,
    #[serde(default)]
    pub fixtures: Vec<ControlFixtureDescriptor>,
    #[serde(default)]
    pub diagnostics: Vec<ControlDiagnosticDescriptor>,
    #[serde(default)]
    pub migrations: Vec<ControlMigrationHook>,
    #[serde(default)]
    pub stories: Vec<ControlStoryDescriptor>,
}

impl ControlModuleDescriptor {
    pub fn new(kind: ControlKindDescriptor) -> Self {
        Self {
            kind,
            schemas: Vec::new(),
            kernels: Vec::new(),
            fixtures: Vec::new(),
            diagnostics: Vec::new(),
            migrations: Vec::new(),
            stories: Vec::new(),
        }
    }
    pub fn with_schema(mut self, value: ControlSchemaDescriptor) -> Self {
        self.schemas.push(value);
        self
    }
    pub fn with_kernel(mut self, value: ControlKernelDescriptor) -> Self {
        self.kernels.push(value);
        self
    }
    pub fn with_fixture(mut self, value: ControlFixtureDescriptor) -> Self {
        self.fixtures.push(value);
        self
    }
    pub fn with_diagnostic(mut self, value: ControlDiagnosticDescriptor) -> Self {
        self.diagnostics.push(value);
        self
    }
    pub fn with_migration(mut self, value: ControlMigrationHook) -> Self {
        self.migrations.push(value);
        self
    }
    pub fn with_story(mut self, value: ControlStoryDescriptor) -> Self {
        self.stories.push(value);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlPackageDescriptor {
    pub package_id: ControlPackageId,
    pub version: ControlPackageVersion,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: ControlPackageCategory,
    #[serde(default)]
    pub tags: Vec<ControlTag>,
    #[serde(default)]
    pub target_profiles: Vec<ControlTargetProfileRef>,
    #[serde(default)]
    pub compatibility: ControlCompatibilityFlags,
    #[serde(default)]
    pub catalog_metadata: ControlCatalogMetadata,
    #[serde(default)]
    pub control_kinds: Vec<ControlKindDescriptor>,
    #[serde(default)]
    pub property_schemas: Vec<ControlSchemaDescriptor>,
    #[serde(default)]
    pub state_schemas: Vec<ControlSchemaDescriptor>,
    #[serde(default)]
    pub event_payload_schemas: Vec<ControlSchemaDescriptor>,
    #[serde(default)]
    pub required_capabilities: Vec<RouteCapability>,
    #[serde(default)]
    pub route_requirements: Vec<ControlRouteRequirement>,
    #[serde(default)]
    pub kernels: Vec<ControlKernelDescriptor>,
    #[serde(default)]
    pub kernel_ids: Vec<ControlKernelId>,
    #[serde(default)]
    pub fixture_ids: Vec<ControlFixtureId>,
    #[serde(default)]
    pub fixtures: Vec<ControlFixtureDescriptor>,
    #[serde(default)]
    pub diagnostics: Vec<ControlDiagnosticDescriptor>,
    #[serde(default)]
    pub diagnostic_ids: Vec<ControlDiagnosticId>,
    #[serde(default)]
    pub migrations: Vec<ControlMigrationHook>,
    #[serde(default)]
    pub migration_ids: Vec<ControlMigrationId>,
    #[serde(default)]
    pub story_ids: Vec<ControlStoryId>,
    #[serde(default)]
    pub stories: Vec<ControlStoryDescriptor>,
    #[serde(default)]
    pub interaction_descriptors: Vec<ControlInteractionDescriptor>,
    #[serde(default)]
    pub overlay_descriptors: Vec<ControlOverlayDescriptor>,
    #[serde(default)]
    pub editable_text_descriptors: Vec<ControlEditableTextDescriptor>,
    #[serde(default)]
    pub generic_text_descriptors: Vec<ControlGenericTextDescriptor>,
    #[serde(default)]
    pub surface2d_descriptors: Vec<ControlSurface2DDescriptor>,
    #[serde(default)]
    pub mount_eligibility: ControlMountEligibility,
    #[serde(default)]
    pub binding_requirements: Vec<ControlRequirement>,
    #[serde(default)]
    pub theme_token_requirements: Vec<ControlRequirement>,
    #[serde(default)]
    pub accessibility_requirements: Vec<ControlRequirement>,
    #[serde(default)]
    pub render_evidence_requirements: Vec<ControlRenderEvidenceRequirement>,
    #[serde(default)]
    pub budget_evidence_requirements: Vec<ControlBudgetEvidenceRequirement>,
    #[serde(default)]
    pub deprecation: ControlDeprecationStatus,
}

impl ControlPackageDescriptor {
    pub fn new(package_id: ControlPackageId, version: ControlPackageVersion) -> Self {
        Self {
            package_id,
            version,
            display_name: String::new(),
            description: String::new(),
            category: ControlPackageCategory::default(),
            tags: Vec::new(),
            target_profiles: Vec::new(),
            compatibility: ControlCompatibilityFlags::descriptor_only(),
            catalog_metadata: ControlCatalogMetadata::default(),
            control_kinds: Vec::new(),
            property_schemas: Vec::new(),
            state_schemas: Vec::new(),
            event_payload_schemas: Vec::new(),
            required_capabilities: Vec::new(),
            route_requirements: Vec::new(),
            kernels: Vec::new(),
            kernel_ids: Vec::new(),
            fixture_ids: Vec::new(),
            fixtures: Vec::new(),
            diagnostics: Vec::new(),
            diagnostic_ids: Vec::new(),
            migrations: Vec::new(),
            migration_ids: Vec::new(),
            story_ids: Vec::new(),
            stories: Vec::new(),
            interaction_descriptors: Vec::new(),
            overlay_descriptors: Vec::new(),
            editable_text_descriptors: Vec::new(),
            generic_text_descriptors: Vec::new(),
            surface2d_descriptors: Vec::new(),
            mount_eligibility: ControlMountEligibility::default(),
            binding_requirements: Vec::new(),
            theme_token_requirements: Vec::new(),
            accessibility_requirements: Vec::new(),
            render_evidence_requirements: Vec::new(),
            budget_evidence_requirements: Vec::new(),
            deprecation: ControlDeprecationStatus::Active,
        }
    }
    pub fn from_modules(
        package_id: ControlPackageId,
        version: ControlPackageVersion,
        modules: impl IntoIterator<Item = ControlModuleDescriptor>,
    ) -> Self {
        let mut package = Self::new(package_id, version);
        for module in modules {
            package = package.with_module(module);
        }
        package
    }
    pub fn with_display_name(mut self, value: impl Into<String>) -> Self {
        self.display_name = value.into();
        self
    }
    pub fn with_description(mut self, value: impl Into<String>) -> Self {
        self.description = value.into();
        self
    }
    pub fn with_category(mut self, value: impl Into<String>) -> Self {
        self.category = ControlPackageCategory::new(value);
        self
    }
    pub fn with_tag(mut self, value: impl Into<String>) -> Self {
        self.tags.push(ControlTag::new(value));
        self
    }
    pub fn with_target_profile(mut self, value: ControlTargetProfileRef) -> Self {
        self.target_profiles.push(value);
        self
    }
    pub fn with_catalog_metadata(mut self, value: ControlCatalogMetadata) -> Self {
        self.catalog_metadata = value;
        self
    }
    pub fn with_mount_eligibility(mut self, value: ControlMountEligibility) -> Self {
        self.mount_eligibility = value;
        self
    }
    pub fn with_module(mut self, module: ControlModuleDescriptor) -> Self {
        let ControlModuleDescriptor {
            kind,
            schemas,
            kernels,
            fixtures,
            diagnostics,
            migrations,
            stories,
        } = module;
        self.required_capabilities
            .extend(kind.required_capabilities.iter().cloned());
        self.route_requirements
            .extend(kind.route_requirements.iter().cloned());
        self.fixture_ids.extend(kind.fixture_ids.iter().cloned());
        self.diagnostic_ids
            .extend(kind.diagnostic_ids.iter().cloned());
        self.migration_ids
            .extend(kind.migration_ids.iter().cloned());
        self.story_ids.extend(kind.story_ids.iter().cloned());
        self.binding_requirements
            .extend(kind.binding_requirements.iter().cloned());
        self.theme_token_requirements
            .extend(kind.theme_token_requirements.iter().cloned());
        self.accessibility_requirements
            .extend(kind.accessibility_requirements.iter().cloned());
        self.render_evidence_requirements
            .extend(kind.render_evidence_requirements.iter().cloned());
        self.budget_evidence_requirements
            .extend(kind.budget_evidence_requirements.iter().cloned());
        for kernel_id in kind.kernels.ids() {
            self.kernel_ids.push(kernel_id.clone());
        }
        for schema in schemas {
            match schema.role {
                ControlSchemaRole::Properties => self.property_schemas.push(schema),
                ControlSchemaRole::State => self.state_schemas.push(schema),
                ControlSchemaRole::EventPayload => self.event_payload_schemas.push(schema),
            }
        }
        self.kernels.extend(kernels);
        self.fixtures.extend(fixtures);
        self.diagnostics.extend(diagnostics);
        self.migrations.extend(migrations);
        self.stories.extend(stories);
        self.control_kinds.push(kind);
        self
    }
    pub fn with_interaction_descriptor(mut self, descriptor: ControlInteractionDescriptor) -> Self {
        self.interaction_descriptors.push(descriptor);
        self
    }
    pub fn with_overlay_descriptor(mut self, descriptor: ControlOverlayDescriptor) -> Self {
        self.overlay_descriptors.push(descriptor);
        self
    }
    pub fn with_editable_text_descriptor(
        mut self,
        descriptor: ControlEditableTextDescriptor,
    ) -> Self {
        self.editable_text_descriptors.push(descriptor);
        self
    }
    pub fn with_generic_text_descriptor(
        mut self,
        descriptor: ControlGenericTextDescriptor,
    ) -> Self {
        self.generic_text_descriptors.push(descriptor);
        self
    }
    pub fn with_surface2d_descriptor(mut self, descriptor: ControlSurface2DDescriptor) -> Self {
        self.surface2d_descriptors.push(descriptor);
        self
    }
    pub fn control_kind(&self, id: &ControlKindId) -> Option<&ControlKindDescriptor> {
        self.control_kinds
            .iter()
            .find(|kind| &kind.control_kind_id == id)
    }
    pub fn interaction_descriptor(
        &self,
        control_kind_id: &ControlKindId,
    ) -> Option<&ControlInteractionDescriptor> {
        self.interaction_descriptors
            .iter()
            .find(|descriptor| &descriptor.control_kind_id == control_kind_id)
    }
    pub fn overlay_descriptor(
        &self,
        control_kind_id: &ControlKindId,
    ) -> Option<&ControlOverlayDescriptor> {
        self.overlay_descriptors
            .iter()
            .find(|descriptor| &descriptor.control_kind_id == control_kind_id)
    }
    pub fn editable_text_descriptor(
        &self,
        control_kind_id: &ControlKindId,
    ) -> Option<&ControlEditableTextDescriptor> {
        self.editable_text_descriptors
            .iter()
            .find(|descriptor| &descriptor.control_kind_id == control_kind_id)
    }
    pub fn generic_text_descriptor(
        &self,
        control_kind_id: &ControlKindId,
    ) -> Option<&ControlGenericTextDescriptor> {
        self.generic_text_descriptors
            .iter()
            .find(|descriptor| &descriptor.control_kind_id == control_kind_id)
    }
    pub fn surface2d_descriptor(
        &self,
        control_kind_id: &ControlKindId,
    ) -> Option<&ControlSurface2DDescriptor> {
        self.surface2d_descriptors
            .iter()
            .find(|descriptor| &descriptor.control_kind_id == control_kind_id)
    }
}
