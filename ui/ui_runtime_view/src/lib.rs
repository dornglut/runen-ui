//! Canonical runtime read model over compiled UI artifact tables.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use ui_artifacts::{
    AccessibilityRow, BindingSnapshotRow, ControlPropertyRow, ControlTableRow, InspectionRow,
    InteractionDispatchRow, LayoutPlanRow, StateTableRow, StyleResolutionRow, UiRuntimeArtifact,
    UiRuntimeArtifactDiagnostic, UiRuntimeArtifactDiagnosticSeverity, VisualOperatorRow,
};
use ui_program::{BindingEndpoint, ControlNodeId, StateRequirementId};
use ui_schema::UiSchemaValue;

pub const DIAGNOSTIC_CONTROL_MISSING_PROPERTY_SNAPSHOT: &str =
    "ui.runtime_view.control.missing_property_snapshot";
pub const DIAGNOSTIC_CONTROL_DUPLICATE_PROPERTY_SNAPSHOTS: &str =
    "ui.runtime_view.control.duplicate_property_snapshots";
pub const DIAGNOSTIC_PROPERTY_MISSING_OWNER_CONTROL: &str =
    "ui.runtime_view.property.missing_owner_control";
pub const DIAGNOSTIC_LAYOUT_MISSING_OWNER_CONTROL: &str =
    "ui.runtime_view.layout.missing_owner_control";
pub const DIAGNOSTIC_STYLE_MISSING_OWNER_CONTROL: &str =
    "ui.runtime_view.style.missing_owner_control";
pub const DIAGNOSTIC_STATE_MISSING_OWNER_CONTROL: &str =
    "ui.runtime_view.state.missing_owner_control";
pub const DIAGNOSTIC_INTERACTION_MISSING_OWNER_CONTROL: &str =
    "ui.runtime_view.interaction.missing_owner_control";
pub const DIAGNOSTIC_VISUAL_MISSING_OWNER_CONTROL: &str =
    "ui.runtime_view.visual.missing_owner_control";
pub const DIAGNOSTIC_ACCESSIBILITY_MISSING_OWNER_CONTROL: &str =
    "ui.runtime_view.accessibility.missing_owner_control";
pub const DIAGNOSTIC_INSPECTION_MISSING_OWNER_CONTROL: &str =
    "ui.runtime_view.inspection.missing_owner_control";
pub const DIAGNOSTIC_BINDING_MISSING_STATE_REQUIREMENT: &str =
    "ui.runtime_view.binding.missing_state_requirement";
pub const DIAGNOSTIC_BINDING_MISSING_CONTROL_PROPERTY_OWNER: &str =
    "ui.runtime_view.binding.missing_control_property_owner";
pub const DIAGNOSTIC_BUTTON_RUNTIME_VIEW_FAILED: &str =
    "ui.runtime_view.button.runtime_view_failed";
pub const DIAGNOSTIC_BUTTON_UNSUPPORTED_CONTROL_KIND: &str =
    "ui.runtime_view.button.unsupported_control_kind";
pub const DIAGNOSTIC_BUTTON_MISSING_PROPERTY: &str = "ui.runtime_view.button.missing_property";
pub const DIAGNOSTIC_BUTTON_MISSING_LABEL: &str = "ui.runtime_view.button.missing_label";
pub const DIAGNOSTIC_BUTTON_SELECTED_BINDING_MISSING_HOST_VALUE: &str =
    "ui.runtime_view.button.selected_binding_missing_host_value";
pub const DIAGNOSTIC_BUTTON_SELECTED_BINDING_NON_BOOL_HOST_VALUE: &str =
    "ui.runtime_view.button.selected_binding_non_bool_host_value";

const BUTTON_CONTROL_KIND_ID: &str = "runenwerk.ui.controls.button";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRuntimeView {
    pub controls: Vec<RuntimeControlView>,
    pub diagnostics: Vec<UiRuntimeViewDiagnostic>,
    #[serde(default)]
    pub artifact_diagnostics: Vec<UiRuntimeArtifactDiagnostic>,
}

