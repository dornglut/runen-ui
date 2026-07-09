//! Route-event to typed action resolution for the proof bridge.

use serde::{Deserialize, Serialize};
use ui_program::{RouteCapability, RouteId, RouteSchemaVersion, UiEventPacket};
use ui_schema::UiSchemaRef;

use crate::ids::{UiAppActionId, UiAppRouteBindingId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppRouteBinding {
    pub binding_id: UiAppRouteBindingId,
    pub route: RouteId,
    pub schema_version: RouteSchemaVersion,
    pub payload_schema: UiSchemaRef,
    pub action_id: UiAppActionId,
    pub capability: RouteCapability,
}

impl UiAppRouteBinding {
    pub fn new(
        binding_id: UiAppRouteBindingId,
        route: RouteId,
        schema_version: RouteSchemaVersion,
        payload_schema: UiSchemaRef,
        action_id: UiAppActionId,
        capability: RouteCapability,
    ) -> Self {
        Self {
            binding_id,
            route,
            schema_version,
            payload_schema,
            action_id,
            capability,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiAppRouteResolutionStatus {
    Resolved,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiAppRouteResolutionDiagnostic {
    UnknownRoute {
        route: String,
    },
    WrongSchemaVersion {
        route: String,
        expected: u32,
        actual: u32,
    },
    MissingCapability {
        route: String,
        capability: String,
    },
    PayloadSchemaMismatch {
        route: String,
        expected: String,
        actual: String,
    },
    PayloadDiagnostic {
        route: String,
    },
    RouteDiagnostic {
        route: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppResolvedAction {
    pub action_id: UiAppActionId,
    pub route: RouteId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppRouteResolution {
    pub status: UiAppRouteResolutionStatus,
    pub action: Option<UiAppResolvedAction>,
    #[serde(default)]
    pub diagnostics: Vec<UiAppRouteResolutionDiagnostic>,
}

impl UiAppRouteResolution {
    pub fn resolved(action: UiAppResolvedAction) -> Self {
        Self {
            status: UiAppRouteResolutionStatus::Resolved,
            action: Some(action),
            diagnostics: Vec::new(),
        }
    }

    pub fn rejected(diagnostic: UiAppRouteResolutionDiagnostic) -> Self {
        Self {
            status: UiAppRouteResolutionStatus::Rejected,
            action: None,
            diagnostics: vec![diagnostic],
        }
    }

    pub fn is_resolved(&self) -> bool {
        self.status == UiAppRouteResolutionStatus::Resolved
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppRouteBridge {
    bindings: Vec<UiAppRouteBinding>,
}

impl UiAppRouteBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_binding(mut self, binding: UiAppRouteBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    pub fn resolve(&self, packet: &UiEventPacket) -> UiAppRouteResolution {
        if !packet.diagnostics.is_empty() {
            return UiAppRouteResolution::rejected(
                UiAppRouteResolutionDiagnostic::RouteDiagnostic {
                    route: packet.route.as_str().to_owned(),
                },
            );
        }
        if !packet.payload.diagnostics.is_empty() {
            return UiAppRouteResolution::rejected(
                UiAppRouteResolutionDiagnostic::PayloadDiagnostic {
                    route: packet.route.as_str().to_owned(),
                },
            );
        }

        let Some(binding) = self
            .bindings
            .iter()
            .find(|candidate| candidate.route == packet.route)
        else {
            return UiAppRouteResolution::rejected(UiAppRouteResolutionDiagnostic::UnknownRoute {
                route: packet.route.as_str().to_owned(),
            });
        };

        if binding.schema_version != packet.schema_version {
            return UiAppRouteResolution::rejected(
                UiAppRouteResolutionDiagnostic::WrongSchemaVersion {
                    route: packet.route.as_str().to_owned(),
                    expected: binding.schema_version.value(),
                    actual: packet.schema_version.value(),
                },
            );
        }

        if binding.payload_schema != packet.payload.schema {
            return UiAppRouteResolution::rejected(
                UiAppRouteResolutionDiagnostic::PayloadSchemaMismatch {
                    route: packet.route.as_str().to_owned(),
                    expected: binding.payload_schema.id.as_str().to_owned(),
                    actual: packet.payload.schema.id.as_str().to_owned(),
                },
            );
        }

        if !packet.requires_capability(&binding.capability) {
            return UiAppRouteResolution::rejected(
                UiAppRouteResolutionDiagnostic::MissingCapability {
                    route: packet.route.as_str().to_owned(),
                    capability: binding.capability.as_str().to_owned(),
                },
            );
        }

        UiAppRouteResolution::resolved(UiAppResolvedAction {
            action_id: binding.action_id.clone(),
            route: binding.route.clone(),
        })
    }
}
