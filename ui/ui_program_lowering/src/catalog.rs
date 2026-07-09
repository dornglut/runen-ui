//! File: domain/ui/ui_program_lowering/src/catalog.rs
//! Crate: ui_program_lowering
//!
//! Control package metadata adapted into semantic UiProgram formation contracts.

use std::collections::BTreeMap;

use ui_controls::{
    ControlKernelDescriptor, ControlKernelId, ControlKernelKind, ControlPackageDescriptor,
    ControlSchemaDescriptor,
};
use ui_program::{ControlKernelRef, RouteCapability, UiProgramDiagnostic, UiSchemaRef};
use ui_schema::UiSchema;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiProgramFormationControlCatalog {
    control_kinds: BTreeMap<String, UiProgramFormationControlContract>,
}

impl UiProgramFormationControlCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_control_kind(mut self, contract: UiProgramFormationControlContract) -> Self {
        self.control_kinds
            .insert(contract.kind_id.clone(), contract);
        self
    }

    pub fn control_kind(&self, kind_id: &str) -> Option<&UiProgramFormationControlContract> {
        self.control_kinds.get(kind_id)
    }

    pub fn contains_control_kind(&self, kind_id: &str) -> bool {
        self.control_kinds.contains_key(kind_id)
    }

    pub fn derive_from_control_package_registry_snapshot(
        snapshot: &ui_controls::ControlPackageRegistrySnapshot,
    ) -> UiProgramFormationCatalogReport {
        let mut catalog = Self::new();
        let mut diagnostics = Vec::new();
        let mut skipped_control_kinds = Vec::new();

        for package in &snapshot.packages {
            for kind in &package.control_kinds {
                let package_id = package.package_id.as_str();
                let control_kind_id = kind.control_kind_id.as_str();
                let mut kind_diagnostics = Vec::new();

                let activation_capability = if let Some(capability) =
                    kind.required_capabilities.first().cloned()
                {
                    Some(capability)
                } else {
                    kind_diagnostics.push(UiProgramDiagnostic::new(
                        "ui.program.catalog.control_kind_missing_activation_capability",
                        format!(
                            "package {package_id} control kind {control_kind_id} contract part activation_capability has no required capability; expected at least one route capability"
                        ),
                    ));
                    None
                };

                let property_schema = validate_schema_ref(
                    package,
                    control_kind_id,
                    "property_schema",
                    &kind.property_schema,
                    "Properties",
                    "ui.program.catalog.missing_property_schema",
                    &package.property_schemas,
                    &mut kind_diagnostics,
                );
                validate_schema_ref(
                    package,
                    control_kind_id,
                    "state_schema",
                    &kind.state_schema,
                    "State",
                    "ui.program.catalog.missing_state_schema",
                    &package.state_schemas,
                    &mut kind_diagnostics,
                );
                validate_schema_ref(
                    package,
                    control_kind_id,
                    "event_payload_schema",
                    &kind.event_payload_schema,
                    "EventPayload",
                    "ui.program.catalog.missing_event_payload_schema",
                    &package.event_payload_schemas,
                    &mut kind_diagnostics,
                );

                validate_kernel_ref(
                    package,
                    control_kind_id,
                    "layout_kernel",
                    &kind.kernels.layout,
                    ControlKernelKind::Layout,
                    "ui.program.catalog.missing_layout_kernel",
                    &mut kind_diagnostics,
                );
                validate_kernel_ref(
                    package,
                    control_kind_id,
                    "interaction_kernel",
                    &kind.kernels.interaction,
                    ControlKernelKind::Interaction,
                    "ui.program.catalog.missing_interaction_kernel",
                    &mut kind_diagnostics,
                );
                validate_kernel_ref(
                    package,
                    control_kind_id,
                    "visual_kernel",
                    &kind.kernels.visual,
                    ControlKernelKind::Visual,
                    "ui.program.catalog.missing_visual_kernel",
                    &mut kind_diagnostics,
                );
                validate_kernel_ref(
                    package,
                    control_kind_id,
                    "accessibility_kernel",
                    &kind.kernels.accessibility,
                    ControlKernelKind::Accessibility,
                    "ui.program.catalog.missing_accessibility_kernel",
                    &mut kind_diagnostics,
                );
                validate_kernel_ref(
                    package,
                    control_kind_id,
                    "inspection_kernel",
                    &kind.kernels.inspection,
                    ControlKernelKind::Inspection,
                    "ui.program.catalog.missing_inspection_kernel",
                    &mut kind_diagnostics,
                );

                let Some(activation_capability) = activation_capability else {
                    diagnostics.extend(kind_diagnostics);
                    skipped_control_kinds.push(control_kind_id.to_owned());
                    continue;
                };

                if !kind_diagnostics.is_empty() {
                    diagnostics.extend(kind_diagnostics);
                    skipped_control_kinds.push(control_kind_id.to_owned());
                    continue;
                }

                catalog = catalog.with_control_kind(UiProgramFormationControlContract::new(
                    control_kind_id,
                    package_id,
                    kind.display_name.clone(),
                    property_schema.expect("property schema was validated before catalog insert"),
                    kind.state_schema.clone(),
                    kind.event_payload_schema.clone(),
                    ControlKernelRef::new(kind.kernels.layout.as_str()),
                    ControlKernelRef::new(kind.kernels.visual.as_str()),
                    activation_capability,
                ));
            }
        }

        UiProgramFormationCatalogReport {
            catalog,
            diagnostics,
            skipped_control_kinds,
        }
    }
}

