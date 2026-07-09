//! File: domain/ui/ui_controls/src/authoring/mod.rs
//! Crate: ui_controls

use ui_program::{RouteCapability, RouteId, RouteSchemaVersion};
use ui_schema::UiSchema;

use crate::diagnostics::{ControlDiagnosticDescriptor, ControlDiagnosticId};
use crate::interaction::ControlInteractionDescriptor;
use crate::kernel::{
    ControlKernelDescriptor, ControlKernelId, ControlKernelKind, ControlKernelSet,
};
use crate::migration::{ControlMigrationHook, ControlMigrationId};
use crate::package::{
    ControlBudgetEvidenceId, ControlBudgetEvidenceRequirement, ControlCatalogMetadata,
    ControlFixtureDescriptor, ControlFixtureId, ControlKindDescriptor, ControlKindId,
    ControlModuleDescriptor, ControlMountEligibility, ControlPackageDescriptor, ControlPackageId,
    ControlPackageVersion, ControlRenderEvidenceId, ControlRenderEvidenceRequirement,
    ControlRequirement, ControlRouteRequirement, ControlStoryDescriptor, ControlStoryId,
    ControlTargetProfileRef,
};
use crate::schema::ControlSchemaDescriptor;

#[derive(Clone, Debug, PartialEq)]
pub struct ControlKindAuthoringSpec {
    pub package_namespace: String,
    pub kind_suffix: String,
    pub display_name: String,
    pub description: String,
    pub target_profile: ControlTargetProfileRef,
    pub property_schema: UiSchema,
    pub state_schema: UiSchema,
    pub event_payload_schema: UiSchema,
    pub route_capability: RouteCapability,
    pub route_schema_version: RouteSchemaVersion,
    pub category: String,
    pub tags: Vec<String>,
    pub mount_ineligible_reason: String,
    pub fixture_description: String,
    pub story_description: String,
    pub diagnostic_message_template: String,
}

