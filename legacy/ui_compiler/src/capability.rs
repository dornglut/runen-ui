//! Route and binding capability validation for UiProgram compilation.

use std::collections::{BTreeMap, BTreeSet};

use ui_artifacts::UiRuntimeArtifactDiagnostic;
use ui_program::{BindingEndpoint, RouteCapability, UiProgram};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityCheck {
    pub capability_id: String,
    pub subject: CapabilityCheckSubject,
    pub owner_control_ids: Vec<String>,
    pub status: CapabilityCheckStatus,
}

impl CapabilityCheck {
    pub fn from_program(program: &UiProgram) -> Vec<Self> {
        let control_capabilities = control_capability_index(program);
        let mut checks = Vec::new();

        for node in &program.graphs.control.nodes {
            for capability in &node.required_capabilities {
                checks.push(Self {
                    capability_id: capability.as_str().to_owned(),
                    subject: CapabilityCheckSubject::ControlNode {
                        id: node.node_id.as_str().to_owned(),
                    },
                    owner_control_ids: vec![node.node_id.as_str().to_owned()],
                    status: CapabilityCheckStatus::DeclaredByControl,
                });
            }
        }

        for handler in &program.graphs.interaction.handlers {
            for capability in &handler.required_capabilities {
                let control_id = handler.control_id.as_str();
                checks.push(Self {
                    capability_id: capability.as_str().to_owned(),
                    subject: CapabilityCheckSubject::InteractionHandler {
                        id: handler.handler_id.as_str().to_owned(),
                    },
                    owner_control_ids: vec![control_id.to_owned()],
                    status: capability_status(&control_capabilities, &[control_id], capability),
                });
            }
        }

        for binding in &program.graphs.binding.bindings {
            let owner_control_ids = binding_owner_controls(binding);
            for capability in &binding.required_capabilities {
                let owner_refs = owner_control_ids
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                checks.push(Self {
                    capability_id: capability.as_str().to_owned(),
                    subject: CapabilityCheckSubject::BindingEdge {
                        id: binding.edge_id.as_str().to_owned(),
                    },
                    owner_control_ids: owner_control_ids.iter().map(String::to_owned).collect(),
                    status: capability_status(&control_capabilities, &owner_refs, capability),
                });
            }
        }

        checks
    }

    pub fn is_satisfied(&self) -> bool {
        matches!(
            self.status,
            CapabilityCheckStatus::DeclaredByControl | CapabilityCheckStatus::SatisfiedByControl
        )
    }

    pub fn diagnostic(&self) -> Option<UiRuntimeArtifactDiagnostic> {
        match self.status {
            CapabilityCheckStatus::DeclaredByControl
            | CapabilityCheckStatus::SatisfiedByControl => None,
            CapabilityCheckStatus::MissingControl => Some(UiRuntimeArtifactDiagnostic::error(
                "ui.compiler.capability.missing_control",
                format!(
                    "capability {} on {} has no owning control",
                    self.capability_id,
                    self.subject.display_id()
                ),
            )),
            CapabilityCheckStatus::MissingControlDeclaration => {
                Some(UiRuntimeArtifactDiagnostic::error(
                    "ui.compiler.capability.missing_control_declaration",
                    format!(
                        "capability {} on {} is not declared by any owning control",
                        self.capability_id,
                        self.subject.display_id()
                    ),
                ))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CapabilityCheckSubject {
    ControlNode { id: String },
    InteractionHandler { id: String },
    BindingEdge { id: String },
}

impl CapabilityCheckSubject {
    fn display_id(&self) -> &str {
        match self {
            Self::ControlNode { id }
            | Self::InteractionHandler { id }
            | Self::BindingEdge { id } => id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityCheckStatus {
    DeclaredByControl,
    SatisfiedByControl,
    MissingControlDeclaration,
    MissingControl,
}

fn control_capability_index(program: &UiProgram) -> BTreeMap<String, BTreeSet<String>> {
    let mut index = BTreeMap::<String, BTreeSet<String>>::new();
    for node in &program.graphs.control.nodes {
        let capabilities = index.entry(node.node_id.as_str().to_owned()).or_default();
        for capability in &node.required_capabilities {
            capabilities.insert(capability.as_str().to_owned());
        }
    }
    index
}

fn capability_status(
    control_capabilities: &BTreeMap<String, BTreeSet<String>>,
    owner_control_ids: &[&str],
    capability: &RouteCapability,
) -> CapabilityCheckStatus {
    if owner_control_ids.is_empty() {
        return CapabilityCheckStatus::MissingControl;
    }
    let mut saw_existing_control = false;
    for control_id in owner_control_ids {
        if let Some(capabilities) = control_capabilities.get(*control_id) {
            saw_existing_control = true;
            if capabilities.contains(capability.as_str()) {
                return CapabilityCheckStatus::SatisfiedByControl;
            }
        }
    }
    if saw_existing_control {
        CapabilityCheckStatus::MissingControlDeclaration
    } else {
        CapabilityCheckStatus::MissingControl
    }
}

fn binding_owner_controls(binding: &ui_program::BindingEdge) -> Vec<String> {
    let mut owner_control_ids = Vec::new();
    push_endpoint_control(&mut owner_control_ids, &binding.source);
    push_endpoint_control(&mut owner_control_ids, &binding.target);
    owner_control_ids
}

fn push_endpoint_control(owner_control_ids: &mut Vec<String>, endpoint: &BindingEndpoint) {
    if let BindingEndpoint::ControlProperty { control_id, .. } = endpoint {
        insert_unique(owner_control_ids, control_id.as_str());
    }
}

fn insert_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_owned());
    }
}