fn validate_schema_ref(
    package: &ControlPackageDescriptor,
    control_kind_id: &str,
    contract_part: &str,
    expected_schema: &UiSchemaRef,
    expected_role: &str,
    diagnostic_code: &'static str,
    schemas: &[ControlSchemaDescriptor],
    diagnostics: &mut Vec<UiProgramDiagnostic>,
) -> Option<UiSchema> {
    let schema = schemas
        .iter()
        .find(|schema| schema.schema_ref() == expected_schema);

    if let Some(schema) = schema {
        return Some(schema.schema.clone());
    }

    diagnostics.push(UiProgramDiagnostic::new(
        diagnostic_code,
        format!(
            "package {} control kind {} contract part {} references schema {}@{}; expected schema role {}",
            package.package_id.as_str(),
            control_kind_id,
            contract_part,
            expected_schema.id.as_str(),
            expected_schema.version.value(),
            expected_role
        ),
    ));
    None
}

fn validate_kernel_ref(
    package: &ControlPackageDescriptor,
    control_kind_id: &str,
    contract_part: &str,
    expected_kernel: &ControlKernelId,
    expected_kind: ControlKernelKind,
    missing_diagnostic_code: &'static str,
    diagnostics: &mut Vec<UiProgramDiagnostic>,
) {
    let Some(kernel) = package
        .kernels
        .iter()
        .find(|kernel| &kernel.kernel_id == expected_kernel)
    else {
        diagnostics.push(UiProgramDiagnostic::new(
            missing_diagnostic_code,
            format!(
                "package {} control kind {} contract part {} references kernel {}; expected kernel kind {:?}",
                package.package_id.as_str(),
                control_kind_id,
                contract_part,
                expected_kernel.as_str(),
                expected_kind
            ),
        ));
        return;
    };

    validate_kernel_kind(
        package,
        control_kind_id,
        contract_part,
        kernel,
        expected_kind,
        diagnostics,
    );
}

fn validate_kernel_kind(
    package: &ControlPackageDescriptor,
    control_kind_id: &str,
    contract_part: &str,
    kernel: &ControlKernelDescriptor,
    expected_kind: ControlKernelKind,
    diagnostics: &mut Vec<UiProgramDiagnostic>,
) {
    if kernel.kind == expected_kind {
        return;
    }

    diagnostics.push(UiProgramDiagnostic::new(
        "ui.program.catalog.kernel_kind_mismatch",
        format!(
            "package {} control kind {} contract part {} references kernel {}; expected kernel kind {:?}, found {:?}",
            package.package_id.as_str(),
            control_kind_id,
            contract_part,
            kernel.kernel_id.as_str(),
            expected_kind,
            kernel.kind
        ),
    ));
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiProgramFormationControlContract {
    pub kind_id: String,
    pub package_id: String,
    pub display_name: String,
    pub property_schema: UiSchema,
    pub state_schema: UiSchemaRef,
    pub event_payload_schema: UiSchemaRef,
    pub layout_kernel: ControlKernelRef,
    pub visual_kernel: ControlKernelRef,
    pub activation_capability: RouteCapability,
}

impl UiProgramFormationControlContract {
    pub fn new(
        kind_id: impl Into<String>,
        package_id: impl Into<String>,
        display_name: impl Into<String>,
        property_schema: UiSchema,
        state_schema: UiSchemaRef,
        event_payload_schema: UiSchemaRef,
        layout_kernel: ControlKernelRef,
        visual_kernel: ControlKernelRef,
        activation_capability: RouteCapability,
    ) -> Self {
        Self {
            kind_id: kind_id.into(),
            package_id: package_id.into(),
            display_name: display_name.into(),
            event_payload_schema,
            layout_kernel,
            state_schema,
            activation_capability,
            property_schema,
            visual_kernel,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiProgramFormationCatalogReport {
    pub catalog: UiProgramFormationControlCatalog,
    pub diagnostics: Vec<UiProgramDiagnostic>,
    pub skipped_control_kinds: Vec<String>,
}

impl UiProgramFormationCatalogReport {
    pub fn passed(&self) -> bool {
        self.diagnostics.is_empty() && self.skipped_control_kinds.is_empty()
    }

    pub fn has_diagnostic(&self, code: &str) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code)
    }
}
