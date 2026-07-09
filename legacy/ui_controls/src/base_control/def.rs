//! File: domain/ui/ui_controls/src/base_control/def.rs
//! Crate: ui_controls

use ui_program::{RouteCapability, RouteSchemaVersion};

use crate::{
    ControlKindId, ControlTargetProfileRef, RUNENWERK_CONTROL_PACKAGE_ID,
    RUNENWERK_CONTROL_TARGET_EDITOR,
};

use super::{ControlContribution, ControlFieldGroup, ControlPreset, ControlThemeGroup};

#[derive(Clone, Debug, PartialEq)]
pub struct ControlDef {
    kind_suffix: String,
    display_name: String,
    description: String,
    route_capability: RouteCapability,
    route_schema_version: RouteSchemaVersion,
    target_profile: ControlTargetProfileRef,
    preset: ControlPreset,
    category: String,
    tags: Vec<String>,
    field_groups: Vec<ControlFieldGroup>,
    theme_groups: Vec<ControlThemeGroup>,
    mount_ineligible_reason: String,
}

impl ControlDef {
    pub fn builder(
        kind_suffix: impl Into<String>,
        display_name: impl Into<String>,
        preset: ControlPreset,
        route_capability: RouteCapability,
    ) -> ControlDefBuilder {
        let kind_suffix = kind_suffix.into();
        let display_name = display_name.into();
        ControlDefBuilder {
            def: Self {
                kind_suffix,
                description: format!(
                    "{display_name} reusable control descriptor with schemas, kernels, diagnostics, fixture, story, host-intent route metadata, and explicit non-mount eligibility until story proof is attached."
                ),
                display_name,
                route_capability,
                route_schema_version: RouteSchemaVersion::new(1),
                target_profile: ControlTargetProfileRef::new(RUNENWERK_CONTROL_TARGET_EDITOR),
                preset,
                category: "base-control".to_owned(),
                tags: vec![
                    "base-control".to_owned(),
                    "catalog-visible".to_owned(),
                    "inspection-ready".to_owned(),
                    "non-mountable".to_owned(),
                ],
                field_groups: Vec::new(),
                theme_groups: Vec::new(),
                mount_ineligible_reason:
                    "runtime mount eligibility requires future story, render, and budget evidence"
                        .to_owned(),
            },
        }
    }

    pub fn control_kind_id(&self) -> ControlKindId {
        ControlKindId::new(format!(
            "{RUNENWERK_CONTROL_PACKAGE_ID}.{}",
            self.kind_suffix
        ))
    }

    pub fn kind_suffix(&self) -> &str {
        &self.kind_suffix
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn route_capability(&self) -> &RouteCapability {
        &self.route_capability
    }

    pub fn route_schema_version(&self) -> RouteSchemaVersion {
        self.route_schema_version
    }

    pub fn target_profile(&self) -> &ControlTargetProfileRef {
        &self.target_profile
    }

    pub fn preset(&self) -> ControlPreset {
        self.preset
    }

    pub fn category(&self) -> &str {
        &self.category
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn field_groups(&self) -> &[ControlFieldGroup] {
        &self.field_groups
    }

    pub fn theme_groups(&self) -> &[ControlThemeGroup] {
        &self.theme_groups
    }

    pub fn mount_ineligible_reason(&self) -> &str {
        &self.mount_ineligible_reason
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlDefBuilder {
    def: ControlDef,
}

impl ControlDefBuilder {
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.def.description = description.into();
        self
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.def.category = category.into();
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.def.tags.push(tag.into());
        self
    }

    pub fn with_target_profile(mut self, target_profile: ControlTargetProfileRef) -> Self {
        self.def.target_profile = target_profile;
        self
    }

    pub fn with_route_schema_version(mut self, version: RouteSchemaVersion) -> Self {
        self.def.route_schema_version = version;
        self
    }

    pub fn with_field_group(mut self, group: ControlFieldGroup) -> Self {
        self.def.field_groups.push(group);
        self
    }

    pub fn with_theme_group(mut self, group: ControlThemeGroup) -> Self {
        self.def.theme_groups.push(group);
        self
    }

    pub fn with_mount_ineligible_reason(mut self, reason: impl Into<String>) -> Self {
        self.def.mount_ineligible_reason = reason.into();
        self
    }

    pub fn build(self) -> ControlDef {
        self.def
    }

    pub fn build_contribution(self) -> ControlContribution {
        ControlContribution::new(self.build())
    }
}
