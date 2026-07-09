//! File: domain/ui/ui_hosts/src/lib.rs
//! Crate: ui_hosts

use std::fmt;

use serde::{Deserialize, Serialize};
use ui_evaluator::UiOutput;
use ui_program::{RouteCapability, RouteId, RouteSchemaVersion, UiEventPacket};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HostRouteMapVersion(u32);

impl HostRouteMapVersion {
    pub const fn new(value: u32) -> Self {
        assert!(value > 0);
        Self(value)
    }

    pub fn try_new(value: u32) -> Result<Self, HostContractError> {
        if value == 0 {
            Err(HostContractError::ZeroRouteMapVersion)
        } else {
            Ok(Self(value))
        }
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostKind {
    Editor,
    Game,
    WorldSpace,
    Headless,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostCommand {
    pub host: HostKind,
    pub command_id: String,
}

impl HostCommand {
    pub fn new(host: HostKind, command_id: impl Into<String>) -> Self {
        Self {
            host,
            command_id: command_id.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainCommand {
    pub domain_id: String,
    pub command_id: String,
}

impl DomainCommand {
    pub fn new(domain_id: impl Into<String>, command_id: impl Into<String>) -> Self {
        Self {
            domain_id: domain_id.into(),
            command_id: command_id.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRouteMapping {
    pub route_id: RouteId,
    pub schema_version: RouteSchemaVersion,
    pub route_map_version: HostRouteMapVersion,
    #[serde(default)]
    pub required_capabilities: Vec<RouteCapability>,
    pub host_command: HostCommand,
    #[serde(default)]
    pub domain_command: Option<DomainCommand>,
}

impl HostRouteMapping {
    pub fn new(
        route_id: RouteId,
        schema_version: RouteSchemaVersion,
        route_map_version: HostRouteMapVersion,
        host_command: HostCommand,
    ) -> Self {
        Self {
            route_id,
            schema_version,
            route_map_version,
            required_capabilities: Vec::new(),
            host_command,
            domain_command: None,
        }
    }

    pub fn with_capability(mut self, capability: RouteCapability) -> Self {
        self.required_capabilities.push(capability);
        self
    }

    pub fn with_domain_command(mut self, domain_command: DomainCommand) -> Self {
        self.domain_command = Some(domain_command);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRouteMap {
    pub host: HostKind,
    pub version: HostRouteMapVersion,
    pub mappings: Vec<HostRouteMapping>,
}

impl HostRouteMap {
    pub fn new(host: HostKind, version: HostRouteMapVersion) -> Self {
        Self {
            host,
            version,
            mappings: Vec::new(),
        }
    }

    pub fn with_mapping(mut self, mapping: HostRouteMapping) -> Self {
        assert_eq!(mapping.host_command.host, self.host);
        assert_eq!(mapping.route_map_version, self.version);
        self.mappings.push(mapping);
        self
    }

    pub fn resolve_event(&self, packet: &UiEventPacket) -> HostRouteResolution {
        let Some(mapping) = self
            .mappings
            .iter()
            .find(|mapping| mapping.route_id == packet.route)
        else {
            return HostRouteResolution::rejected(
                self.host,
                HostRouteResolutionStatus::MissingRoute,
                HostDiagnostic::missing_route(&packet.route),
            );
        };

        if mapping.schema_version != packet.schema_version {
            return HostRouteResolution::rejected(
                self.host,
                HostRouteResolutionStatus::UnsupportedSchemaVersion,
                HostDiagnostic {
                    code: "ui.host.route.unsupported_schema_version".to_owned(),
                    message: format!(
                        "route {} schema version {} is not accepted by host map version {}",
                        packet.route.as_str(),
                        packet.schema_version.value(),
                        self.version.value()
                    ),
                },
            );
        }

        if let Some(missing) = mapping
            .required_capabilities
            .iter()
            .find(|capability| !packet.requires_capability(capability))
        {
            return HostRouteResolution::rejected(
                self.host,
                HostRouteResolutionStatus::MissingCapability,
                HostDiagnostic {
                    code: "ui.host.route.missing_capability".to_owned(),
                    message: format!(
                        "route {} is missing capability {}",
                        packet.route.as_str(),
                        missing.as_str()
                    ),
                },
            );
        }

        HostRouteResolution {
            host: self.host,
            status: HostRouteResolutionStatus::Mapped,
            mapping: Some(mapping.clone()),
            diagnostics: Vec::new(),
        }
    }
}

pub trait HostBoundaryContract {
    const KIND: HostKind;

    fn route_map_version(&self) -> HostRouteMapVersion;
    fn required_capabilities(&self) -> &[RouteCapability];

    fn accepts_route_map(&self, route_map: &HostRouteMap) -> bool {
        route_map.host == Self::KIND && route_map.version == self.route_map_version()
    }
}

macro_rules! define_host_contract {
    ($type_name:ident, $kind:expr) => {
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $type_name {
            pub route_map_version: HostRouteMapVersion,
            #[serde(default)]
            pub required_capabilities: Vec<RouteCapability>,
        }

        impl $type_name {
            pub fn new(route_map_version: HostRouteMapVersion) -> Self {
                Self {
                    route_map_version,
                    required_capabilities: Vec::new(),
                }
            }

            pub fn with_capability(mut self, capability: RouteCapability) -> Self {
                self.required_capabilities.push(capability);
                self
            }
        }

        impl HostBoundaryContract for $type_name {
            const KIND: HostKind = $kind;

            fn route_map_version(&self) -> HostRouteMapVersion {
                self.route_map_version
            }

            fn required_capabilities(&self) -> &[RouteCapability] {
                &self.required_capabilities
            }
        }
    };
}

define_host_contract!(EditorHostContract, HostKind::Editor);
define_host_contract!(GameHostContract, HostKind::Game);
define_host_contract!(WorldSpaceHostContract, HostKind::WorldSpace);
define_host_contract!(HeadlessHostContract, HostKind::Headless);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRouteResolution {
    pub host: HostKind,
    pub status: HostRouteResolutionStatus,
    #[serde(default)]
    pub mapping: Option<HostRouteMapping>,
    #[serde(default)]
    pub diagnostics: Vec<HostDiagnostic>,
}

impl HostRouteResolution {
    pub fn is_mapped(&self) -> bool {
        self.status == HostRouteResolutionStatus::Mapped
    }

    fn rejected(
        host: HostKind,
        status: HostRouteResolutionStatus,
        diagnostic: HostDiagnostic,
    ) -> Self {
        Self {
            host,
            status,
            mapping: None,
            diagnostics: vec![diagnostic],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostRouteResolutionStatus {
    Mapped,
    MissingRoute,
    UnsupportedSchemaVersion,
    MissingCapability,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostDiagnostic {
    pub code: String,
    pub message: String,
}

impl HostDiagnostic {
    fn missing_route(route: &RouteId) -> Self {
        Self {
            code: "ui.host.route.missing".to_owned(),
            message: format!(
                "route {} is not present in the host route map",
                route.as_str()
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HostSurfaceFacts {
    pub surface_id: String,
    pub visible: bool,
    pub scale_factor: f32,
    #[serde(default)]
    pub world_space: Option<WorldSpaceHostFacts>,
}

impl HostSurfaceFacts {
    pub fn headless(surface_id: impl Into<String>) -> Self {
        Self {
            surface_id: surface_id.into(),
            visible: true,
            scale_factor: 1.0,
            world_space: None,
        }
    }

    pub fn with_world_space(mut self, facts: WorldSpaceHostFacts) -> Self {
        self.world_space = Some(facts);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSpaceHostFacts {
    pub anchor_id: String,
    pub alive: bool,
    pub visible: bool,
    pub data_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HostOutputReceipt {
    pub host: HostKind,
    pub route_map_version: HostRouteMapVersion,
    pub surface_facts: HostSurfaceFacts,
    pub visual_operator_count: usize,
    pub diagnostic_count: usize,
    pub source_mapped_rows: usize,
}

pub trait UiHost {
    fn kind(&self) -> HostKind;
    fn route_map_version(&self) -> HostRouteMapVersion;
    fn resolve_event(&self, packet: &UiEventPacket) -> HostRouteResolution;

    fn map_event(&self, packet: &UiEventPacket) -> Option<HostRouteMapping> {
        self.resolve_event(packet).mapping
    }

    fn consume_output(
        &self,
        output: &UiOutput,
        surface_facts: HostSurfaceFacts,
    ) -> HostOutputReceipt {
        HostOutputReceipt {
            host: self.kind(),
            route_map_version: self.route_map_version(),
            surface_facts,
            visual_operator_count: output.visual.operators.len(),
            diagnostic_count: output.diagnostics.len(),
            source_mapped_rows: source_mapped_rows(output),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorHost {
    pub route_map: HostRouteMap,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameHost {
    pub route_map: HostRouteMap,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSpaceHost {
    pub route_map: HostRouteMap,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessHost {
    pub route_map: HostRouteMap,
}

macro_rules! impl_host {
    ($type_name:ident, $kind:expr) => {
        impl $type_name {
            pub fn new(version: HostRouteMapVersion) -> Self {
                Self {
                    route_map: HostRouteMap::new($kind, version),
                }
            }

            pub fn with_mapping(mut self, mapping: HostRouteMapping) -> Self {
                self.route_map = self.route_map.with_mapping(mapping);
                self
            }
        }

        impl UiHost for $type_name {
            fn kind(&self) -> HostKind {
                $kind
            }

            fn route_map_version(&self) -> HostRouteMapVersion {
                self.route_map.version
            }

            fn resolve_event(&self, packet: &UiEventPacket) -> HostRouteResolution {
                self.route_map.resolve_event(packet)
            }
        }
    };
}

impl_host!(EditorHost, HostKind::Editor);
impl_host!(GameHost, HostKind::Game);
impl_host!(WorldSpaceHost, HostKind::WorldSpace);
impl_host!(HeadlessHost, HostKind::Headless);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostContractError {
    ZeroRouteMapVersion,
}

impl fmt::Display for HostContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroRouteMapVersion => {
                write!(formatter, "host route map version must be non-zero")
            }
        }
    }
}

impl std::error::Error for HostContractError {}

fn source_mapped_rows(output: &UiOutput) -> usize {
    output
        .controls
        .rows
        .iter()
        .filter(|row| row.source_map_index.is_some())
        .count()
        + output
            .layout
            .rows
            .iter()
            .filter(|row| row.source_map_index.is_some())
            .count()
        + output
            .style
            .rows
            .iter()
            .filter(|row| row.source_map_index.is_some())
            .count()
        + output
            .state
            .rows
            .iter()
            .filter(|row| row.source_map_index.is_some())
            .count()
        + output
            .binding
            .table_rows
            .iter()
            .filter(|row| row.source_map_index.is_some())
            .count()
        + output
            .interaction
            .rows
            .iter()
            .filter(|row| row.source_map_index.is_some())
            .count()
        + output
            .visual
            .operators
            .iter()
            .filter(|row| row.source_map_index.is_some())
            .count()
        + output
            .accessibility
            .rows
            .iter()
            .filter(|row| row.source_map_index.is_some())
            .count()
        + output
            .inspection
            .rows
            .iter()
            .filter(|row| row.source_map_index.is_some())
            .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_program::{RouteId, RouteSchemaVersion, UiEventPacket};
    use ui_schema::{UiSchemaRef, UiSchemaValue};

    #[test]
    fn host_contract_maps_events_and_consumes_outputs_without_product_authority() {
        let version = HostRouteMapVersion::new(2);
        let editor = EditorHost::new(version).with_mapping(
            HostRouteMapping::new(
                RouteId::new("editor.panel.open"),
                RouteSchemaVersion::new(1),
                version,
                HostCommand::new(HostKind::Editor, "editor.command.open_panel"),
            )
            .with_capability(RouteCapability::new("editor.panel.open"))
            .with_domain_command(DomainCommand::new(
                "domain.editor",
                "domain.editor.open_panel",
            )),
        );
        let packet = UiEventPacket::new(
            RouteId::new("editor.panel.open"),
            RouteSchemaVersion::new(1),
            UiSchemaRef::new("ui.panel.open", 1),
            UiSchemaValue::object([("panel", UiSchemaValue::string("inspector"))]),
        )
        .with_capability(RouteCapability::new("editor.panel.open"));

        let mapped = editor.resolve_event(&packet);
        let missing_capability = editor.resolve_event(&UiEventPacket::new(
            RouteId::new("editor.panel.open"),
            RouteSchemaVersion::new(1),
            UiSchemaRef::new("ui.panel.open", 1),
            UiSchemaValue::object([("panel", UiSchemaValue::string("inspector"))]),
        ));
        let receipt = editor.consume_output(
            &UiOutput::default(),
            HostSurfaceFacts::headless("surface.editor.inspector"),
        );

        assert!(mapped.is_mapped());
        assert_eq!(
            mapped
                .mapping
                .as_ref()
                .map(|mapping| mapping.host_command.command_id.as_str()),
            Some("editor.command.open_panel")
        );
        assert_eq!(
            mapped
                .mapping
                .as_ref()
                .and_then(|mapping| mapping.domain_command.as_ref())
                .map(|command| command.command_id.as_str()),
            Some("domain.editor.open_panel")
        );
        assert_eq!(
            missing_capability.status,
            HostRouteResolutionStatus::MissingCapability
        );
        assert_eq!(receipt.host, HostKind::Editor);
        assert_eq!(receipt.route_map_version.value(), 2);
        assert_eq!(receipt.visual_operator_count, 0);

        assert_eq!(GameHost::new(version).kind(), HostKind::Game);
        assert_eq!(WorldSpaceHost::new(version).kind(), HostKind::WorldSpace);
        assert_eq!(HeadlessHost::new(version).kind(), HostKind::Headless);
    }

    #[test]
    fn host_contract_boundary_contracts_match_route_maps() {
        let version = HostRouteMapVersion::new(3);
        let editor = EditorHostContract::new(version)
            .with_capability(RouteCapability::new("editor.inspector.write"));
        let game = GameHostContract::new(version);
        let world_space = WorldSpaceHostContract::new(version);
        let headless = HeadlessHostContract::new(version);

        assert!(editor.accepts_route_map(&HostRouteMap::new(HostKind::Editor, version)));
        assert!(game.accepts_route_map(&HostRouteMap::new(HostKind::Game, version)));
        assert!(world_space.accepts_route_map(&HostRouteMap::new(HostKind::WorldSpace, version)));
        assert!(headless.accepts_route_map(&HostRouteMap::new(HostKind::Headless, version)));
        assert!(!editor.accepts_route_map(&HostRouteMap::new(HostKind::Game, version)));
        assert_eq!(editor.required_capabilities().len(), 1);
    }
}