impl ControlKindAuthoringSpec {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        package_namespace: impl Into<String>,
        kind_suffix: impl Into<String>,
        display_name: impl Into<String>,
        description: impl Into<String>,
        target_profile: ControlTargetProfileRef,
        property_schema: UiSchema,
        state_schema: UiSchema,
        event_payload_schema: UiSchema,
        route_capability: RouteCapability,
    ) -> Self {
        let display_name = display_name.into();
        Self {
            package_namespace: package_namespace.into(),
            kind_suffix: kind_suffix.into(),
            fixture_description: format!("Default {display_name} descriptor fixture"),
            story_description: format!("{display_name} descriptor contract story placeholder"),
            diagnostic_message_template: format!(
                "{display_name} control package contract violation"
            ),
            display_name,
            description: description.into(),
            target_profile,
            property_schema,
            state_schema,
            event_payload_schema,
            route_capability,
            route_schema_version: RouteSchemaVersion::new(1),
            category: "base-control".to_owned(),
            tags: vec!["descriptor-only".to_owned()],
            mount_ineligible_reason:
                "runtime mount eligibility requires future story, render, and budget evidence"
                    .to_owned(),
        }
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_mount_ineligible_reason(mut self, reason: impl Into<String>) -> Self {
        self.mount_ineligible_reason = reason.into();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlKernelAuthoring {
    pub package_namespace: String,
    pub kind_suffix: String,
}

impl ControlKernelAuthoring {
    pub fn new(package_namespace: impl Into<String>, kind_suffix: impl Into<String>) -> Self {
        Self {
            package_namespace: package_namespace.into(),
            kind_suffix: kind_suffix.into(),
        }
    }

    pub fn build(&self) -> ControlAuthoredKernels {
        let layout = self.kernel(ControlKernelKind::Layout, "layout");
        let interaction = self.kernel(ControlKernelKind::Interaction, "interaction");
        let visual = self.kernel(ControlKernelKind::Visual, "visual");
        let accessibility = self.kernel(ControlKernelKind::Accessibility, "accessibility");
        let inspection = self.kernel(ControlKernelKind::Inspection, "inspection");
        let set = ControlKernelSet::new(
            layout.kernel_id.clone(),
            interaction.kernel_id.clone(),
            visual.kernel_id.clone(),
            accessibility.kernel_id.clone(),
            inspection.kernel_id.clone(),
        );
        ControlAuthoredKernels {
            set,
            descriptors: vec![layout, interaction, visual, accessibility, inspection],
        }
    }

    fn kernel(&self, kind: ControlKernelKind, suffix: &str) -> ControlKernelDescriptor {
        ControlKernelDescriptor::new(
            ControlKernelId::new(format!(
                "{}.{}.{}",
                self.package_namespace, self.kind_suffix, suffix
            )),
            kind,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlAuthoredKernels {
    pub set: ControlKernelSet,
    pub descriptors: Vec<ControlKernelDescriptor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlSchemaAuthoring {
    pub properties: ControlSchemaDescriptor,
    pub state: ControlSchemaDescriptor,
    pub event_payload: ControlSchemaDescriptor,
}

impl ControlSchemaAuthoring {
    pub fn new(
        property_schema: UiSchema,
        state_schema: UiSchema,
        event_payload_schema: UiSchema,
    ) -> Self {
        Self {
            properties: ControlSchemaDescriptor::properties(property_schema),
            state: ControlSchemaDescriptor::state(state_schema),
            event_payload: ControlSchemaDescriptor::event_payload(event_payload_schema),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlEvidenceAuthoring {
    pub fixture_id: ControlFixtureId,
    pub diagnostic_id: ControlDiagnosticId,
    pub migration_id: ControlMigrationId,
    pub story_id: ControlStoryId,
    pub render_evidence_id: ControlRenderEvidenceId,
    pub budget_evidence_id: ControlBudgetEvidenceId,
}

impl ControlEvidenceAuthoring {
    pub fn new(package_namespace: impl Into<String>, kind_suffix: impl Into<String>) -> Self {
        let package_namespace = package_namespace.into();
        let kind_suffix = kind_suffix.into();
        Self {
            fixture_id: ControlFixtureId::new(format!(
                "{package_namespace}.{kind_suffix}.fixture.default"
            )),
            diagnostic_id: ControlDiagnosticId::new(format!(
                "{package_namespace}.{kind_suffix}.diagnostic.contract"
            )),
            migration_id: ControlMigrationId::new(format!(
                "{package_namespace}.{kind_suffix}.migration.initial"
            )),
            story_id: ControlStoryId::new(format!(
                "{package_namespace}.{kind_suffix}.story.contract"
            )),
            render_evidence_id: ControlRenderEvidenceId::new(format!(
                "{package_namespace}.{kind_suffix}.evidence.render.contract"
            )),
            budget_evidence_id: ControlBudgetEvidenceId::new(format!(
                "{package_namespace}.{kind_suffix}.evidence.budget.contract"
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlModuleAuthoringBuilder {
    spec: ControlKindAuthoringSpec,
}

impl ControlModuleAuthoringBuilder {
    pub fn new(spec: ControlKindAuthoringSpec) -> Self {
        Self { spec }
    }

    pub fn build(self) -> ControlModuleDescriptor {
        let spec = self.spec;
        let base_id = format!("{}.{}", spec.package_namespace, spec.kind_suffix);
        let schemas = ControlSchemaAuthoring::new(
            spec.property_schema,
            spec.state_schema,
            spec.event_payload_schema,
        );
        let kernels =
            ControlKernelAuthoring::new(&spec.package_namespace, &spec.kind_suffix).build();
        let evidence = ControlEvidenceAuthoring::new(&spec.package_namespace, &spec.kind_suffix);
        let control_kind_id = ControlKindId::new(base_id.clone());
        let route_requirement = ControlRouteRequirement::new(
            RouteId::new(format!("{base_id}.intent")),
            spec.route_schema_version,
        )
        .with_capability(spec.route_capability.clone());

        let mut kind = ControlKindDescriptor::new(
            control_kind_id.clone(),
            spec.display_name.clone(),
            schemas.properties.schema_ref().clone(),
            schemas.state.schema_ref().clone(),
            schemas.event_payload.schema_ref().clone(),
            kernels.set,
        )
        .with_description(spec.description)
        .with_category(spec.category)
        .with_target_profile(spec.target_profile)
        .with_required_capability(spec.route_capability)
        .with_route_requirement(route_requirement)
        .with_fixture(evidence.fixture_id.clone())
        .with_diagnostic(evidence.diagnostic_id.clone())
        .with_migration(evidence.migration_id.clone())
        .with_story(evidence.story_id.clone())
        .with_mount_eligibility(ControlMountEligibility::not_eligible(spec.mount_ineligible_reason))
        .with_binding_requirement(ControlRequirement::new(
            format!("{base_id}.binding.contract"),
            "Binding behavior must be declared through host-owned state and route contracts.",
        ))
        .with_theme_token_requirement(ControlRequirement::new(
            format!("{base_id}.theme.contract"),
            "Visual states must use theme/token metadata rather than renderer-owned semantics.",
        ))
        .with_accessibility_requirement(ControlRequirement::new(
            format!("{base_id}.accessibility.contract"),
            "Accessibility role, label, focus, and inspection facts must be explicit before mount eligibility.",
        ))
        .with_render_evidence_requirement(ControlRenderEvidenceRequirement::new(
            evidence.render_evidence_id.clone(),
            "Renderer-neutral primitive evidence required before runtime mount eligibility.",
        ))
        .with_budget_evidence_requirement(ControlBudgetEvidenceRequirement::new(
            evidence.budget_evidence_id.clone(),
            "Layout, interaction, text, and render budget evidence required before runtime mount eligibility.",
        ));

        kind = kind.with_tag(spec.kind_suffix);
        for tag in spec.tags {
            kind = kind.with_tag(tag);
        }

        let mut module = ControlModuleDescriptor::new(kind)
            .with_schema(schemas.properties)
            .with_schema(schemas.state)
            .with_schema(schemas.event_payload)
            .with_fixture(ControlFixtureDescriptor::new(
                evidence.fixture_id,
                spec.fixture_description,
            ))
            .with_diagnostic(ControlDiagnosticDescriptor::contract(
                evidence.diagnostic_id,
                control_kind_id,
                spec.diagnostic_message_template,
            ))
            .with_migration(ControlMigrationHook::initial(
                evidence.migration_id,
                ControlPackageVersion::new(1),
            ))
            .with_story(ControlStoryDescriptor::new(
                evidence.story_id,
                spec.story_description,
            ));

        for kernel in kernels.descriptors {
            module = module.with_kernel(kernel);
        }

        module
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlPackageAuthoringBuilder {
    package_id: ControlPackageId,
    version: ControlPackageVersion,
    display_name: String,
    description: String,
    category: String,
    tags: Vec<String>,
    target_profiles: Vec<ControlTargetProfileRef>,
    catalog_metadata: Option<ControlCatalogMetadata>,
    modules: Vec<ControlModuleDescriptor>,
    interaction_descriptors: Vec<ControlInteractionDescriptor>,
}

impl ControlPackageAuthoringBuilder {
    pub fn new(package_id: impl Into<String>, version: ControlPackageVersion) -> Self {
        Self {
            package_id: ControlPackageId::new(package_id),
            version,
            display_name: String::new(),
            description: String::new(),
            category: "control-package".to_owned(),
            tags: vec!["descriptor-only".to_owned()],
            target_profiles: Vec::new(),
            catalog_metadata: None,
            modules: Vec::new(),
            interaction_descriptors: Vec::new(),
        }
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_target_profile(mut self, target_profile: ControlTargetProfileRef) -> Self {
        self.target_profiles.push(target_profile);
        self
    }

    pub fn with_catalog_metadata(mut self, catalog_metadata: ControlCatalogMetadata) -> Self {
        self.catalog_metadata = Some(catalog_metadata);
        self
    }

    pub fn with_module(mut self, module: ControlModuleDescriptor) -> Self {
        self.modules.push(module);
        self
    }

    pub fn with_interaction_descriptor(mut self, descriptor: ControlInteractionDescriptor) -> Self {
        self.interaction_descriptors.push(descriptor);
        self
    }

    pub fn with_authored_kind(mut self, spec: ControlKindAuthoringSpec) -> Self {
        self.modules
            .push(ControlModuleAuthoringBuilder::new(spec).build());
        self
    }

    pub fn build(self) -> ControlPackageDescriptor {
        let package_id_string = self.package_id.as_str().to_owned();
        let mut package =
            ControlPackageDescriptor::from_modules(self.package_id, self.version, self.modules)
                .with_display_name(self.display_name)
                .with_description(self.description)
                .with_category(self.category);

        for tag in self.tags {
            package = package.with_tag(tag);
        }
        for target_profile in self.target_profiles {
            package = package.with_target_profile(target_profile);
        }
        for descriptor in self.interaction_descriptors {
            package = package.with_interaction_descriptor(descriptor);
        }

        let catalog_metadata = self
            .catalog_metadata
            .unwrap_or_else(|| ControlCatalogMetadata::new(package_id_string, "Controls"));
        package.with_catalog_metadata(catalog_metadata)
    }
}
