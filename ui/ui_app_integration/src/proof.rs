//! Proof orchestration for the ECS-backed counter UI integration proof.

use ui_controls::{ControlPackageRegistry, runenwerk_control_package};
use ui_program::{
    RouteCapability, RouteId, RouteSchemaVersion, UiEventPacket, UiEventSourceControlId,
};
use ui_program_lowering::UiProgramFormationReport;
use ui_program_lowering::form_ui_program_report_from_node_with_registry_snapshot;
use ui_schema::{UiSchema, UiSchemaRef, UiSchemaShape, UiSchemaValue};

use crate::bridge::{UiAppRouteBinding, UiAppRouteBridge};
use crate::host::CounterHost;
use crate::ids::{UiAppActionId, UiAppProofId, UiAppRouteBindingId};
use crate::report::{
    UiAppActionReport, UiAppFormationReportSummary, UiAppIntegrationReport,
    UiAppIntegrationStepReport, UiAppProofDiagnostic, UiAppRuntimeReportSummary, UiAppSourceReport,
};
use crate::source::{UiAppSourceBuildReport, UiAppSourceBuilder};

#[derive(Clone, Debug)]
pub struct UiAppIntegrationProof {
    proof_id: UiAppProofId,
}

impl UiAppIntegrationProof {
    pub fn builder(proof_id: UiAppProofId) -> UiAppIntegrationProofBuilder {
        UiAppIntegrationProofBuilder { proof_id }
    }
}

#[derive(Clone, Debug)]
pub struct UiAppIntegrationProofBuilder {
    proof_id: UiAppProofId,
}