impl UiRuntimeView {
    pub fn from_artifact(artifact: &UiRuntimeArtifact) -> Self {
        Self::from_artifact_report(artifact).view
    }

    pub fn from_artifact_report(artifact: &UiRuntimeArtifact) -> UiRuntimeViewReport {
        let tables = &artifact.tables;
        let control_ids = tables
            .controls
            .rows
            .iter()
            .map(|row| row.node.node_id.as_str().to_owned())
            .collect::<HashSet<_>>();

        let mut diagnostics = Vec::new();
        diagnostics.extend(control_property_diagnostics(artifact, &control_ids));
        diagnostics.extend(owner_diagnostics(artifact, &control_ids));
        diagnostics.extend(binding_diagnostics(artifact, &control_ids));

        let controls = tables
            .controls
            .rows
            .iter()
            .map(|control| RuntimeControlView {
                control: control.clone(),
                property: property_row_for_control(artifact, &control.node.node_id).cloned(),
                layout: tables
                    .layout
                    .rows
                    .iter()
                    .filter(|row| row.constraint.target_control == control.node.node_id)
                    .cloned()
                    .collect(),
                style: tables
                    .style
                    .rows
                    .iter()
                    .filter(|row| row.rule.target_control == control.node.node_id)
                    .cloned()
                    .collect(),
                state: tables
                    .state
                    .rows
                    .iter()
                    .filter(|row| row.requirement.owner_control == control.node.node_id)
                    .cloned()
                    .collect(),
                interaction: tables
                    .interaction
                    .rows
                    .iter()
                    .filter(|row| row.handler.control_id == control.node.node_id)
                    .cloned()
                    .collect(),
                binding_snapshots: tables
                    .binding_snapshots
                    .rows
                    .iter()
                    .filter(|row| binding_belongs_to_control(row, &control.node.node_id, artifact))
                    .cloned()
                    .collect(),
                visual: tables
                    .visual
                    .rows
                    .iter()
                    .filter(|row| row.operator.control_id == control.node.node_id)
                    .cloned()
                    .collect(),
                accessibility: tables
                    .accessibility
                    .rows
                    .iter()
                    .filter(|row| row.node.control_id == control.node.node_id)
                    .cloned()
                    .collect(),
                inspection: tables
                    .inspection
                    .rows
                    .iter()
                    .filter(|row| row.entry.control_id == control.node.node_id)
                    .cloned()
                    .collect(),
            })
            .collect();

        UiRuntimeViewReport {
            artifact_diagnostics: artifact.manifest.diagnostics.clone(),
            view: UiRuntimeView {
                controls,
                diagnostics,
                artifact_diagnostics: artifact.manifest.diagnostics.clone(),
            },
        }
    }

    pub fn control(&self, control_id: &ControlNodeId) -> Option<&RuntimeControlView> {
        self.controls
            .iter()
            .find(|control| &control.control.node.node_id == control_id)
    }

    pub fn controls(&self) -> impl Iterator<Item = &RuntimeControlView> {
        self.controls.iter()
    }

