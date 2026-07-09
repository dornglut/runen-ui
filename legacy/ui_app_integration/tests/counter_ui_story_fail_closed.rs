use ui_app_integration::{
    CounterHost, UiAppActionId, UiAppProofDiagnostic, UiAppRouteBinding, UiAppRouteBindingId,
    UiAppRouteBridge, UiAppRouteResolutionDiagnostic, UiAppSourceBuilder, counter_packet,
    counter_payload_schema, counter_payload_validation_schema, counter_route_bridge,
    run_counter_step_for_packet,
};
use ui_program::{
    RouteCapability, RouteId, RouteSchemaVersion, UiEventPacket, UiProgramDiagnostic,
};
use ui_schema::{UiSchemaRef, UiSchemaValue};

#[test]
fn unknown_route_is_rejected_without_mutation() {
    let bridge = counter_route_bridge();
    let host = CounterHost::new(0);
    let before = host.snapshot();
    let resolution = bridge.resolve(&counter_packet(
        "counter.unknown",
        "counter.action.increment",
    ));

    assert!(!resolution.is_resolved());
    assert!(matches!(
        resolution.diagnostics.first(),
        Some(UiAppRouteResolutionDiagnostic::UnknownRoute { .. })
    ));
    assert_eq!(host.snapshot(), before);
}

#[test]
fn packet_diagnostics_are_rejected_without_mutation() {
    let bridge = counter_route_bridge();
    let host = CounterHost::new(0);
    let packet = counter_packet("counter.increment", "counter.action.increment").with_diagnostic(
        UiProgramDiagnostic::new(
            "ui.event.route.disabled",
            "disabled control suppressed activation",
        ),
    );

    let resolution = bridge.resolve(&packet);
    assert!(!resolution.is_resolved());
    assert!(matches!(
        resolution.diagnostics.first(),
        Some(UiAppRouteResolutionDiagnostic::RouteDiagnostic { .. })
    ));
    assert_eq!(host.count(), 0);
}

#[test]
fn payload_diagnostics_are_rejected_without_mutation() {
    let bridge = counter_route_bridge();
    let host = CounterHost::new(0);
    let packet = UiEventPacket::new(
        RouteId::new("counter.increment"),
        RouteSchemaVersion::new(1),
        counter_payload_schema(),
        UiSchemaValue::null(),
    )
    .with_capability(RouteCapability::new("counter.action.increment"))
    .with_payload_validation(&counter_payload_validation_schema());

    assert!(!packet.payload.diagnostics.is_empty());
    let resolution = bridge.resolve(&packet);
    assert!(!resolution.is_resolved());
    assert!(matches!(
        resolution.diagnostics.first(),
        Some(UiAppRouteResolutionDiagnostic::PayloadDiagnostic { .. })
    ));
    assert_eq!(host.count(), 0);
}

#[test]
fn wrong_schema_version_is_rejected_without_mutation() {
    let bridge = counter_route_bridge();
    let host = CounterHost::new(0);
    let packet = UiEventPacket::new(
        RouteId::new("counter.increment"),
        RouteSchemaVersion::new(2),
        counter_payload_schema(),
        UiSchemaValue::null(),
    )
    .with_capability(RouteCapability::new("counter.action.increment"));

    let resolution = bridge.resolve(&packet);
    assert!(!resolution.is_resolved());
    assert!(matches!(
        resolution.diagnostics.first(),
        Some(UiAppRouteResolutionDiagnostic::WrongSchemaVersion { .. })
    ));
    assert_eq!(host.count(), 0);
}

#[test]
fn missing_capability_is_rejected_without_mutation() {
    let bridge = counter_route_bridge();
    let host = CounterHost::new(0);
    let packet = UiEventPacket::new(
        RouteId::new("counter.increment"),
        RouteSchemaVersion::new(1),
        counter_payload_schema(),
        UiSchemaValue::null(),
    );

    let resolution = bridge.resolve(&packet);
    assert!(!resolution.is_resolved());
    assert!(matches!(
        resolution.diagnostics.first(),
        Some(UiAppRouteResolutionDiagnostic::MissingCapability { .. })
    ));
    assert_eq!(host.count(), 0);
}

#[test]
fn invalid_payload_schema_is_rejected_without_mutation() {
    let bridge = counter_route_bridge();
    let host = CounterHost::new(0);
    let packet = UiEventPacket::new(
        RouteId::new("counter.increment"),
        RouteSchemaVersion::new(1),
        UiSchemaRef::new("counter.action.wrong_payload", 1),
        UiSchemaValue::null(),
    )
    .with_capability(RouteCapability::new("counter.action.increment"));

    let resolution = bridge.resolve(&packet);
    assert!(!resolution.is_resolved());
    assert!(matches!(
        resolution.diagnostics.first(),
        Some(UiAppRouteResolutionDiagnostic::PayloadSchemaMismatch { .. })
    ));
    assert_eq!(host.count(), 0);
}

#[test]
fn resolved_action_is_the_only_mutation_path() {
    let bridge = counter_route_bridge();
    let mut host = CounterHost::new(0);
    let bad = bridge.resolve(&counter_packet(
        "counter.unknown",
        "counter.action.increment",
    ));
    assert!(!bad.is_resolved());
    assert_eq!(host.count(), 0);

    let good = bridge.resolve(&counter_packet(
        "counter.increment",
        "counter.action.increment",
    ));
    assert!(good.is_resolved());
    let action = good.action.expect("resolved action is required");
    host.apply_resolved_action(&action)
        .expect("known counter action should mutate");
    assert_eq!(host.count(), 1);
}

#[test]
fn unformed_route_reports_no_action_and_no_mutation() {
    let bridge = counter_route_bridge();
    let step = run_counter_step_for_packet(
        0,
        UiAppSourceBuilder::counter_screen(0),
        &bridge,
        counter_packet("counter.reset", "counter.action.reset"),
    );

    assert!(!step.passed());
    assert!(step.action.is_none());
    assert!(step.mutation.is_none());
    assert_eq!(step.before.count, 0);
    assert_eq!(step.after.count, 0);
    assert!(step
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic, UiAppProofDiagnostic::RouteMissing { route } if route == "counter.reset")));
}

#[test]
fn missing_host_action_data_reports_diagnostic_without_mutation() {
    let bridge = UiAppRouteBridge::new().with_binding(UiAppRouteBinding::new(
        UiAppRouteBindingId::new("counter.missing.binding"),
        RouteId::new("counter.increment"),
        RouteSchemaVersion::new(1),
        counter_payload_schema(),
        UiAppActionId::new("counter.missing"),
        RouteCapability::new("counter.action.increment"),
    ));

    let step = run_counter_step_for_packet(
        0,
        UiAppSourceBuilder::counter_screen(0),
        &bridge,
        counter_packet("counter.increment", "counter.action.increment"),
    );

    assert!(!step.passed());
    assert!(step.action.as_ref().is_some_and(|action| action.resolved));
    assert!(step.mutation.is_none());
    assert_eq!(step.before.count, 0);
    assert_eq!(step.after.count, 0);
    assert!(step.diagnostics.iter().any(|diagnostic| {
        matches!(diagnostic, UiAppProofDiagnostic::MutationMissing { action } if action == "counter.missing")
    }));
}