impl UiAppIntegrationProofBuilder {
    pub fn build(self) -> UiAppIntegrationProof {
        UiAppIntegrationProof {
            proof_id: self.proof_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiAppIntegrationProofRun {
    pub report: UiAppIntegrationReport,
}

impl UiAppIntegrationProof {
    pub fn run_counter_story(&self) -> UiAppIntegrationProofRun {
        let mut host = CounterHost::new(0);
        let initial = host.snapshot();
        let mut steps = Vec::new();
        let mut diagnostics = Vec::new();
        let bridge = counter_route_bridge();

        for index in 0..5 {
            let count = host.count();
            let source = UiAppSourceBuilder::counter_screen(count);
            let step = run_step_from_formed_route(
                format!("increment.{index}"),
                &mut host,
                source,
                &bridge,
                RouteId::new("counter.increment"),
                RouteCapability::new("counter.action.increment"),
            );
            if !step.passed() {
                diagnostics.extend(step.diagnostics.clone());
            }
            steps.push(step);
        }

        let reset = run_step_from_formed_route(
            "reset".to_string(),
            &mut host,
            UiAppSourceBuilder::win_screen(),
            &bridge,
            RouteId::new("counter.reset"),
            RouteCapability::new("counter.action.reset"),
        );
        if !reset.passed() {
            diagnostics.extend(reset.diagnostics.clone());
        }
        steps.push(reset);

        let report = UiAppIntegrationReport {
            proof_id: self.proof_id.clone(),
            initial,
            final_snapshot: host.snapshot(),
            steps,
            diagnostics,
        };

        UiAppIntegrationProofRun { report }
    }
}

pub fn run_counter_step_for_packet(
    count: u32,
    source: UiAppSourceBuildReport,
    bridge: &UiAppRouteBridge,
    packet: UiEventPacket,
) -> UiAppIntegrationStepReport {
    let mut host = CounterHost::new(count);
    run_step_with_packet("single".to_owned(), &mut host, source, bridge, packet)
}

fn run_step_from_formed_route(
    step: String,
    host: &mut CounterHost,
    source: UiAppSourceBuildReport,
    bridge: &UiAppRouteBridge,
    route: RouteId,
    app_capability: RouteCapability,
) -> UiAppIntegrationStepReport {
    let before = host.snapshot();
    let formation_report = form_source(&source);

    let mut diagnostics = Vec::new();
    if !formation_report.passed() {
        diagnostics.push(UiAppProofDiagnostic::FormationFailed {
            screen: source.screen_id.as_str().to_owned(),
            diagnostic_count: formation_report.diagnostics.len(),
        });
    }

    let packet = if formation_report.passed() {
        match packet_from_formed_route(&formation_report, &route, app_capability) {
            Ok(packet) => Some(packet),
            Err(diagnostic) => {
                diagnostics.push(diagnostic);
                None
            }
        }
    } else {
        None
    };

    finish_step(
        step,
        before,
        host,
        source,
        formation_report,
        bridge,
        packet,
        diagnostics,
    )
}

fn run_step_with_packet(
    step: String,
    host: &mut CounterHost,
    source: UiAppSourceBuildReport,
    bridge: &UiAppRouteBridge,
    packet: UiEventPacket,
) -> UiAppIntegrationStepReport {
    let before = host.snapshot();
    let formation_report = form_source(&source);

    let mut diagnostics = Vec::new();
    if !formation_report.passed() {
        diagnostics.push(UiAppProofDiagnostic::FormationFailed {
            screen: source.screen_id.as_str().to_owned(),
            diagnostic_count: formation_report.diagnostics.len(),
        });
    }

    let formed = formation_report.passed();
    finish_step(
        step,
        before,
        host,
        source,
        formation_report,
        bridge,
        formed.then_some(packet),
        diagnostics,
    )
}

fn finish_step(
    step: String,
    before: crate::host::UiAppHostSnapshot,
    host: &mut CounterHost,
    source: UiAppSourceBuildReport,
    formation_report: UiProgramFormationReport,
    bridge: &UiAppRouteBridge,
    packet: Option<UiEventPacket>,
    mut diagnostics: Vec<UiAppProofDiagnostic>,
) -> UiAppIntegrationStepReport {
    let mut action = None;
    let mut mutation = None;

    if let Some(packet) = packet {
        if !formed_route_ids(&formation_report)
            .iter()
            .any(|route| route == packet.route.as_str())
        {
            diagnostics.push(UiAppProofDiagnostic::RouteMissing {
                route: packet.route.as_str().to_owned(),
            });
        } else {
            let resolution = bridge.resolve(&packet);
            action = resolution.action.clone().map(|resolved| UiAppActionReport {
                action_id: resolved.action_id.clone(),
                route: resolved.route.as_str().to_owned(),
                resolved: true,
            });

            for diagnostic in &resolution.diagnostics {
                diagnostics.push(UiAppProofDiagnostic::RouteRejected {
                    diagnostic: diagnostic.clone(),
                });
            }

            mutation = resolution
                .action
                .as_ref()
                .and_then(|resolved| host.apply_resolved_action(resolved));

            if resolution.is_resolved() && mutation.is_none() {
                if let Some(action) = action.as_ref() {
                    diagnostics.push(UiAppProofDiagnostic::MutationMissing {
                        action: action.action_id.as_str().to_owned(),
                    });
                }
            }
        }
    }

    let output_text = source.output_text_facts();
    if output_text.is_empty() {
        diagnostics.push(UiAppProofDiagnostic::NextOutputMissing {
            screen: source.screen_id.as_str().to_owned(),
        });
    }

    let after = host.snapshot();
    UiAppIntegrationStepReport {
        step,
        before,
        source: UiAppSourceReport {
            source: source.clone(),
        },
        formation: UiAppFormationReportSummary {
            screen_id: source.screen_id.clone(),
            passed: formation_report.passed(),
            diagnostics: formation_report.diagnostics.len(),
            source_map_entries: formation_report.program.source_map.len(),
        },
        runtime: UiAppRuntimeReportSummary {
            screen_id: source.screen_id.clone(),
            output_contains_text: output_text,
            route_ids: formed_route_ids(&formation_report),
        },
        action,
        mutation,
        after,
        diagnostics,
    }
}

fn form_source(source: &UiAppSourceBuildReport) -> UiProgramFormationReport {
    let registry = ControlPackageRegistry::new()
        .with_package(runenwerk_control_package())
        .expect("runenwerk control package must register for proof");
    let snapshot = registry.snapshot();
    form_ui_program_report_from_node_with_registry_snapshot(
        format!("{}.program", source.screen_id.as_str()),
        format!("{}.source", source.screen_id.as_str()),
        &source.root,
        &snapshot,
    )
}

fn formed_route_ids(formation_report: &UiProgramFormationReport) -> Vec<String> {
    formation_report
        .program
        .graphs
        .interaction
        .handlers
        .iter()
        .map(|handler| handler.route.as_str().to_owned())
        .collect()
}

fn packet_from_formed_route(
    formation_report: &UiProgramFormationReport,
    route: &RouteId,
    app_capability: RouteCapability,
) -> Result<UiEventPacket, UiAppProofDiagnostic> {
    let Some(handler) = formation_report
        .program
        .graphs
        .interaction
        .handlers
        .iter()
        .find(|handler| &handler.route == route)
    else {
        return Err(UiAppProofDiagnostic::RouteMissing {
            route: route.as_str().to_owned(),
        });
    };

    let mut packet = UiEventPacket::new(
        handler.route.clone(),
        RouteSchemaVersion::new(1),
        handler.payload_schema.clone(),
        counter_payload_value(route.as_str()),
    )
    .with_source_control(UiEventSourceControlId::new(handler.control_id.as_str()))
    .with_capability(app_capability);

    for capability in &handler.required_capabilities {
        packet = packet.with_capability(capability.clone());
    }
    if let Some(source_map) = handler.source_map.as_ref() {
        packet = packet.with_source_map_entry(source_map.entry.clone());
    }

    Ok(packet)
}

pub fn counter_payload_schema() -> UiSchemaRef {
    UiSchemaRef::new("runenwerk.ui.controls.button.event", 1)
}

pub fn counter_payload_validation_schema() -> UiSchema {
    UiSchema::object("runenwerk.ui.controls.button.event", 1)
        .with_required_field("route", UiSchemaShape::RouteRef)
        .with_required_field("activated", UiSchemaShape::Bool)
}

fn counter_payload_value(route: &str) -> UiSchemaValue {
    UiSchemaValue::object([
        ("route", UiSchemaValue::route_ref(route)),
        ("activated", UiSchemaValue::bool(true)),
    ])
}

pub fn counter_packet(route: &str, capability: &str) -> UiEventPacket {
    UiEventPacket::new(
        RouteId::new(route),
        RouteSchemaVersion::new(1),
        counter_payload_schema(),
        counter_payload_value(route),
    )
    .with_capability(RouteCapability::new(capability))
}

pub fn counter_route_bridge() -> UiAppRouteBridge {
    UiAppRouteBridge::new()
        .with_binding(UiAppRouteBinding::new(
            UiAppRouteBindingId::new("counter.increment.binding"),
            RouteId::new("counter.increment"),
            RouteSchemaVersion::new(1),
            counter_payload_schema(),
            UiAppActionId::new("counter.increment"),
            RouteCapability::new("counter.action.increment"),
        ))
        .with_binding(UiAppRouteBinding::new(
            UiAppRouteBindingId::new("counter.reset.binding"),
            RouteId::new("counter.reset"),
            RouteSchemaVersion::new(1),
            counter_payload_schema(),
            UiAppActionId::new("counter.reset"),
            RouteCapability::new("counter.action.reset"),
        ))
}