    pub fn passed(&self) -> bool {
        artifact_diagnostics_passed(&self.artifact_diagnostics)
            && self
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.severity != UiRuntimeViewDiagnosticSeverity::Error)
    }

    pub fn view_passed(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != UiRuntimeViewDiagnosticSeverity::Error)
    }

    pub fn artifact_passed(&self) -> bool {
        artifact_diagnostics_passed(&self.artifact_diagnostics)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeControlView {
    pub control: ControlTableRow,
    #[serde(default)]
    pub property: Option<ControlPropertyRow>,
    #[serde(default)]
    pub layout: Vec<LayoutPlanRow>,
    #[serde(default)]
    pub style: Vec<StyleResolutionRow>,
    #[serde(default)]
    pub state: Vec<StateTableRow>,
    #[serde(default)]
    pub interaction: Vec<InteractionDispatchRow>,
    #[serde(default)]
    pub binding_snapshots: Vec<BindingSnapshotRow>,
    #[serde(default)]
    pub visual: Vec<VisualOperatorRow>,
    #[serde(default)]
    pub accessibility: Vec<AccessibilityRow>,
    #[serde(default)]
    pub inspection: Vec<InspectionRow>,
}

impl RuntimeControlView {
    pub fn control_id(&self) -> &ControlNodeId {
        &self.control.node.node_id
    }

    pub fn property(&self) -> Option<&ControlPropertyRow> {
        self.property.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRuntimeViewReport {
    pub view: UiRuntimeView,
    #[serde(default)]
    pub artifact_diagnostics: Vec<UiRuntimeArtifactDiagnostic>,
}

impl UiRuntimeViewReport {
    pub fn passed(&self) -> bool {
        self.artifact_passed() && self.view_passed()
    }

    pub fn artifact_passed(&self) -> bool {
        artifact_diagnostics_passed(&self.artifact_diagnostics)
    }

    pub fn view_passed(&self) -> bool {
        self.view.view_passed()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRuntimeViewDiagnostic {
    pub code: String,
    pub message: String,
    pub severity: UiRuntimeViewDiagnosticSeverity,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}

impl UiRuntimeViewDiagnostic {
    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        source_map_index: Option<u32>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: UiRuntimeViewDiagnosticSeverity::Error,
            source_map_index,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiRuntimeViewDiagnosticSeverity {
    Info,
    Warning,
    #[default]
    Error,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ButtonRuntimeViewReport {
    pub buttons: Vec<ButtonRuntimeView>,
    pub diagnostics: Vec<ButtonRuntimeViewDiagnostic>,
}

impl ButtonRuntimeViewReport {
    pub fn from_runtime_view_report(report: &UiRuntimeViewReport) -> Self {
        Self::from_runtime_view_report_with_host_data(report, &ButtonRuntimeHostData::default())
    }

    pub fn from_runtime_view_report_with_host_data(
        report: &UiRuntimeViewReport,
        host_data: &ButtonRuntimeHostData,
    ) -> Self {
        if !report.passed() {
            return Self {
                buttons: Vec::new(),
                diagnostics: vec![ButtonRuntimeViewDiagnostic::error(
                    DIAGNOSTIC_BUTTON_RUNTIME_VIEW_FAILED,
                    "button runtime view refused failed runtime view report",
                    None,
                )],
            };
        }

        let mut buttons = Vec::new();
        let mut diagnostics = Vec::new();

        for control in report.view.controls() {
            if control.control.node.control_kind.as_str() != BUTTON_CONTROL_KIND_ID {
                diagnostics.push(ButtonRuntimeViewDiagnostic::warning(
                    DIAGNOSTIC_BUTTON_UNSUPPORTED_CONTROL_KIND,
                    format!(
                        "control {} uses unsupported control kind {}",
                        control.control_id().as_str(),
                        control.control.node.control_kind.as_str()
                    ),
                    control.control.source_map_index,
                ));
                continue;
            }

            let Some(property) = control.property() else {
                diagnostics.push(ButtonRuntimeViewDiagnostic::error(
                    DIAGNOSTIC_BUTTON_MISSING_PROPERTY,
                    format!(
                        "button control {} is missing a property snapshot",
                        control.control_id().as_str()
                    ),
                    control.control.source_map_index,
                ));
                continue;
            };

            let Some(label) = property
                .snapshot
                .get("label")
                .and_then(UiSchemaValue::as_str)
            else {
                diagnostics.push(ButtonRuntimeViewDiagnostic::error(
                    DIAGNOSTIC_BUTTON_MISSING_LABEL,
                    format!(
                        "button control {} is missing string property label",
                        control.control_id().as_str()
                    ),
                    property
                        .source_map_index
                        .or(control.control.source_map_index),
                ));
                continue;
            };

            let selected_binding = host_binding_for_state(control, "selected");
            let (selected, selected_diagnostic) =
                resolve_bound_bool_state(control, selected_binding.as_ref(), host_data, "selected");
            if let Some(diagnostic) = selected_diagnostic {
                diagnostics.push(diagnostic);
            }

            buttons.push(ButtonRuntimeView {
                control_id: control.control_id().as_str().to_owned(),
                label: label.to_owned(),
                route: control
                    .interaction
                    .first()
                    .map(|row| row.handler.route.as_str().to_owned()),
                capability: control
                    .interaction
                    .first()
                    .and_then(|row| row.handler.required_capabilities.first())
                    .map(|capability| capability.as_str().to_owned())
                    .or_else(|| {
                        control
                            .control
                            .node
                            .required_capabilities
                            .first()
                            .map(|capability| capability.as_str().to_owned())
                    }),
                selected,
                selected_host_endpoint: selected_binding,
                disabled: property
                    .snapshot
                    .get("disabled")
                    .and_then(UiSchemaValue::as_bool)
                    .unwrap_or(false),
                accessibility_label: control
                    .accessibility
                    .first()
                    .and_then(|row| row.node.label.clone()),
                style_axes: ButtonStyleAxes {
                    variant: optional_string_property_or(control, "variant", "secondary")
                        .to_owned(),
                    tone: optional_string_property_or(control, "tone", "neutral").to_owned(),
                    density: optional_string_property_or(control, "density", "normal").to_owned(),
                    size: optional_string_property_or(control, "size", "md").to_owned(),
                },
                source_map_indexes: ButtonRuntimeSourceMapIndexes {
                    control: control.control.source_map_index,
                    property: property.source_map_index,
                    style: control.style.first().and_then(|row| row.source_map_index),
                    state: control.state.first().and_then(|row| row.source_map_index),
                    interaction: control
                        .interaction
                        .first()
                        .and_then(|row| row.source_map_index),
                    accessibility: control
                        .accessibility
                        .first()
                        .and_then(|row| row.source_map_index),
                },
            });
        }

        Self {
            buttons,
            diagnostics,
        }
    }

    pub fn passed(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != ButtonRuntimeViewDiagnosticSeverity::Error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonRuntimeView {
    pub control_id: String,
    pub label: String,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub capability: Option<String>,
    pub selected: bool,
    #[serde(default)]
    pub selected_host_endpoint: Option<String>,
    pub disabled: bool,
    #[serde(default)]
    pub accessibility_label: Option<String>,
    pub style_axes: ButtonStyleAxes,
    pub source_map_indexes: ButtonRuntimeSourceMapIndexes,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonStyleAxes {
    pub variant: String,
    pub tone: String,
    pub density: String,
    pub size: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonRuntimeSourceMapIndexes {
    #[serde(default)]
    pub control: Option<u32>,
    #[serde(default)]
    pub property: Option<u32>,
    #[serde(default)]
    pub style: Option<u32>,
    #[serde(default)]
    pub state: Option<u32>,
    #[serde(default)]
    pub interaction: Option<u32>,
    #[serde(default)]
    pub accessibility: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ButtonRuntimeHostData {
    values: BTreeMap<String, UiSchemaValue>,
}

impl ButtonRuntimeHostData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_bool(mut self, endpoint_id: impl Into<String>, value: bool) -> Self {
        self.values
            .insert(endpoint_id.into(), UiSchemaValue::bool(value));
        self
    }

    pub fn value(&self, endpoint_id: &str) -> Option<&UiSchemaValue> {
        self.values.get(endpoint_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonRuntimeViewDiagnostic {
    pub code: String,
    pub message: String,
    pub severity: ButtonRuntimeViewDiagnosticSeverity,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}

impl ButtonRuntimeViewDiagnostic {
    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        source_map_index: Option<u32>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: ButtonRuntimeViewDiagnosticSeverity::Error,
            source_map_index,
        }
    }

    pub fn warning(
        code: impl Into<String>,
        message: impl Into<String>,
        source_map_index: Option<u32>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: ButtonRuntimeViewDiagnosticSeverity::Warning,
            source_map_index,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonRuntimeViewDiagnosticSeverity {
    Info,
    Warning,
    #[default]
    Error,
}

fn artifact_diagnostics_passed(diagnostics: &[UiRuntimeArtifactDiagnostic]) -> bool {
    diagnostics
        .iter()
        .all(|diagnostic| diagnostic.severity != UiRuntimeArtifactDiagnosticSeverity::Error)
}

fn control_property_diagnostics(
    artifact: &UiRuntimeArtifact,
    control_ids: &HashSet<String>,
) -> Vec<UiRuntimeViewDiagnostic> {
    let mut diagnostics = Vec::new();
    for control in &artifact.tables.controls.rows {
        let rows = artifact
            .tables
            .properties
            .rows
            .iter()
            .filter(|row| row.snapshot.owner_control == control.node.node_id)
            .collect::<Vec<_>>();
        match rows.len() {
            0 => diagnostics.push(UiRuntimeViewDiagnostic::error(
                DIAGNOSTIC_CONTROL_MISSING_PROPERTY_SNAPSHOT,
                format!(
                    "control {} is missing a property snapshot",
                    control.node.node_id.as_str()
                ),
                control.source_map_index,
            )),
            1 => {}
            _ => diagnostics.push(UiRuntimeViewDiagnostic::error(
                DIAGNOSTIC_CONTROL_DUPLICATE_PROPERTY_SNAPSHOTS,
                format!(
                    "control {} has duplicate property snapshots: {}",
                    control.node.node_id.as_str(),
                    rows.iter()
                        .map(|row| row.snapshot.snapshot_id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                rows.first().and_then(|row| row.source_map_index),
            )),
        }
    }

    for property in &artifact.tables.properties.rows {
        if !control_ids.contains(property.snapshot.owner_control.as_str()) {
            diagnostics.push(UiRuntimeViewDiagnostic::error(
                DIAGNOSTIC_PROPERTY_MISSING_OWNER_CONTROL,
                format!(
                    "property snapshot {} references missing owner control {}",
                    property.snapshot.snapshot_id.as_str(),
                    property.snapshot.owner_control.as_str()
                ),
                property.source_map_index,
            ));
        }
    }

    diagnostics
}

fn owner_diagnostics(
    artifact: &UiRuntimeArtifact,
    control_ids: &HashSet<String>,
) -> Vec<UiRuntimeViewDiagnostic> {
    let mut diagnostics = Vec::new();

    for row in &artifact.tables.layout.rows {
        if !control_ids.contains(row.constraint.target_control.as_str()) {
            diagnostics.push(missing_owner_diagnostic(
                DIAGNOSTIC_LAYOUT_MISSING_OWNER_CONTROL,
                "layout row",
                row.constraint.constraint_id.as_str(),
                row.constraint.target_control.as_str(),
                row.source_map_index,
            ));
        }
    }
    for row in &artifact.tables.style.rows {
        if !control_ids.contains(row.rule.target_control.as_str()) {
            diagnostics.push(missing_owner_diagnostic(
                DIAGNOSTIC_STYLE_MISSING_OWNER_CONTROL,
                "style row",
                row.rule.rule_id.as_str(),
                row.rule.target_control.as_str(),
                row.source_map_index,
            ));
        }
    }
    for row in &artifact.tables.state.rows {
        if !control_ids.contains(row.requirement.owner_control.as_str()) {
            diagnostics.push(missing_owner_diagnostic(
                DIAGNOSTIC_STATE_MISSING_OWNER_CONTROL,
                "state requirement",
                row.requirement.requirement_id.as_str(),
                row.requirement.owner_control.as_str(),
                row.source_map_index,
            ));
        }
    }
    for row in &artifact.tables.interaction.rows {
        if !control_ids.contains(row.handler.control_id.as_str()) {
            diagnostics.push(missing_owner_diagnostic(
                DIAGNOSTIC_INTERACTION_MISSING_OWNER_CONTROL,
                "interaction handler",
                row.handler.handler_id.as_str(),
                row.handler.control_id.as_str(),
                row.source_map_index,
            ));
        }
    }
    for row in &artifact.tables.visual.rows {
        if !control_ids.contains(row.operator.control_id.as_str()) {
            diagnostics.push(missing_owner_diagnostic(
                DIAGNOSTIC_VISUAL_MISSING_OWNER_CONTROL,
                "visual operator",
                row.operator.operator_id.as_str(),
                row.operator.control_id.as_str(),
                row.source_map_index,
            ));
        }
    }
    for row in &artifact.tables.accessibility.rows {
        if !control_ids.contains(row.node.control_id.as_str()) {
            diagnostics.push(missing_owner_diagnostic(
                DIAGNOSTIC_ACCESSIBILITY_MISSING_OWNER_CONTROL,
                "accessibility node",
                row.node.node_id.as_str(),
                row.node.control_id.as_str(),
                row.source_map_index,
            ));
        }
    }
    for row in &artifact.tables.inspection.rows {
        if !control_ids.contains(row.entry.control_id.as_str()) {
            diagnostics.push(missing_owner_diagnostic(
                DIAGNOSTIC_INSPECTION_MISSING_OWNER_CONTROL,
                "inspection entry",
                row.entry.entry_id.as_str(),
                row.entry.control_id.as_str(),
                row.source_map_index,
            ));
        }
    }

    diagnostics
}

fn binding_diagnostics(
    artifact: &UiRuntimeArtifact,
    control_ids: &HashSet<String>,
) -> Vec<UiRuntimeViewDiagnostic> {
    let state_ids = artifact
        .tables
        .state
        .rows
        .iter()
        .map(|row| row.requirement.requirement_id.as_str().to_owned())
        .collect::<HashSet<_>>();
    let mut diagnostics = Vec::new();

    for row in &artifact.tables.binding_snapshots.rows {
        for endpoint in [&row.binding.source, &row.binding.target] {
            match endpoint {
                BindingEndpoint::ControlProperty { control_id, .. } => {
                    if !control_ids.contains(control_id.as_str()) {
                        diagnostics.push(UiRuntimeViewDiagnostic::error(
                            DIAGNOSTIC_BINDING_MISSING_CONTROL_PROPERTY_OWNER,
                            format!(
                                "binding {} references missing control property owner {}",
                                row.binding.edge_id.as_str(),
                                control_id.as_str()
                            ),
                            row.source_map_index,
                        ));
                    }
                }
                BindingEndpoint::UiState { requirement_id, .. } => {
                    if !state_ids.contains(requirement_id.as_str()) {
                        diagnostics.push(UiRuntimeViewDiagnostic::error(
                            DIAGNOSTIC_BINDING_MISSING_STATE_REQUIREMENT,
                            format!(
                                "binding {} references missing state requirement {}",
                                row.binding.edge_id.as_str(),
                                requirement_id.as_str()
                            ),
                            row.source_map_index,
                        ));
                    }
                }
                BindingEndpoint::HostData { .. } => {}
            }
        }
    }

    diagnostics
}

fn missing_owner_diagnostic(
    code: &'static str,
    row_kind: &'static str,
    row_id: &str,
    control_id: &str,
    source_map_index: Option<u32>,
) -> UiRuntimeViewDiagnostic {
    UiRuntimeViewDiagnostic::error(
        code,
        format!("{row_kind} {row_id} references missing owner control {control_id}"),
        source_map_index,
    )
}

fn property_row_for_control<'a>(
    artifact: &'a UiRuntimeArtifact,
    control_id: &ControlNodeId,
) -> Option<&'a ControlPropertyRow> {
    artifact
        .tables
        .properties
        .rows
        .iter()
        .find(|row| &row.snapshot.owner_control == control_id)
}

fn state_owner_control<'a>(
    artifact: &'a UiRuntimeArtifact,
    requirement_id: &StateRequirementId,
) -> Option<&'a ControlNodeId> {
    artifact
        .tables
        .state
        .rows
        .iter()
        .find(|row| &row.requirement.requirement_id == requirement_id)
        .map(|row| &row.requirement.owner_control)
}

fn binding_belongs_to_control(
    row: &BindingSnapshotRow,
    control_id: &ControlNodeId,
    artifact: &UiRuntimeArtifact,
) -> bool {
    endpoint_belongs_to_control(&row.binding.source, control_id, artifact)
        || endpoint_belongs_to_control(&row.binding.target, control_id, artifact)
}

fn optional_string_property_or<'a>(
    control: &'a RuntimeControlView,
    property_name: &str,
    fallback: &'a str,
) -> &'a str {
    control
        .property()
        .and_then(|property| property.snapshot.get(property_name))
        .and_then(UiSchemaValue::as_str)
        .unwrap_or(fallback)
}

fn host_binding_for_state(control: &RuntimeControlView, state_name: &str) -> Option<String> {
    control.binding_snapshots.iter().find_map(|row| {
        let BindingEndpoint::HostData { endpoint_id } = &row.binding.source else {
            return None;
        };
        let BindingEndpoint::UiState { requirement_id, .. } = &row.binding.target else {
            return None;
        };

        requirement_id
            .as_str()
            .rsplit('.')
            .next()
            .filter(|name| *name == state_name)
            .map(|_| endpoint_id.as_str().to_owned())
    })
}

fn resolve_bound_bool_state(
    control: &RuntimeControlView,
    endpoint_id: Option<&String>,
    host_data: &ButtonRuntimeHostData,
    state_name: &str,
) -> (bool, Option<ButtonRuntimeViewDiagnostic>) {
    let Some(endpoint_id) = endpoint_id else {
        return (false, None);
    };

    let Some(value) = host_data.value(endpoint_id) else {
        return (
            false,
            Some(ButtonRuntimeViewDiagnostic::warning(
                DIAGNOSTIC_BUTTON_SELECTED_BINDING_MISSING_HOST_VALUE,
                format!(
                    "button control {} has {state_name} binding {} without a supplied host value",
                    control.control_id().as_str(),
                    endpoint_id
                ),
                control
                    .binding_snapshots
                    .first()
                    .and_then(|row| row.source_map_index),
            )),
        );
    };

    match value.as_bool() {
        Some(value) => (value, None),
        None => (
            false,
            Some(ButtonRuntimeViewDiagnostic::error(
                DIAGNOSTIC_BUTTON_SELECTED_BINDING_NON_BOOL_HOST_VALUE,
                format!(
                    "button control {} expected bool host value for {state_name} binding {}",
                    control.control_id().as_str(),
                    endpoint_id
                ),
                control
                    .binding_snapshots
                    .first()
                    .and_then(|row| row.source_map_index),
            )),
        ),
    }
}

fn endpoint_belongs_to_control(
    endpoint: &BindingEndpoint,
    control_id: &ControlNodeId,
    artifact: &UiRuntimeArtifact,
) -> bool {
    match endpoint {
        BindingEndpoint::ControlProperty {
            control_id: endpoint_control_id,
            ..
        } => endpoint_control_id == control_id,
        BindingEndpoint::UiState { requirement_id, .. } => {
            state_owner_control(artifact, requirement_id) == Some(control_id)
        }
        BindingEndpoint::HostData { .. } => false,
    }
}
