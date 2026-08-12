#![allow(refining_impl_trait)]

#[path = "../src/app.rs"]
mod app;
#[path = "../src/ui.rs"]
mod ui;

use app::{Counter, CounterApp};
use runenui_core::{
    CommandOrigin, ElementId, EventSource, KeyLocation, KeyModifiers, KeyboardCompositionState,
    KeyboardEvent, KeyboardPhase, LogicalDelta, LogicalKey, LogicalPoint, PhysicalKey,
    PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    SemanticCommand, StyleTokens,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, MountedNodeId, PumpBudget, RuntimeConfig, SurfaceBuildContext,
    TraceConfig, TraceRecordKind, TraceReplay,
};

fn surface_size() -> LogicalSize {
    LogicalSize::try_new(240.0, 160.0)
        .unwrap_or_else(|_| unreachable!("counter proof surface size is finite"))
}

fn authored_id(value: &str) -> ElementId {
    ElementId::new(value).unwrap_or_else(|_| unreachable!("counter authored id is valid"))
}

fn settle(runtime: &mut AppRuntime<CounterApp>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent(), "counter did not settle: {report:?}");
}

fn mounted_counter() -> AppRuntime<CounterApp> {
    let mut runtime = AppRuntime::<CounterApp>::mount_with_config(
        Counter::new(),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(4096)),
    );
    settle(&mut runtime);
    runtime
}

fn increment_target(runtime: &mut AppRuntime<CounterApp>) -> MountedNodeId {
    let index = runtime.index();
    let matches = index
        .nodes()
        .iter()
        .filter(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "counter.increment")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        matches.len(),
        1,
        "counter.increment must resolve to one exact mounted node"
    );
    matches[0].id().clone()
}

fn assert_increment_identity(runtime: &mut AppRuntime<CounterApp>, expected: &MountedNodeId) {
    let index = runtime.index();
    let node = index
        .node(expected)
        .unwrap_or_else(|| unreachable!("increment mounted identity remains live"));
    assert_eq!(
        node.authored_id().map(ElementId::as_str),
        Some("counter.increment")
    );
}

fn assert_activation_source(
    runtime: &AppRuntime<CounterApp>,
    trace_start: usize,
    expected: EventSource,
) {
    let matches = runtime
        .trace()
        .records()
        .skip(trace_start)
        .filter(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::SemanticDefaultApplied {
                    command: SemanticCommand::Activate
                }
            ) && record
                .command_origin()
                .is_some_and(|origin| origin.source() == expected)
        })
        .count();
    assert_eq!(
        matches, 1,
        "one canonical activation default must be attributed to {expected:?}"
    );
}

fn activate_command(
    runtime: &mut AppRuntime<CounterApp>,
    target: &MountedNodeId,
    origin: CommandOrigin,
    expected_count: i32,
    expected_source: EventSource,
) {
    let trace_start = runtime.trace().len();
    runtime
        .submit_command(target.clone(), SemanticCommand::Activate, origin)
        .unwrap_or_else(|_| unreachable!("exact semantic activation is accepted"));
    settle(runtime);
    assert_eq!(runtime.state().count, expected_count);
    assert_activation_source(runtime, trace_start, expected_source);
    assert_increment_identity(runtime, target);
}

fn primary_pointer_event(
    pointer_id: PointerId,
    phase: PointerPhase,
    point: LogicalPoint,
    context: runenui_core::SurfaceInputContext,
) -> PointerEvent {
    PointerEvent::new(pointer_id, PointerDeviceKind::Mouse, phase, point, context)
        .with_buttons(if phase == PointerPhase::Down {
            PointerButtons::new([PointerButton::Primary])
        } else {
            PointerButtons::default()
        })
        .with_changed_button(PointerButton::Primary)
        .with_movement_delta(LogicalDelta::ZERO)
}

const fn enter_down() -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Enter,
        LogicalKey::Enter,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

#[test]
fn m4_close_01_counter_converges_all_canonical_activation_origins() {
    let mut runtime = mounted_counter();
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::tight(&tokens, surface_size());
    let publication = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("M4 closure publication is admitted"));
    let increment = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "counter.increment")
        })
        .unwrap_or_else(|| unreachable!("increment control is published"));
    let bounds = increment.bounds();
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published coordinates are finite"));
    let input = publication.input_context().clone();
    let pointer_id =
        PointerId::new(1).unwrap_or_else(|| unreachable!("pointer identity is non-zero"));

    let trace_start = runtime.trace().len();
    runtime
        .submit_pointer(primary_pointer_event(
            pointer_id,
            PointerPhase::Down,
            point,
            input.clone(),
        ))
        .unwrap_or_else(|_| unreachable!("displayed pointer down is accepted"));
    runtime
        .submit_pointer(primary_pointer_event(
            pointer_id,
            PointerPhase::Up,
            point,
            input,
        ))
        .unwrap_or_else(|_| unreachable!("displayed pointer up is accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().count, 1);
    assert_activation_source(&runtime, trace_start, EventSource::Pointer);

    let mounted_increment = increment_target(&mut runtime);
    runtime
        .submit_command(
            mounted_increment.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("exact increment focus request is accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&mounted_increment));

    let trace_start = runtime.trace().len();
    runtime
        .submit_keyboard(enter_down())
        .unwrap_or_else(|_| unreachable!("raw Enter is accepted for the focused increment"));
    settle(&mut runtime);
    assert_eq!(runtime.state().count, 2);
    assert_activation_source(&runtime, trace_start, EventSource::Keyboard);
    assert_increment_identity(&mut runtime, &mounted_increment);

    activate_command(
        &mut runtime,
        &mounted_increment,
        CommandOrigin::controller(),
        3,
        EventSource::Controller,
    );
    activate_command(
        &mut runtime,
        &mounted_increment,
        CommandOrigin::accessibility(),
        4,
        EventSource::Accessibility,
    );

    let trace_start = runtime.trace().len();
    runtime
        .submit_automation_command(authored_id("counter.increment"), SemanticCommand::Activate)
        .unwrap_or_else(|_| unreachable!("authored-ID automation activation is accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().count, 5);
    assert_activation_source(&runtime, trace_start, EventSource::Automation);
    assert_increment_identity(&mut runtime, &mounted_increment);

    activate_command(
        &mut runtime,
        &mounted_increment,
        CommandOrigin::programmatic(),
        6,
        EventSource::Programmatic,
    );

    let jsonl = runtime.trace().export_jsonl();
    let replay = TraceReplay::parse_jsonl(&jsonl)
        .unwrap_or_else(|error| unreachable!("complete Counter trace must replay: {error}"));
    assert!(replay.is_complete());
    assert_eq!(
        replay
            .records()
            .filter(|record| record.kind().as_str() == "application_state_updated")
            .count(),
        6,
        "all six canonical activation sources must converge through application update"
    );
}
