//! File: domain/ui/ui_controls/src/registry.rs
//! Crate: ui_controls

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::diagnostics::ControlDiagnosticDescriptor;
use crate::kernel::ControlKernelDescriptor;
use crate::migration::ControlMigrationHook;
use crate::package::{
    ControlFixtureDescriptor, ControlKindDescriptor, ControlKindId, ControlPackageDescriptor,
    ControlPackageId, ControlPackageValidationReport, ControlRouteRequirement,
    ControlStoryDescriptor, ControlTargetProfileRef,
};
use crate::schema::ControlSchemaDescriptor;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControlPackageRegistry {
    packages: BTreeMap<String, ControlPackageDescriptor>,
}

impl ControlPackageRegistry {
    pub fn new() -> Self {
        Self {
            packages: BTreeMap::new(),
        }
    }

    pub fn with_package(
        mut self,
        package: ControlPackageDescriptor,
    ) -> Result<Self, ControlPackageRegistryError> {
        self.register(package)?;
        Ok(self)
    }

    pub fn register(
        &mut self,
        package: ControlPackageDescriptor,
    ) -> Result<(), ControlPackageRegistryError> {
        let report = package.validate_contract();
        if !report.is_valid() {
            return Err(ControlPackageRegistryError::InvalidPackage { report });
        }
        let package_key = package.package_id.as_str().to_owned();
        if self.packages.contains_key(&package_key) {
            return Err(ControlPackageRegistryError::DuplicatePackage {
                package_id: package_key,
            });
        }
        for kind in &package.control_kinds {
            if self.contains_kind(&kind.control_kind_id) {
                return Err(ControlPackageRegistryError::DuplicateControlKind {
                    control_kind_id: kind.control_kind_id.as_str().to_owned(),
                });
            }
        }
        self.packages.insert(package_key, package);
        Ok(())
    }

    pub fn package(&self, package_id: &ControlPackageId) -> Option<&ControlPackageDescriptor> {
        self.packages.get(package_id.as_str())
    }
    pub fn control_kind(&self, control_kind_id: &ControlKindId) -> Option<&ControlKindDescriptor> {
        self.packages.values().find_map(|package| {
            package
                .control_kinds
                .iter()
                .find(|kind| &kind.control_kind_id == control_kind_id)
        })
    }
    pub fn contains_kind(&self, control_kind_id: &ControlKindId) -> bool {
        self.control_kind(control_kind_id).is_some()
    }
    pub fn diagnostics_for(
        &self,
        package_id: &ControlPackageId,
    ) -> Option<ControlPackageValidationReport> {
        self.package(package_id)
            .map(ControlPackageDescriptor::validate_contract)
    }

    pub fn snapshot(&self) -> ControlPackageRegistrySnapshot {
        let packages: Vec<_> = self.packages.values().cloned().collect();
        ControlPackageRegistrySnapshot {
            control_kinds: packages
                .iter()
                .flat_map(|package| package.control_kinds.iter().cloned())
                .collect(),
            schemas: packages
                .iter()
                .flat_map(|package| {
                    package
                        .property_schemas
                        .iter()
                        .chain(package.state_schemas.iter())
                        .chain(package.event_payload_schemas.iter())
                        .cloned()
                })
                .collect(),
            kernels: packages
                .iter()
                .flat_map(|package| package.kernels.iter().cloned())
                .collect(),
            fixtures: packages
                .iter()
                .flat_map(|package| package.fixtures.iter().cloned())
                .collect(),
            diagnostics: packages
                .iter()
                .flat_map(|package| package.diagnostics.iter().cloned())
                .collect(),
            migrations: packages
                .iter()
                .flat_map(|package| package.migrations.iter().cloned())
                .collect(),
            stories: packages
                .iter()
                .flat_map(|package| package.stories.iter().cloned())
                .collect(),
            route_requirements: packages
                .iter()
                .flat_map(|package| package.route_requirements.iter().cloned())
                .collect(),
            target_profiles: packages
                .iter()
                .flat_map(|package| package.target_profiles.iter().cloned())
                .collect(),
            packages,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlPackageRegistrySnapshot {
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

impl ControlPackageRegistrySnapshot {
    pub fn validate_contract(&self) -> ControlPackageValidationReport {
        let mut report = ControlPackageValidationReport::new();
        for package in &self.packages {
            report.extend(package.validate_contract());
        }
        report
    }
    pub fn package_ids(&self) -> Vec<&str> {
        self.packages
            .iter()
            .map(|package| package.package_id.as_str())
            .collect()
    }
    pub fn kernel_ids(&self) -> Vec<&str> {
        self.kernels
            .iter()
            .map(|kernel| kernel.kernel_id.as_str())
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ControlPackageRegistryError {
    InvalidPackage {
        report: ControlPackageValidationReport,
    },
    DuplicatePackage {
        package_id: String,
    },
    DuplicateControlKind {
        control_kind_id: String,
    },
}

impl fmt::Display for ControlPackageRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPackage { report } => write!(
                formatter,
                "control package rejected with {} diagnostics",
                report.diagnostics.len()
            ),
            Self::DuplicatePackage { package_id } => write!(
                formatter,
                "control package {package_id} is already registered"
            ),
            Self::DuplicateControlKind { control_kind_id } => write!(
                formatter,
                "control kind {control_kind_id} is already registered"
            ),
        }
    }
}

impl std::error::Error for ControlPackageRegistryError {}
